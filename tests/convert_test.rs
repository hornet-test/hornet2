use hornet2::commands::{ConvertCommandArgs, execute_convert};
use std::path::Path;

#[test]
fn convert_returns_error_for_unsupported_target() {
    // 異常系: 未サポートのターゲットを指定した場合に検証エラーになることを確認する
    let args = ConvertCommandArgs {
        arazzo_path: Path::new("tests/fixtures/arazzo.yaml"),
        openapi_path: Path::new("tests/fixtures/openapi.yaml"),
        output_path: None,
        target: "unknown",
        workflow_id: None,
        base_url: None,
        vus: None,
        duration: None,
        iterations: None,
    };

    let result = execute_convert(args);

    assert!(result.is_err());
    let message = format!("{}", result.unwrap_err());
    assert!(
        message.contains("Unsupported target format"),
        "unexpected message: {message}"
    );
}

#[test]
fn convert_returns_error_for_missing_workflow() {
    // 異常系: 存在しないワークフローIDを指定した場合に検証エラーになることを確認する
    let args = ConvertCommandArgs {
        arazzo_path: Path::new("tests/fixtures/arazzo.yaml"),
        openapi_path: Path::new("tests/fixtures/openapi.yaml"),
        output_path: None,
        target: "k6",
        workflow_id: Some("does-not-exist"),
        base_url: None,
        vus: None,
        duration: None,
        iterations: None,
    };

    let result = execute_convert(args);

    assert!(result.is_err());
    let message = format!("{}", result.unwrap_err());
    assert!(
        message.contains("Workflow 'does-not-exist' not found"),
        "unexpected message: {message}"
    );
}
