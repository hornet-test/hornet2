use hornet2::loader::{load_arazzo, load_openapi};
use std::path::Path;

#[test]
fn test_load_openapi_fixture() {
    let path = Path::new("tests/fixtures/openapi.yaml");
    let result = load_openapi(path);

    assert!(result.is_ok(), "Failed to load OpenAPI fixture");

    let spec = result.unwrap();
    assert_eq!(spec.info.title, "User Management API");
    assert_eq!(spec.info.version, "1.0.0");
    assert_eq!(spec.openapi, "3.0.0");

    // Check that we have the expected paths
    let paths = spec.paths.unwrap();
    assert!(paths.contains_key("/register"));
    assert!(paths.contains_key("/login"));
    assert!(paths.contains_key("/profile"));
}

#[test]
fn test_load_arazzo_fixture() {
    let path = Path::new("tests/fixtures/arazzo.yaml");
    let result = load_arazzo(path);

    assert!(result.is_ok(), "Failed to load Arazzo fixture");

    let spec = result.unwrap();
    assert_eq!(spec.info.title, "User Registration and Profile Update Flow");
    assert_eq!(spec.info.version, "1.0.0");
    assert_eq!(spec.arazzo, "1.0.0");

    // Check workflows
    assert_eq!(spec.workflows.len(), 2);

    let first_workflow = &spec.workflows[0];
    assert_eq!(first_workflow.workflow_id, "user-onboarding-flow");
    assert_eq!(first_workflow.steps.len(), 4);

    // Check step IDs
    assert_eq!(first_workflow.steps[0].step_id, "register");
    assert_eq!(first_workflow.steps[1].step_id, "login");
    assert_eq!(first_workflow.steps[2].step_id, "getProfile");
    assert_eq!(first_workflow.steps[3].step_id, "updateProfile");

    // Check operation IDs
    assert_eq!(
        first_workflow.steps[0].operation_id.as_ref().unwrap(),
        "registerUser"
    );
    assert_eq!(
        first_workflow.steps[1].operation_id.as_ref().unwrap(),
        "loginUser"
    );
}

#[test]
fn test_arazzo_step_parameters() {
    let path = Path::new("tests/fixtures/arazzo.yaml");
    let spec = load_arazzo(path).unwrap();

    let workflow = &spec.workflows[0];
    let get_profile_step = &workflow.steps[2];

    // Check that the getProfile step has Authorization header
    assert_eq!(get_profile_step.parameters.len(), 1);
    assert_eq!(get_profile_step.parameters[0].name, "Authorization");
    assert_eq!(get_profile_step.parameters[0].location, "header");
}

#[test]
fn test_arazzo_step_outputs() {
    let path = Path::new("tests/fixtures/arazzo.yaml");
    let spec = load_arazzo(path).unwrap();

    let workflow = &spec.workflows[0];
    let login_step = &workflow.steps[1];

    // Check that login step has outputs defined
    assert!(login_step.outputs.is_some());
}

#[test]
fn test_arazzo_success_criteria() {
    let path = Path::new("tests/fixtures/arazzo.yaml");
    let spec = load_arazzo(path).unwrap();

    let workflow = &spec.workflows[0];
    let register_step = &workflow.steps[0];

    // Check success criteria
    assert!(register_step.success_criteria.is_some());
    let criteria = register_step.success_criteria.as_ref().unwrap();
    assert_eq!(criteria.len(), 1);
    assert_eq!(criteria[0].context, "$statusCode");
    assert_eq!(criteria[0].condition, "==");
}

#[test]
fn test_second_workflow() {
    let path = Path::new("tests/fixtures/arazzo.yaml");
    let spec = load_arazzo(path).unwrap();

    let second_workflow = &spec.workflows[1];
    assert_eq!(second_workflow.workflow_id, "simple-login-flow");
    assert_eq!(second_workflow.steps.len(), 1);
    assert_eq!(second_workflow.steps[0].step_id, "login");
}
