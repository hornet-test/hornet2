use super::{ErrorType, ValidationError, ValidationWarning};
use crate::error::Result;
use crate::models::arazzo::ArazzoSpec;
use oas3::OpenApiV3Spec;
use regex::Regex;
use std::collections::{HashMap, HashSet};

/// Validator for data dependencies and runtime expressions
pub struct DataDependencyValidator<'a> {
    arazzo: &'a ArazzoSpec,
    #[allow(dead_code)]
    openapi: &'a OpenApiV3Spec,
}

impl<'a> DataDependencyValidator<'a> {
    pub fn new(arazzo: &'a ArazzoSpec, openapi: &'a OpenApiV3Spec) -> Self {
        Self { arazzo, openapi }
    }

    /// Validate data dependencies
    pub fn validate(&self) -> Result<(Vec<ValidationError>, Vec<ValidationWarning>)> {
        let mut errors = vec![];
        let mut warnings = vec![];

        for workflow in &self.arazzo.workflows {
            // Build step order map for this workflow
            let step_order: HashMap<String, usize> = workflow
                .steps
                .iter()
                .enumerate()
                .map(|(i, step)| (step.step_id.clone(), i))
                .collect();

            // Build set of step IDs for existence checks
            let step_ids: HashSet<String> =
                workflow.steps.iter().map(|s| s.step_id.clone()).collect();

            // Track which outputs are defined
            let mut defined_outputs: HashMap<String, Vec<String>> = HashMap::new();
            for step in &workflow.steps {
                if step.outputs.is_some() {
                    defined_outputs.insert(step.step_id.clone(), vec![]);
                }
            }

            // Track which outputs are referenced
            let mut referenced_outputs: HashSet<String> = HashSet::new();

            for (current_idx, step) in workflow.steps.iter().enumerate() {
                // Extract runtime references from this step
                let refs = self.extract_runtime_references(step);

                // Validate $steps references
                for step_ref in &refs.step_refs {
                    // Check if referenced step exists
                    if !step_ids.contains(&step_ref.step_id) {
                        errors.push(
                            ValidationError::new(
                                ErrorType::InvalidStepReference,
                                format!(
                                    "Invalid step reference: $steps.{}.outputs.{} refers to non-existent step '{}'",
                                    step_ref.step_id, step_ref.field, step_ref.step_id
                                ),
                            )
                            .with_workflow(&workflow.workflow_id)
                            .with_step(&step.step_id),
                        );
                        continue;
                    }

                    // Check if referenced step comes before current step
                    if let Some(&ref_idx) = step_order.get(&step_ref.step_id)
                        && ref_idx >= current_idx
                    {
                        errors.push(
                                ValidationError::new(
                                    ErrorType::StepOrderViolation,
                                    format!(
                                        "Step order violation: step '{}' references outputs from step '{}' which comes after it in execution order",
                                        step.step_id, step_ref.step_id
                                    ),
                                )
                                .with_workflow(&workflow.workflow_id)
                                .with_step(&step.step_id),
                            );
                    }

                    // Track that this output is referenced
                    referenced_outputs.insert(format!("{}.{}", step_ref.step_id, step_ref.field));
                }

                // Validate $inputs references
                for input_ref in &refs.input_refs {
                    // Check if workflow defines inputs
                    if workflow.inputs.is_none() {
                        warnings.push(
                            ValidationWarning::new(format!(
                                "Input reference $inputs.{} used but workflow does not define inputs",
                                input_ref
                            ))
                            .with_workflow(&workflow.workflow_id)
                            .with_step(&step.step_id),
                        );
                    }
                }
            }

            // Warn about unused outputs
            for step in &workflow.steps {
                if step.outputs.is_some() {
                    // Check if any output from this step is referenced
                    let step_referenced = referenced_outputs
                        .iter()
                        .any(|ref_output| ref_output.starts_with(&format!("{}.", step.step_id)));

                    if !step_referenced {
                        warnings.push(
                            ValidationWarning::new(
                                "Unused output: step defines outputs but they are never referenced by subsequent steps"
                            )
                            .with_workflow(&workflow.workflow_id)
                            .with_step(&step.step_id),
                        );
                    }
                }
            }
        }

        Ok((errors, warnings))
    }

    /// Extract runtime references from a step
    fn extract_runtime_references(&self, step: &crate::models::arazzo::Step) -> RuntimeReferences {
        let mut refs = RuntimeReferences {
            step_refs: vec![],
            input_refs: vec![],
            response_refs: vec![],
        };

        // Regex patterns
        let step_re = Regex::new(r"\$steps\.([a-zA-Z0-9_-]+)\.outputs\.([a-zA-Z0-9_.-]+)").unwrap();
        let input_re = Regex::new(r"\$inputs\.([a-zA-Z0-9_.-]+)").unwrap();
        let response_re = Regex::new(r"\$response\.(body|header)\.([a-zA-Z0-9_.-]+)").unwrap();
        let status_code_re = Regex::new(r"\$statusCode").unwrap();

        // Check parameters
        for param in &step.parameters {
            if let Some(value_str) = param.value.as_str() {
                for cap in step_re.captures_iter(value_str) {
                    refs.step_refs.push(StepReference {
                        step_id: cap[1].to_string(),
                        field: cap[2].to_string(),
                    });
                }

                for cap in input_re.captures_iter(value_str) {
                    refs.input_refs.push(cap[1].to_string());
                }

                for cap in response_re.captures_iter(value_str) {
                    refs.response_refs.push(ResponseReference {
                        location: cap[1].to_string(),
                        field: cap[2].to_string(),
                    });
                }

                if status_code_re.is_match(value_str) {
                    refs.response_refs.push(ResponseReference {
                        location: "statusCode".to_string(),
                        field: String::new(),
                    });
                }
            }
        }

        // Check request body
        if let Some(ref body) = step.request_body {
            let body_str = serde_json::to_string(&body.payload).unwrap_or_default();

            for cap in step_re.captures_iter(&body_str) {
                refs.step_refs.push(StepReference {
                    step_id: cap[1].to_string(),
                    field: cap[2].to_string(),
                });
            }

            for cap in input_re.captures_iter(&body_str) {
                refs.input_refs.push(cap[1].to_string());
            }
        }

        // Check success criteria
        if let Some(ref criteria) = step.success_criteria {
            for criterion in criteria {
                let context_str = &criterion.context;

                for cap in response_re.captures_iter(context_str) {
                    refs.response_refs.push(ResponseReference {
                        location: cap[1].to_string(),
                        field: cap[2].to_string(),
                    });
                }

                if status_code_re.is_match(context_str) {
                    refs.response_refs.push(ResponseReference {
                        location: "statusCode".to_string(),
                        field: String::new(),
                    });
                }
            }
        }

        refs
    }
}

/// Runtime references found in a step
#[derive(Debug)]
struct RuntimeReferences {
    step_refs: Vec<StepReference>,
    input_refs: Vec<String>,
    response_refs: Vec<ResponseReference>,
}

/// Reference to another step's output
#[derive(Debug)]
struct StepReference {
    step_id: String,
    field: String,
}

/// Reference to response data
#[derive(Debug)]
#[allow(dead_code)]
struct ResponseReference {
    location: String, // "body", "header", or "statusCode"
    field: String,
}
