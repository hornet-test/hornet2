//! k6 script converter for Arazzo workflows
//!
//! This module generates k6 JavaScript test scripts from Arazzo workflows.

use crate::error::{HornetError, Result};
use crate::models::arazzo::{ArazzoSpec, RequestBody, Step, SuccessCriteria, Workflow};

use super::{ConvertOptions, Converter};
use oas3::OpenApiV3Spec;
use regex::Regex;
use std::collections::HashMap;

/// Converter for generating k6 test scripts
#[derive(Debug, Clone, Default)]
pub struct K6Converter;

impl K6Converter {
    /// Create a new K6Converter
    pub fn new() -> Self {
        Self
    }

    /// Get the base URL from OpenAPI spec
    fn get_base_url(openapi: &OpenApiV3Spec) -> String {
        openapi
            .servers
            .first()
            .map(|server| server.url.clone())
            .unwrap_or_else(|| "http://localhost:8080".to_string())
    }

    /// Find operation info (path and method) by operationId
    fn find_operation(openapi: &OpenApiV3Spec, operation_id: &str) -> Option<(String, String)> {
        if let Some(paths) = &openapi.paths {
            for (path, path_item) in paths.iter() {
                let operations = [
                    ("GET", &path_item.get),
                    ("POST", &path_item.post),
                    ("PUT", &path_item.put),
                    ("DELETE", &path_item.delete),
                    ("PATCH", &path_item.patch),
                    ("HEAD", &path_item.head),
                    ("OPTIONS", &path_item.options),
                ];

                for (method, op_opt) in operations.iter() {
                    if let Some(op) = op_opt {
                        if let Some(ref op_id) = op.operation_id {
                            if op_id == operation_id {
                                return Some((path.clone(), method.to_string()));
                            }
                        }
                    }
                }
            }
        }
        None
    }

    /// Convert a JSON value to JavaScript code
    fn json_to_js(value: &serde_json::Value, indent: usize) -> String {
        let indent_str = "  ".repeat(indent);
        let inner_indent = "  ".repeat(indent + 1);

        match value {
            serde_json::Value::Null => "null".to_string(),
            serde_json::Value::Bool(b) => b.to_string(),
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::String(s) => {
                // Check if it's a runtime expression that needs conversion
                if s.starts_with('$') {
                    Self::convert_runtime_expr(s)
                } else {
                    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
                }
            }
            serde_json::Value::Array(arr) => {
                if arr.is_empty() {
                    "[]".to_string()
                } else {
                    let items: Vec<String> = arr
                        .iter()
                        .map(|v| format!("{}{}", inner_indent, Self::json_to_js(v, indent + 1)))
                        .collect();
                    format!("[\n{}\n{}]", items.join(",\n"), indent_str)
                }
            }
            serde_json::Value::Object(obj) => {
                if obj.is_empty() {
                    "{}".to_string()
                } else {
                    let items: Vec<String> = obj
                        .iter()
                        .map(|(k, v)| {
                            format!(
                                "{}\"{}\": {}",
                                inner_indent,
                                k,
                                Self::json_to_js(v, indent + 1)
                            )
                        })
                        .collect();
                    format!("{{\n{}\n{}}}", items.join(",\n"), indent_str)
                }
            }
        }
    }

    /// Convert Arazzo runtime expression to JavaScript variable reference
    fn convert_runtime_expr(expr: &str) -> String {
        // Handle various Arazzo runtime expressions:
        // $inputs.field -> inputs.field (need to use template literal or variable)
        // $steps.stepId.outputs.field -> stepId_field
        // $response.body.field -> response.json('field')
        // $statusCode -> response.status

        if expr.starts_with("$inputs.") {
            let field = expr.strip_prefix("$inputs.").unwrap();
            format!("inputs.{}", field)
        } else if expr.starts_with("$steps.") {
            // e.g., $steps.login.outputs.token -> login_outputs.token
            let rest = expr.strip_prefix("$steps.").unwrap();
            let parts: Vec<&str> = rest.splitn(3, '.').collect();
            if parts.len() >= 3 && parts[1] == "outputs" {
                format!("{}_{}", parts[0], parts[2])
            } else {
                // Just return as-is with underscore replacement
                rest.replace('.', "_")
            }
        } else if expr.starts_with("$response.body.") {
            let field = expr.strip_prefix("$response.body.").unwrap();
            format!("response.json('{}')", field)
        } else if expr == "$statusCode" {
            "response.status".to_string()
        } else if expr.starts_with("$response.header.") {
            let header = expr.strip_prefix("$response.header.").unwrap();
            format!("response.headers['{}']", header)
        } else {
            // Unknown expression, return as string
            format!("\"{}\"", expr)
        }
    }

    /// Check if string contains runtime expressions
    fn contains_runtime_expr(s: &str) -> bool {
        s.contains("$inputs.")
            || s.contains("$steps.")
            || s.contains("$response.")
            || s.contains("$statusCode")
    }

    /// Convert a string value that may contain embedded runtime expressions
    fn convert_value_with_expr(value: &serde_json::Value) -> String {
        match value {
            serde_json::Value::String(s) => {
                if s.starts_with('$') && !s.contains(' ') {
                    // Pure runtime expression
                    Self::convert_runtime_expr(s)
                } else if Self::contains_runtime_expr(s) {
                    // String with embedded expressions, use template literal
                    let re = Regex::new(r"\$[a-zA-Z][a-zA-Z0-9_.]*").unwrap();
                    let result = re.replace_all(s, |caps: &regex::Captures| {
                        format!("${{{}}}", Self::convert_runtime_expr(&caps[0]))
                    });
                    format!("`{}`", result)
                } else {
                    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
                }
            }
            _ => Self::json_to_js(value, 0),
        }
    }

    /// Generate the options export
    fn generate_options(options: &ConvertOptions) -> String {
        let mut parts = Vec::new();

        if let Some(vus) = options.vus {
            parts.push(format!("  vus: {},", vus));
        } else {
            parts.push("  vus: 1,".to_string());
        }

        if let Some(ref duration) = options.duration {
            parts.push(format!("  duration: '{}',", duration));
        } else if let Some(iterations) = options.iterations {
            parts.push(format!("  iterations: {},", iterations));
        } else {
            parts.push("  iterations: 1,".to_string());
        }

        format!("export let options = {{\n{}\n}};\n", parts.join("\n"))
    }

    /// Generate k6 code for a step
    fn generate_step(
        &self,
        step: &Step,
        openapi: &OpenApiV3Spec,
        base_url: &str,
    ) -> Result<String> {
        let mut lines = Vec::new();

        // Add step comment
        lines.push(self.generate_step_comment(step));

        // Get operation info
        let (path, method) = self.get_operation_info(step, openapi)?;

        // Build URL
        let final_url = self.build_url(base_url, &path, &step.parameters);

        // Generate request body and headers
        let (body_code, headers_code) = self.generate_body_and_headers(step);

        // Generate the HTTP call
        let response_var = format!("{}_response", step.step_id);
        let http_call = self.generate_http_call(
            &method,
            &response_var,
            &final_url,
            body_code.as_deref(),
            headers_code.as_deref(),
        );
        lines.push(http_call);

        // Generate check assertions from successCriteria
        if let Some(ref criteria) = step.success_criteria {
            let checks = self.generate_checks(criteria, &response_var);
            if !checks.is_empty() {
                lines.push(checks);
            }
        }

        // Generate output variable extractions
        if let Some(serde_json::Value::Object(map)) = step.outputs.as_ref() {
            for (name, expr) in map {
                if let serde_json::Value::String(expr_str) = expr {
                    let extraction = self.generate_output_extraction(
                        &step.step_id,
                        name,
                        expr_str,
                        &response_var,
                    );
                    lines.push(extraction);
                }
            }
        }

        lines.push(String::new()); // Empty line after step

        Ok(lines.join("\n"))
    }

    fn generate_step_comment(&self, step: &Step) -> String {
        if let Some(ref desc) = step.description {
            format!("  // Step: {} - {}", step.step_id, desc)
        } else {
            format!("  // Step: {}", step.step_id)
        }
    }

    fn get_operation_info(&self, step: &Step, openapi: &OpenApiV3Spec) -> Result<(String, String)> {
        if let Some(ref op_id) = step.operation_id {
            Self::find_operation(openapi, op_id).ok_or_else(|| {
                HornetError::OperationNotFound(format!("Operation '{}' not found", op_id))
            })
        } else if let Some(ref op_path) = step.operation_path {
            let parts: Vec<&str> = op_path.splitn(2, ' ').collect();
            if parts.len() == 2 {
                Ok((parts[1].to_string(), parts[0].to_uppercase()))
            } else {
                Err(HornetError::ValidationError(format!(
                    "Invalid operationPath format: {}",
                    op_path
                )))
            }
        } else {
            Err(HornetError::ValidationError(format!(
                "Step '{}' has no operationId or operationPath",
                step.step_id
            )))
        }
    }

    fn build_url(
        &self,
        base_url: &str,
        path: &str,
        parameters: &[crate::models::arazzo::Parameter],
    ) -> String {
        let url = format!("{}{}", base_url, path);
        let mut query_params: Vec<String> = Vec::new();
        let mut path_params: HashMap<String, String> = HashMap::new();

        for param in parameters {
            let value = Self::convert_value_with_expr(&param.value);
            match param.location.as_str() {
                "query" => {
                    query_params.push(format!("{}=${{{}}}", param.name, value));
                }
                "path" => {
                    path_params.insert(param.name.clone(), value);
                }
                _ => {}
            }
        }

        let mut final_url = if query_params.is_empty() {
            format!("\"{}\"", url)
        } else {
            format!("`{}?{}`", url, query_params.join("&"))
        };

        if !path_params.is_empty() {
            for (name, value) in &path_params {
                final_url = final_url.replace(&format!("{{{}}}", name), &format!("${{{}}}", value));
            }
            if final_url.starts_with('"') && final_url.contains("${") {
                final_url = format!("`{}`", &final_url[1..final_url.len() - 1]);
            }
        }

        final_url
    }

    fn generate_body_and_headers(&self, step: &Step) -> (Option<String>, Option<String>) {
        let mut headers: HashMap<String, String> = HashMap::new();

        // Process header parameters
        for param in &step.parameters {
            if param.location == "header" {
                headers.insert(
                    param.name.clone(),
                    Self::convert_value_with_expr(&param.value),
                );
            }
        }

        let body_code = step.request_body.as_ref().map(Self::generate_request_body);

        if step.request_body.is_some() {
            let content_type = step
                .request_body
                .as_ref()
                .and_then(|b| b.content_type.clone())
                .unwrap_or_else(|| "application/json".to_string());
            headers
                .entry("Content-Type".to_string())
                .or_insert_with(|| format!("\"{}\"", content_type));
        }

        let headers_code = if headers.is_empty() {
            None
        } else {
            let header_lines: Vec<String> = headers
                .iter()
                .map(|(k, v)| format!("      '{}': {}", k, v))
                .collect();
            Some(format!(
                "    headers: {{\n{}\n    }}",
                header_lines.join(",\n")
            ))
        };

        (body_code, headers_code)
    }

    fn generate_http_call(
        &self,
        method: &str,
        response_var: &str,
        url: &str,
        body: Option<&str>,
        headers: Option<&str>,
    ) -> String {
        match method {
            "GET" => {
                if let Some(h) = headers {
                    format!(
                        "  let {} = http.get({}, {{\n{}\n  }});",
                        response_var, url, h
                    )
                } else {
                    format!("  let {} = http.get({});", response_var, url)
                }
            }
            "DELETE" => {
                // Fixed: Support body in DELETE requests
                let body_arg = body.unwrap_or("null");
                if let Some(h) = headers {
                    format!(
                        "  let {} = http.del({}, {}, {{\n{}\n  }});",
                        response_var, url, body_arg, h
                    )
                } else {
                    format!("  let {} = http.del({}, {});", response_var, url, body_arg)
                }
            }
            "POST" | "PUT" | "PATCH" => {
                let method_fn = method.to_lowercase();
                let body_arg = body.unwrap_or("null");
                if let Some(h) = headers {
                    format!(
                        "  let {} = http.{}({}, {}, {{\n{}\n  }});",
                        response_var, method_fn, url, body_arg, h
                    )
                } else {
                    format!(
                        "  let {} = http.{}({}, {});",
                        response_var, method_fn, url, body_arg
                    )
                }
            }
            _ => {
                // Fixed: Support headers (params) in generic requests
                let body_arg = body.unwrap_or("null");
                if let Some(h) = headers {
                    format!(
                        "  let {} = http.request('{}', {}, {}, {{\n{}\n  }});",
                        response_var, method, url, body_arg, h
                    )
                } else {
                    format!(
                        "  let {} = http.request('{}', {}, {});",
                        response_var, method, url, body_arg
                    )
                }
            }
        }
    }

    /// Generate request body code
    fn generate_request_body(body: &RequestBody) -> String {
        // Check if payload contains runtime expressions
        let payload_str = Self::json_to_js(&body.payload, 0);

        // Wrap in JSON.stringify
        format!("JSON.stringify({})", payload_str)
    }

    /// Generate check assertions
    fn generate_checks(&self, criteria: &[SuccessCriteria], response_var: &str) -> String {
        let mut checks = Vec::new();

        for (i, crit) in criteria.iter().enumerate() {
            let check_name = format!("check_{}", i + 1);

            // Use 'r' instead of response_var in check callbacks
            let (left, right) = self.convert_criteria_to_check(crit, "r");

            let operator = match crit.condition.as_str() {
                "==" | "$eq" => "===",
                "!=" | "$ne" => "!==",
                ">" | "$gt" => ">",
                ">=" | "$gte" => ">=",
                "<" | "$lt" => "<",
                "<=" | "$lte" => "<=",
                _ => "===",
            };

            // Wrap JSON access in a try-catch or conditional
            let check_expr = if left.contains(".json()") {
                // For JSON access, add safety check
                format!(
                    "r.status >= 200 && r.status < 300 && {} {} {}",
                    left, operator, right
                )
            } else {
                format!("{} {} {}", left, operator, right)
            };

            checks.push(format!("    '{}': (r) => {}", check_name, check_expr));
        }

        if checks.is_empty() {
            String::new()
        } else {
            format!(
                "  check({}, {{\n{}\n  }});",
                response_var,
                checks.join(",\n")
            )
        }
    }

    /// Convert success criteria to check expression
    fn convert_criteria_to_check(
        &self,
        crit: &SuccessCriteria,
        response_var: &str,
    ) -> (String, String) {
        let left = self.convert_context_to_js(&crit.context, response_var);

        let right = if let Some(ref val) = crit.value {
            match val {
                serde_json::Value::String(s) if s.starts_with('$') => Self::convert_runtime_expr(s),
                _ => Self::json_to_js(val, 0),
            }
        } else {
            "true".to_string()
        };

        (left, right)
    }

    /// Convert context expression to JavaScript
    fn convert_context_to_js(&self, context: &str, response_var: &str) -> String {
        if context == "$statusCode" {
            format!("{}.status", response_var)
        } else if context.starts_with("$response.body.") {
            let field = context.strip_prefix("$response.body.").unwrap();
            // Use .json().field syntax for k6
            format!("{}.json().{}", response_var, field)
        } else if context.starts_with("$response.header.") {
            let header = context.strip_prefix("$response.header.").unwrap();
            format!("{}.headers['{}']", response_var, header)
        } else if context.starts_with("$steps.") {
            Self::convert_runtime_expr(context)
        } else {
            format!("\"{}\"", context)
        }
    }

    /// Generate output variable extraction
    fn generate_output_extraction(
        &self,
        step_id: &str,
        output_name: &str,
        expr: &str,
        response_var: &str,
    ) -> String {
        let var_name = format!("{}_{}", step_id, output_name);

        let extraction = if expr.starts_with("$response.body.") {
            let field = expr.strip_prefix("$response.body.").unwrap();
            // Always use .json().field syntax for k6
            format!("{}.json().{}", response_var, field)
        } else if expr == "$response.body" {
            format!("{}.json()", response_var)
        } else if expr.starts_with("$response.header.") {
            let header = expr.strip_prefix("$response.header.").unwrap();
            format!("{}.headers['{}']", response_var, header)
        } else {
            Self::convert_runtime_expr(expr)
        };

        // Wrap in a conditional to avoid parsing errors on failed responses
        format!(
            "  let {} = {}.status >= 200 && {}.status < 300 ? {} : undefined;",
            var_name, response_var, response_var, extraction
        )
    }

    /// Generate workflow with conditional branching support
    fn generate_workflow_with_branching(
        &self,
        workflow: &Workflow,
        openapi: &OpenApiV3Spec,
        base_url: &str,
    ) -> Result<String> {
        let mut lines = Vec::new();

        // Create step map for easy lookup
        let step_map: HashMap<String, &Step> = workflow
            .steps
            .iter()
            .map(|s| (s.step_id.clone(), s))
            .collect();

        // Generate each step as a separate function (with inputs parameter)
        for step in &workflow.steps {
            let func_name = format!("step_{}", step.step_id);
            lines.push(format!("function {}(inputs) {{", func_name));

            // Generate step code
            let step_code = self.generate_step(step, openapi, base_url)?;
            lines.push(step_code);

            // Return response variable for conditional logic
            let response_var = format!("{}_response", step.step_id);
            lines.push(format!("  return {};", response_var));
            lines.push("}".to_string());
            lines.push(String::new());
        }

        // Generate main function with control flow
        lines.push("export default function () {".to_string());

        // Add inputs
        let inputs = Self::generate_inputs(workflow);
        lines.push(inputs);

        // Generate control flow starting from first step
        if !workflow.steps.is_empty() {
            let control_flow = self.generate_control_flow(&workflow.steps, &step_map, 0, "  ")?;
            lines.push(control_flow);
        }

        lines.push("  sleep(1);".to_string());
        lines.push("}".to_string());

        Ok(lines.join("\n"))
    }

    /// Generate control flow for steps with branching
    fn generate_control_flow(
        &self,
        steps: &[Step],
        step_map: &HashMap<String, &Step>,
        start_idx: usize,
        indent: &str,
    ) -> Result<String> {
        let mut lines = Vec::new();
        let mut visited = std::collections::HashSet::new();
        let mut current_idx = start_idx;

        while current_idx < steps.len() {
            let step = &steps[current_idx];

            // Prevent infinite loops
            if visited.contains(&step.step_id) {
                break;
            }
            visited.insert(step.step_id.clone());

            let response_var = format!("{}_response", step.step_id);
            lines.push(format!("{}let {} = step_{}(inputs);", indent, response_var, step.step_id));

            // Check for branching
            if step.on_success.is_some() || step.on_failure.is_some() {
                // Determine success condition
                let success_check = if let Some(ref criteria) = step.success_criteria {
                    self.generate_success_condition(criteria, &response_var)
                } else {
                    // Default: check status code 2xx
                    format!("{}.status >= 200 && {}.status < 300", response_var, response_var)
                };

                lines.push(format!("{}if ({}) {{", indent, success_check));

                // Handle onSuccess actions
                if let Some(ref actions) = step.on_success {
                    for action in actions {
                        if action.action_type == "goto" {
                            if let Some(step_id) = action.config.get("stepId") {
                                if let Some(step_id_str) = step_id.as_str() {
                                    lines.push(format!(
                                        "{}  // onSuccess: goto {}",
                                        indent, step_id_str
                                    ));
                                    // Find target step and generate its execution
                                    if let Some(target_step) = step_map.get(step_id_str) {
                                        let target_response_var = format!("{}_response", target_step.step_id);
                                        lines.push(format!(
                                            "{}  let {} = step_{}(inputs);",
                                            indent, target_response_var, target_step.step_id
                                        ));
                                    }
                                }
                            }
                        } else if action.action_type == "end" {
                            lines.push(format!("{}  return; // onSuccess: end", indent));
                        }
                    }
                } else {
                    // No onSuccess action, continue to next step
                    lines.push(format!("{}  // Continue to next step", indent));
                }

                lines.push(format!("{}}} else {{", indent));

                // Handle onFailure actions
                if let Some(ref actions) = step.on_failure {
                    for action in actions {
                        // Check if action has specific criteria
                        if let Some(criteria_val) = action.config.get("criteria") {
                            if let Some(criteria_arr) = criteria_val.as_array() {
                                // Generate condition for criteria
                                let mut conditions = Vec::new();
                                for crit_val in criteria_arr {
                                    if let Some(context) = crit_val.get("context").and_then(|v| v.as_str()) {
                                        if let Some(condition) = crit_val.get("condition").and_then(|v| v.as_str()) {
                                            if let Some(value) = crit_val.get("value") {
                                                let left = self.convert_context_to_js(context, &response_var);
                                                let operator = match condition {
                                                    "==" | "$eq" => "===",
                                                    "!=" | "$ne" => "!==",
                                                    ">" | "$gt" => ">",
                                                    ">=" | "$gte" => ">=",
                                                    "<" | "$lt" => "<",
                                                    "<=" | "$lte" => "<=",
                                                    _ => "===",
                                                };
                                                let right = Self::json_to_js(value, 0);
                                                conditions.push(format!("{} {} {}", left, operator, right));
                                            }
                                        }
                                    }
                                }

                                if !conditions.is_empty() {
                                    lines.push(format!("{}  if ({}) {{", indent, conditions.join(" && ")));
                                }
                            }
                        }

                        if action.action_type == "goto" {
                            if let Some(step_id) = action.config.get("stepId") {
                                if let Some(step_id_str) = step_id.as_str() {
                                    lines.push(format!(
                                        "{}    // onFailure: goto {}",
                                        indent, step_id_str
                                    ));
                                    // Find target step and generate its execution
                                    if let Some(target_step) = step_map.get(step_id_str) {
                                        let target_response_var = format!("{}_response", target_step.step_id);
                                        lines.push(format!(
                                            "{}    let {} = step_{}(inputs);",
                                            indent, target_response_var, target_step.step_id
                                        ));
                                    }
                                }
                            }
                        } else if action.action_type == "end" {
                            lines.push(format!("{}    return; // onFailure: end", indent));
                        }

                        // Close criteria if block
                        if action.config.get("criteria").is_some() {
                            lines.push(format!("{}  }}", indent));
                        }
                    }
                } else {
                    // No onFailure action, just log and continue
                    lines.push(format!("{}  console.error('Step {} failed');", indent, step.step_id));
                }

                lines.push(format!("{}}}", indent));

                // After branching, we usually don't continue sequentially
                break;
            }

            current_idx += 1;
        }

        Ok(lines.join("\n"))
    }

    /// Generate control flow for multi-workflow mode (with function name prefix)
    fn generate_control_flow_for_multi(
        &self,
        steps: &[Step],
        step_map: &HashMap<String, &Step>,
        func_prefix: &str,
        indent: &str,
    ) -> Result<String> {
        let mut lines = Vec::new();
        let mut visited = std::collections::HashSet::new();
        let mut current_idx = 0;

        while current_idx < steps.len() {
            let step = &steps[current_idx];

            // Prevent infinite loops
            if visited.contains(&step.step_id) {
                break;
            }
            visited.insert(step.step_id.clone());

            let response_var = format!("{}_response", step.step_id);
            let step_func = format!("{}__step_{}", func_prefix, step.step_id);
            lines.push(format!("{}let {} = {}(inputs);", indent, response_var, step_func));

            // Check for branching
            if step.on_success.is_some() || step.on_failure.is_some() {
                // Determine success condition
                let success_check = if let Some(ref criteria) = step.success_criteria {
                    self.generate_success_condition(criteria, &response_var)
                } else {
                    // Default: check status code 2xx
                    format!("{}.status >= 200 && {}.status < 300", response_var, response_var)
                };

                lines.push(format!("{}if ({}) {{", indent, success_check));

                // Handle onSuccess actions
                if let Some(ref actions) = step.on_success {
                    for action in actions {
                        if action.action_type == "goto" {
                            if let Some(step_id) = action.config.get("stepId") {
                                if let Some(step_id_str) = step_id.as_str() {
                                    lines.push(format!(
                                        "{}  // onSuccess: goto {}",
                                        indent, step_id_str
                                    ));
                                    // Find target step and generate its execution
                                    if let Some(target_step) = step_map.get(step_id_str) {
                                        let target_response_var = format!("{}_response", target_step.step_id);
                                        let target_func = format!("{}__step_{}", func_prefix, target_step.step_id);
                                        lines.push(format!(
                                            "{}  let {} = {}(inputs);",
                                            indent, target_response_var, target_func
                                        ));
                                    }
                                }
                            }
                        } else if action.action_type == "end" {
                            lines.push(format!("{}  return; // onSuccess: end", indent));
                        }
                    }
                } else {
                    // No onSuccess action, continue to next step
                    lines.push(format!("{}  // Continue to next step", indent));
                }

                lines.push(format!("{}}} else {{", indent));

                // Handle onFailure actions
                if let Some(ref actions) = step.on_failure {
                    for action in actions {
                        // Check if action has specific criteria
                        if let Some(criteria_val) = action.config.get("criteria") {
                            if let Some(criteria_arr) = criteria_val.as_array() {
                                // Generate condition for criteria
                                let mut conditions = Vec::new();
                                for crit_val in criteria_arr {
                                    if let Some(context) = crit_val.get("context").and_then(|v| v.as_str()) {
                                        if let Some(condition) = crit_val.get("condition").and_then(|v| v.as_str()) {
                                            if let Some(value) = crit_val.get("value") {
                                                let left = self.convert_context_to_js(context, &response_var);
                                                let operator = match condition {
                                                    "==" | "$eq" => "===",
                                                    "!=" | "$ne" => "!==",
                                                    ">" | "$gt" => ">",
                                                    ">=" | "$gte" => ">=",
                                                    "<" | "$lt" => "<",
                                                    "<=" | "$lte" => "<=",
                                                    _ => "===",
                                                };
                                                let right = Self::json_to_js(value, 0);
                                                conditions.push(format!("{} {} {}", left, operator, right));
                                            }
                                        }
                                    }
                                }

                                if !conditions.is_empty() {
                                    lines.push(format!("{}  if ({}) {{", indent, conditions.join(" && ")));
                                }
                            }
                        }

                        if action.action_type == "goto" {
                            if let Some(step_id) = action.config.get("stepId") {
                                if let Some(step_id_str) = step_id.as_str() {
                                    lines.push(format!(
                                        "{}    // onFailure: goto {}",
                                        indent, step_id_str
                                    ));
                                    // Find target step and generate its execution
                                    if let Some(target_step) = step_map.get(step_id_str) {
                                        let target_response_var = format!("{}_response", target_step.step_id);
                                        let target_func = format!("{}__step_{}", func_prefix, target_step.step_id);
                                        lines.push(format!(
                                            "{}    let {} = {}(inputs);",
                                            indent, target_response_var, target_func
                                        ));
                                    }
                                }
                            }
                        } else if action.action_type == "end" {
                            lines.push(format!("{}    return; // onFailure: end", indent));
                        }

                        // Close criteria if block
                        if action.config.get("criteria").is_some() {
                            lines.push(format!("{}  }}", indent));
                        }
                    }
                } else {
                    // No onFailure action, just log and continue
                    lines.push(format!("{}  console.error('Step {} failed');", indent, step.step_id));
                }

                lines.push(format!("{}}}", indent));

                // After branching, we usually don't continue sequentially
                break;
            }

            current_idx += 1;
        }

        Ok(lines.join("\n"))
    }

    /// Generate success condition from criteria
    fn generate_success_condition(&self, criteria: &[SuccessCriteria], response_var: &str) -> String {
        let mut conditions = Vec::new();

        for crit in criteria {
            let left = self.convert_context_to_js(&crit.context, response_var);
            let operator = match crit.condition.as_str() {
                "==" | "$eq" => "===",
                "!=" | "$ne" => "!==",
                ">" | "$gt" => ">",
                ">=" | "$gte" => ">=",
                "<" | "$lt" => "<",
                "<=" | "$lte" => "<=",
                _ => "===",
            };
            let right = if let Some(ref val) = crit.value {
                Self::json_to_js(val, 0)
            } else {
                "true".to_string()
            };

            conditions.push(format!("{} {} {}", left, operator, right));
        }

        if conditions.is_empty() {
            "true".to_string()
        } else {
            conditions.join(" && ")
        }
    }

    /// Generate workflow inputs
    fn generate_inputs(workflow: &Workflow) -> String {
        if let Some(serde_json::Value::Object(props_map)) =
            workflow.inputs.as_ref().and_then(|i| i.get("properties"))
        {
            let mut input_lines = Vec::new();
            for (name, schema) in props_map {
                let default_val = schema.get("default");
                let value = if let Some(def) = default_val {
                    Self::json_to_js(def, 0)
                } else {
                    match schema.get("type").and_then(|t| t.as_str()) {
                        Some("string") => "\"\"".to_string(),
                        Some("number") | Some("integer") => "0".to_string(),
                        Some("boolean") => "false".to_string(),
                        _ => "null".to_string(),
                    }
                };
                input_lines.push(format!("    {}: {},", name, value));
            }
            return format!("  let inputs = {{\n{}\n  }};\n", input_lines.join("\n"));
        }
        "  let inputs = {};\n".to_string()
    }
}

impl Converter for K6Converter {
    type Output = String;

    fn convert_spec(
        &self,
        arazzo: &ArazzoSpec,
        openapi: &OpenApiV3Spec,
        options: &ConvertOptions,
    ) -> Result<Self::Output> {
        if arazzo.workflows.is_empty() {
            return Ok(String::new());
        }

        // If only one workflow, generate it directly
        if arazzo.workflows.len() == 1 {
            return self.convert_workflow(&arazzo.workflows[0], openapi, options);
        }

        // For multiple workflows, combine them into one script with separate functions
        let base_url = options
            .base_url
            .clone()
            .unwrap_or_else(|| Self::get_base_url(openapi));

        let mut lines = Vec::new();

        // Add header comment
        lines.push("// k6 script generated from Arazzo specification".to_string());
        lines.push(format!("// Contains {} workflows", arazzo.workflows.len()));
        lines.push(String::new());

        // Add imports
        lines.push("import http from 'k6/http';".to_string());
        lines.push("import { check, sleep } from 'k6';".to_string());
        lines.push(String::new());

        // Add options
        lines.push(Self::generate_options(options));

        // Generate each workflow as a separate function
        for workflow in &arazzo.workflows {
            let func_name = workflow.workflow_id.replace('-', "_");
            lines.push(format!("// Workflow: {}", workflow.workflow_id));
            if let Some(ref summary) = workflow.summary {
                lines.push(format!("// {}", summary));
            }

            // Check if workflow has conditional branching
            let has_branching = workflow
                .steps
                .iter()
                .any(|s| s.on_success.is_some() || s.on_failure.is_some());

            if has_branching {
                // Generate step functions for branching workflow
                let step_map: HashMap<String, &Step> = workflow
                    .steps
                    .iter()
                    .map(|s| (s.step_id.clone(), s))
                    .collect();

                // Generate each step as a separate function (with inputs parameter)
                for step in &workflow.steps {
                    let step_func_name = format!("{}__step_{}", func_name, step.step_id);
                    lines.push(format!("function {}(inputs) {{", step_func_name));

                    // Generate step code
                    let step_code = self.generate_step(step, openapi, &base_url)?;
                    lines.push(step_code);

                    // Return response variable for conditional logic
                    let response_var = format!("{}_response", step.step_id);
                    lines.push(format!("  return {};", response_var));
                    lines.push("}".to_string());
                    lines.push(String::new());
                }

                // Generate main workflow function with control flow
                lines.push(format!("function {}() {{", func_name));

                // Add inputs
                let inputs = Self::generate_inputs(workflow);
                lines.push(inputs);

                // Generate control flow
                let control_flow = self.generate_control_flow_for_multi(&workflow.steps, &step_map, &func_name, "  ")?;
                lines.push(control_flow);

                lines.push("}".to_string());
                lines.push(String::new());
            } else {
                // Simple sequential workflow
                lines.push(format!("function {}() {{", func_name));

                // Add inputs
                let inputs = Self::generate_inputs(workflow);
                lines.push(inputs);

                // Generate steps
                for step in &workflow.steps {
                    let step_code = self.generate_step(step, openapi, &base_url)?;
                    lines.push(step_code);
                }

                lines.push("}".to_string());
                lines.push(String::new());
            }
        }

        // Add default function that calls all workflows
        lines.push("export default function () {".to_string());
        for workflow in &arazzo.workflows {
            let func_name = workflow.workflow_id.replace('-', "_");
            lines.push(format!("  {}();", func_name));
        }
        lines.push("  sleep(1);".to_string());
        lines.push("}".to_string());

        Ok(lines.join("\n"))
    }

    fn convert_workflow(
        &self,
        workflow: &Workflow,
        openapi: &OpenApiV3Spec,
        options: &ConvertOptions,
    ) -> Result<Self::Output> {
        let base_url = options
            .base_url
            .clone()
            .unwrap_or_else(|| Self::get_base_url(openapi));

        let mut lines = Vec::new();

        // Add header comment
        lines.push(format!(
            "// k6 script generated from Arazzo workflow: {}",
            workflow.workflow_id
        ));
        if let Some(ref summary) = workflow.summary {
            lines.push(format!("// {}", summary));
        }
        lines.push(String::new());

        // Add imports
        lines.push("import http from 'k6/http';".to_string());
        lines.push("import { check, sleep } from 'k6';".to_string());
        lines.push(String::new());

        // Add options
        lines.push(Self::generate_options(options));

        // Check if workflow has conditional branching
        let has_branching = workflow
            .steps
            .iter()
            .any(|s| s.on_success.is_some() || s.on_failure.is_some());

        if has_branching {
            // Generate workflow with conditional branching support
            lines.push(self.generate_workflow_with_branching(workflow, openapi, &base_url)?);
        } else {
            // Generate simple sequential workflow
            lines.push("export default function () {".to_string());

            // Add inputs
            let inputs = Self::generate_inputs(workflow);
            lines.push(inputs);

            // Generate steps
            for step in &workflow.steps {
                let step_code = self.generate_step(step, openapi, &base_url)?;
                lines.push(step_code);
            }

            // Add sleep at end (optional, for load testing)
            lines.push("  sleep(1);".to_string());
            lines.push("}".to_string());
        }

        Ok(lines.join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::loader::{arazzo::load_arazzo, openapi::load_openapi};

    #[test]
    fn test_convert_runtime_expr() {
        assert_eq!(
            K6Converter::convert_runtime_expr("$inputs.username"),
            "inputs.username"
        );
        assert_eq!(
            K6Converter::convert_runtime_expr("$steps.login.outputs.token"),
            "login_token"
        );
        assert_eq!(
            K6Converter::convert_runtime_expr("$statusCode"),
            "response.status"
        );
        assert_eq!(
            K6Converter::convert_runtime_expr("$response.body.id"),
            "response.json('id')"
        );
    }

    #[test]
    fn test_json_to_js() {
        let json = serde_json::json!({
            "name": "test",
            "value": 42
        });
        let js = K6Converter::json_to_js(&json, 0);
        assert!(js.contains("\"name\": \"test\""));
        assert!(js.contains("\"value\": 42"));
    }

    #[test]
    fn test_generate_options() {
        let options = ConvertOptions {
            vus: Some(10),
            duration: Some("30s".to_string()),
            ..Default::default()
        };
        let code = K6Converter::generate_options(&options);
        assert!(code.contains("vus: 10"));
        assert!(code.contains("duration: '30s'"));
    }

    #[test]
    fn test_generate_http_call_delete_with_body() {
        let converter = K6Converter::new();
        let code = converter.generate_http_call(
            "DELETE",
            "resp",
            "http://example.com",
            Some("JSON.stringify({id: 1})"),
            Some("headers: {'Content-Type': 'application/json'}"),
        );
        assert!(code.contains("http.del(http://example.com, JSON.stringify({id: 1}), {"));
        assert!(code.contains("headers: {'Content-Type': 'application/json'}"));
    }

    #[test]
    fn test_generate_http_call_generic_with_headers() {
        let converter = K6Converter::new();
        // Test OPTIONS with headers
        let code = converter.generate_http_call(
            "OPTIONS",
            "resp",
            "http://example.com",
            None,
            Some("headers: {'X-Custom': 'value'}"),
        );
        assert!(code.contains("http.request('OPTIONS', http://example.com, null, {"));
        assert!(code.contains("headers: {'X-Custom': 'value'}"));
    }

    #[test]
    fn test_convert_workflow_integration() {
        let arazzo = load_arazzo("tests/fixtures/arazzo.yaml").unwrap();
        let openapi = load_openapi("tests/fixtures/openapi.yaml").unwrap();

        let converter = K6Converter::new();
        let options = ConvertOptions::default();

        let result = converter.convert_workflow(&arazzo.workflows[0], &openapi, &options);
        assert!(result.is_ok());

        let script = result.unwrap();
        assert!(script.contains("import http from 'k6/http'"));
        assert!(script.contains("export default function"));
        assert!(!script.contains("registerUser")); // Should use actual paths
        assert!(script.contains("/register") || script.contains("/login"));
    }
}
