use hornet2::loader::ProjectScanner;
use std::path::Path;

#[test]
fn test_scan_multi_project_fixtures() {
    let root = Path::new("tests/fixtures/multi_project");

    let scanner = ProjectScanner::new(root);
    let projects = scanner.scan_projects().unwrap();

    // 2つのプロジェクトが検出されること
    assert_eq!(projects.len(), 2);

    // プロジェクト名が正しいこと
    let project_names: Vec<String> = projects.iter().map(|p| p.name.clone()).collect();
    assert!(project_names.contains(&"project1".to_string()));
    assert!(project_names.contains(&"project2".to_string()));

    // project1の検証
    let project1 = projects.iter().find(|p| p.name == "project1").unwrap();
    assert_eq!(project1.arazzo_spec.info.title, "Project 1 API Testing");
    assert_eq!(project1.arazzo_spec.workflows.len(), 1);
    assert_eq!(project1.arazzo_spec.workflows[0].workflow_id, "get-pet-workflow");
    assert_eq!(project1.openapi_paths.len(), 1);

    // project2の検証
    let project2 = projects.iter().find(|p| p.name == "project2").unwrap();
    assert_eq!(project2.arazzo_spec.info.title, "Project 2 User API Testing");
    assert_eq!(project2.arazzo_spec.workflows.len(), 2);
    assert_eq!(project2.openapi_paths.len(), 1);
}

#[test]
fn test_project_metadata_openapi_detection() {
    let root = Path::new("tests/fixtures/multi_project");

    let scanner = ProjectScanner::new(root);
    let projects = scanner.scan_projects().unwrap();

    for project in projects {
        // 各プロジェクトにarazzo.yamlが存在すること
        assert!(project.arazzo_path.exists());
        assert!(project.arazzo_path.ends_with("arazzo.yaml"));

        // 各プロジェクトに少なくとも1つのOpenAPIファイルがあること
        assert!(!project.openapi_paths.is_empty());

        for openapi_path in &project.openapi_paths {
            assert!(openapi_path.exists());
            assert!(openapi_path.file_name().unwrap().to_str().unwrap().contains("openapi"));
        }
    }
}
