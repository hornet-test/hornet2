use super::{EdgeType, FlowGraph};
use crate::error::Result;
use petgraph::visit::EdgeRef;
use serde_json::{json, Value};

/// Exporter for flow graphs
pub struct FlowGraphExporter<'a> {
    graph: &'a FlowGraph,
}

impl<'a> FlowGraphExporter<'a> {
    /// Create a new exporter
    pub fn new(graph: &'a FlowGraph) -> Self {
        Self { graph }
    }

    /// Export to DOT format (Graphviz)
    pub fn export_dot(&self) -> String {
        let mut dot = String::new();

        // Header
        dot.push_str(&format!("digraph \"{}\" {{\n", self.graph.workflow_id));
        dot.push_str("  rankdir=LR;\n");
        dot.push_str("  node [shape=box, style=rounded];\n\n");

        // Nodes
        for node_idx in self.graph.graph.node_indices() {
            if let Some(node) = self.graph.graph.node_weight(node_idx) {
                let label = self.format_node_label(node);
                let color = self.get_node_color(node);

                dot.push_str(&format!(
                    "  \"{}\" [label=\"{}\", color=\"{}\", fillcolor=\"{}\", style=\"rounded,filled\"];\n",
                    node.step_id,
                    label,
                    color,
                    self.get_fill_color(color)
                ));
            }
        }

        dot.push('\n');

        // Edges
        for edge in self.graph.graph.edge_references() {
            let source = &self.graph.graph[edge.source()].step_id;
            let target = &self.graph.graph[edge.target()].step_id;
            let edge_data = edge.weight();

            let (style, color, label) = self.format_edge_style(edge_data);

            dot.push_str(&format!(
                "  \"{}\" -> \"{}\" [style=\"{}\", color=\"{}\", label=\"{}\"];\n",
                source, target, style, color, label
            ));
        }

        dot.push_str("}\n");
        dot
    }

    /// Export to JSON format
    pub fn export_json(&self) -> Result<Value> {
        let mut nodes = vec![];
        let mut edges = vec![];

        // Export nodes
        for node_idx in self.graph.graph.node_indices() {
            if let Some(node) = self.graph.graph.node_weight(node_idx) {
                nodes.push(json!({
                    "id": node.step_id,
                    "operationId": node.operation_id,
                    "operationPath": node.operation_path,
                    "method": node.method,
                    "description": node.description,
                    "hasOutputs": node.has_outputs,
                    "hasSuccessCriteria": node.has_success_criteria,
                }));
            }
        }

        // Export edges
        for edge in self.graph.graph.edge_references() {
            let source = &self.graph.graph[edge.source()].step_id;
            let target = &self.graph.graph[edge.target()].step_id;
            let edge_data = edge.weight();

            edges.push(json!({
                "source": source,
                "target": target,
                "edge_type": format!("{:?}", edge_data.edge_type),
                "dataRef": edge_data.data_ref,
                "description": edge_data.description,
            }));
        }

        Ok(json!({
            "workflowId": self.graph.workflow_id,
            "nodes": nodes,
            "edges": edges,
            "stats": {
                "nodeCount": self.graph.node_count(),
                "edgeCount": self.graph.edge_count(),
            }
        }))
    }

    /// Format node label for DOT
    fn format_node_label(&self, node: &super::FlowNode) -> String {
        let mut label = node.step_id.clone();

        if let Some(ref op_id) = node.operation_id {
            label.push_str(&format!("\\n{}", op_id));
        }

        if let Some(ref method) = node.method {
            label.push_str(&format!("\\n[{}]", method));
        }

        label
    }

    /// Get node color based on properties
    fn get_node_color(&self, node: &super::FlowNode) -> &'static str {
        if node.has_success_criteria {
            "orange" // Conditional node
        } else if node.has_outputs {
            "blue" // Node with outputs
        } else {
            "black" // Regular node
        }
    }

    /// Get fill color for nodes
    fn get_fill_color(&self, color: &str) -> &'static str {
        match color {
            "orange" => "lightyellow",
            "blue" => "lightblue",
            _ => "lightgray",
        }
    }

    /// Format edge style for DOT
    fn format_edge_style(&self, edge: &super::FlowEdge) -> (&'static str, &'static str, String) {
        match edge.edge_type {
            EdgeType::Sequential => ("solid", "black", "".to_string()),
            EdgeType::Conditional => (
                "dashed",
                "orange",
                edge.description.clone().unwrap_or_default(),
            ),
            EdgeType::DataDependency => {
                ("dotted", "blue", edge.data_ref.clone().unwrap_or_default())
            }
            EdgeType::OnSuccess => (
                "solid",
                "green",
                edge.description.clone().unwrap_or_default(),
            ),
            EdgeType::OnFailure => ("solid", "red", edge.description.clone().unwrap_or_default()),
        }
    }

    /// Export to Mermaid format
    pub fn export_mermaid(&self) -> String {
        let mut mermaid = String::new();

        // Header
        mermaid.push_str("flowchart LR\n");

        // Nodes
        for node_idx in self.graph.graph.node_indices() {
            if let Some(node) = self.graph.graph.node_weight(node_idx) {
                let label = self.format_mermaid_node_label(node);
                let shape = self.get_mermaid_node_shape(node);

                mermaid.push_str(&format!(
                    "  {}{}{}]\n",
                    node.step_id.replace('-', "_"),
                    shape.0,
                    label
                ));
            }
        }

        mermaid.push('\n');

        // Edges
        for edge in self.graph.graph.edge_references() {
            let source = &self.graph.graph[edge.source()].step_id.replace('-', "_");
            let target = &self.graph.graph[edge.target()].step_id.replace('-', "_");
            let edge_data = edge.weight();

            let arrow = match edge_data.edge_type {
                EdgeType::Sequential => "-->",
                EdgeType::Conditional => "-.->",
                EdgeType::DataDependency => "==>",
                EdgeType::OnSuccess => "==>",
                EdgeType::OnFailure => "-.->",
            };

            if let Some(label) = self.get_mermaid_edge_label(edge_data) {
                mermaid.push_str(&format!("  {} {}|{}| {}\n", source, arrow, label, target));
            } else {
                mermaid.push_str(&format!("  {} {} {}\n", source, arrow, target));
            }
        }

        mermaid
    }

    /// Format node label for Mermaid
    fn format_mermaid_node_label(&self, node: &super::FlowNode) -> String {
        let mut label = node.step_id.clone();

        if let Some(ref op_id) = node.operation_id {
            label.push_str(&format!("<br/>{}", op_id));
        }

        if let Some(ref method) = node.method {
            label.push_str(&format!("<br/>[{}]", method));
        }

        label
    }

    /// Get Mermaid node shape
    fn get_mermaid_node_shape(&self, node: &super::FlowNode) -> (&'static str, &'static str) {
        if node.has_success_criteria {
            ("(", ")") // Rectangular with rounded corners
        } else if node.has_outputs {
            ("([", "])") // Stadium shape
        } else {
            ("[", "]") // Regular rectangle
        }
    }

    /// Get edge label for Mermaid
    fn get_mermaid_edge_label(&self, edge: &super::FlowEdge) -> Option<String> {
        match edge.edge_type {
            EdgeType::Sequential => None,
            EdgeType::Conditional => edge.description.clone(),
            EdgeType::DataDependency => edge.data_ref.clone(),
            EdgeType::OnSuccess => edge.description.clone(),
            EdgeType::OnFailure => edge.description.clone(),
        }
    }
}

/// Export flow graph to DOT format
pub fn export_dot(graph: &FlowGraph) -> String {
    FlowGraphExporter::new(graph).export_dot()
}

/// Export flow graph to JSON format
pub fn export_json(graph: &FlowGraph) -> Result<Value> {
    FlowGraphExporter::new(graph).export_json()
}

/// Export flow graph to Mermaid format
pub fn export_mermaid(graph: &FlowGraph) -> String {
    FlowGraphExporter::new(graph).export_mermaid()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{FlowEdge, FlowNode};

    #[test]
    fn test_export_dot() {
        let mut graph = FlowGraph::new("test-workflow".to_string());

        let n1 = graph.add_node(FlowNode {
            step_id: "step1".to_string(),
            operation_id: Some("getUser".to_string()),
            operation_path: None,
            method: Some("GET".to_string()),
            description: None,
            has_outputs: true,
            has_success_criteria: false,
        });

        let n2 = graph.add_node(FlowNode {
            step_id: "step2".to_string(),
            operation_id: Some("updateUser".to_string()),
            operation_path: None,
            method: Some("PUT".to_string()),
            description: None,
            has_outputs: false,
            has_success_criteria: false,
        });

        graph.add_edge(n1, n2, FlowEdge::sequential());
        graph.add_edge(
            n1,
            n2,
            FlowEdge::data_dependency("$steps.step1.outputs.id".to_string()),
        );

        let dot = export_dot(&graph);

        assert!(dot.contains("digraph \"test-workflow\""));
        assert!(dot.contains("step1"));
        assert!(dot.contains("step2"));
        assert!(dot.contains("getUser"));
        assert!(dot.contains("GET"));
    }

    #[test]
    fn test_export_json() {
        let mut graph = FlowGraph::new("test-workflow".to_string());

        let n1 = graph.add_node(FlowNode {
            step_id: "step1".to_string(),
            operation_id: Some("getUser".to_string()),
            operation_path: None,
            method: Some("GET".to_string()),
            description: None,
            has_outputs: true,
            has_success_criteria: false,
        });

        let n2 = graph.add_node(FlowNode {
            step_id: "step2".to_string(),
            operation_id: Some("updateUser".to_string()),
            operation_path: None,
            method: Some("PUT".to_string()),
            description: None,
            has_outputs: false,
            has_success_criteria: false,
        });

        graph.add_edge(n1, n2, FlowEdge::sequential());

        let json = export_json(&graph).unwrap();

        assert_eq!(json["workflowId"], "test-workflow");
        assert_eq!(json["nodes"].as_array().unwrap().len(), 2);
        assert_eq!(json["edges"].as_array().unwrap().len(), 1);
        assert_eq!(json["stats"]["nodeCount"], 2);
        assert_eq!(json["stats"]["edgeCount"], 1);
    }
}
