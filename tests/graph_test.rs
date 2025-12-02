use hornet2::{
    graph::{builder::build_flow_graph, exporter::{export_dot, export_json}, validator::validate_flow_graph},
    loader::{load_arazzo, load_openapi},
};
use std::path::Path;

#[test]
fn test_build_graph_from_fixture() {
    let arazzo_path = Path::new("tests/fixtures/arazzo.yaml");
    let arazzo = load_arazzo(arazzo_path).unwrap();

    // Build graph for first workflow
    let workflow = &arazzo.workflows[0];
    let graph = build_flow_graph(workflow, None).unwrap();

    assert_eq!(graph.workflow_id, "user-onboarding-flow");
    assert_eq!(graph.node_count(), 4); // 4 steps
    assert!(graph.edge_count() >= 4); // At least sequential edges + data dependencies
}

#[test]
fn test_build_graph_with_openapi() {
    let arazzo_path = Path::new("tests/fixtures/arazzo.yaml");
    let openapi_path = Path::new("tests/fixtures/openapi.yaml");

    let arazzo = load_arazzo(arazzo_path).unwrap();
    let openapi = load_openapi(openapi_path).unwrap();

    let workflow = &arazzo.workflows[0];
    let graph = build_flow_graph(workflow, Some(&openapi)).unwrap();

    // Check that HTTP methods were resolved
    let register_node = graph.get_node("register").unwrap();
    assert_eq!(register_node.method, Some("POST".to_string()));

    let login_node = graph.get_node("login").unwrap();
    assert_eq!(login_node.method, Some("POST".to_string()));

    let get_profile_node = graph.get_node("getProfile").unwrap();
    assert_eq!(get_profile_node.method, Some("GET".to_string()));

    let update_profile_node = graph.get_node("updateProfile").unwrap();
    assert_eq!(update_profile_node.method, Some("PUT".to_string()));
}

#[test]
fn test_data_dependencies() {
    let arazzo_path = Path::new("tests/fixtures/arazzo.yaml");
    let arazzo = load_arazzo(arazzo_path).unwrap();

    let workflow = &arazzo.workflows[0];
    let graph = build_flow_graph(workflow, None).unwrap();

    // 4 nodes: register, login, getProfile, updateProfile
    // 3 sequential edges: register->login, login->getProfile, getProfile->updateProfile
    // 4 conditional self-loops (one per step with success_criteria)
    assert_eq!(graph.node_count(), 4);
    assert!(graph.edge_count() >= 3); // At least sequential edges
}

#[test]
fn test_validate_graph() {
    let arazzo_path = Path::new("tests/fixtures/arazzo.yaml");
    let arazzo = load_arazzo(arazzo_path).unwrap();

    let workflow = &arazzo.workflows[0];
    let graph = build_flow_graph(workflow, None).unwrap();

    let validation = validate_flow_graph(&graph).unwrap();

    println!("Validation: {}", validation.summary());
    println!("Errors: {:?}", validation.errors);
    println!("Warnings: {:?}", validation.warnings);

    // Graph should be valid (or at least have no errors)
    // Warnings are acceptable (e.g., topological order info)
    assert!(
        validation.errors.is_empty(),
        "Graph should have no errors: {:?}",
        validation.errors
    );
}

#[test]
fn test_export_dot() {
    let arazzo_path = Path::new("tests/fixtures/arazzo.yaml");
    let openapi_path = Path::new("tests/fixtures/openapi.yaml");

    let arazzo = load_arazzo(arazzo_path).unwrap();
    let openapi = load_openapi(openapi_path).unwrap();

    let workflow = &arazzo.workflows[0];
    let graph = build_flow_graph(workflow, Some(&openapi)).unwrap();

    let dot = export_dot(&graph);

    // Check basic DOT structure
    assert!(dot.contains("digraph"));
    assert!(dot.contains("user-onboarding-flow"));
    assert!(dot.contains("register"));
    assert!(dot.contains("login"));
    assert!(dot.contains("getProfile"));
    assert!(dot.contains("updateProfile"));
    assert!(dot.contains("POST"));
    assert!(dot.contains("GET"));
    assert!(dot.contains("PUT"));

    println!("DOT output:\n{}", dot);
}

#[test]
fn test_export_json() {
    let arazzo_path = Path::new("tests/fixtures/arazzo.yaml");
    let openapi_path = Path::new("tests/fixtures/openapi.yaml");

    let arazzo = load_arazzo(arazzo_path).unwrap();
    let openapi = load_openapi(openapi_path).unwrap();

    let workflow = &arazzo.workflows[0];
    let graph = build_flow_graph(workflow, Some(&openapi)).unwrap();

    let json = export_json(&graph).unwrap();

    // Check JSON structure
    assert_eq!(json["workflowId"], "user-onboarding-flow");
    assert_eq!(json["nodes"].as_array().unwrap().len(), 4);
    assert_eq!(json["stats"]["nodeCount"], 4);

    // Check node details
    let nodes = json["nodes"].as_array().unwrap();
    assert_eq!(nodes[0]["id"], "register");
    assert_eq!(nodes[0]["method"], "POST");
    assert_eq!(nodes[1]["id"], "login");
    assert_eq!(nodes[1]["method"], "POST");

    println!("JSON output:\n{}", serde_json::to_string_pretty(&json).unwrap());
}

#[test]
fn test_second_workflow() {
    let arazzo_path = Path::new("tests/fixtures/arazzo.yaml");
    let arazzo = load_arazzo(arazzo_path).unwrap();

    // Build graph for second workflow (simple-login-flow)
    let workflow = &arazzo.workflows[1];
    let graph = build_flow_graph(workflow, None).unwrap();

    assert_eq!(graph.workflow_id, "simple-login-flow");
    assert_eq!(graph.node_count(), 1); // Only 1 step
}
