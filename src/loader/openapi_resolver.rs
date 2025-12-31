use crate::error::Result;
use oas3::OpenApiV3Spec;
use oas3::spec::Operation;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// OpenAPI解決器
#[derive(Debug, Clone)]
pub struct OpenApiResolver {
    #[allow(dead_code)]
    project_dir: PathBuf,
    specs: HashMap<String, OpenApiV3Spec>,
}

impl OpenApiResolver {
    pub fn new(project_dir: impl Into<PathBuf>) -> Self {
        Self {
            project_dir: project_dir.into(),
            specs: HashMap::new(),
        }
    }

    /// 複数のOpenAPIファイルをロード
    pub fn load_specs(&mut self, paths: &[PathBuf]) -> Result<()> {
        for path in paths {
            let spec = crate::loader::load_openapi(path)?;
            let name = self.extract_name(path);
            self.specs.insert(name, spec);
        }
        Ok(())
    }

    /// Load a single OpenAPI spec with an explicit name
    pub fn load_spec(&mut self, name: &str, path: &Path) -> Result<()> {
        let spec = crate::loader::load_openapi(path)?;
        self.specs.insert(name.to_string(), spec);
        Ok(())
    }

    /// ファイルパスから名前を抽出
    fn extract_name(&self, path: &Path) -> String {
        path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("default")
            .to_string()
    }

    /// 名前でOpenAPI仕様を取得
    pub fn get_spec(&self, name: &str) -> Option<&OpenApiV3Spec> {
        self.specs.get(name)
    }

    /// すべてのOpenAPI仕様を取得
    pub fn get_all_specs(&self) -> &HashMap<String, OpenApiV3Spec> {
        &self.specs
    }

    /// operationIdで操作を検索（すべてのOpenAPIから）
    pub fn find_operation(&self, operation_id: &str) -> Option<OperationRef> {
        for (source_name, spec) in &self.specs {
            if let Some(paths) = &spec.paths {
                for (path, path_item) in paths.iter() {
                    // 各HTTPメソッドをチェック
                    let operations = [
                        ("GET", &path_item.get),
                        ("POST", &path_item.post),
                        ("PUT", &path_item.put),
                        ("DELETE", &path_item.delete),
                        ("PATCH", &path_item.patch),
                        ("OPTIONS", &path_item.options),
                        ("HEAD", &path_item.head),
                        ("TRACE", &path_item.trace),
                    ];

                    for (method, op_option) in &operations {
                        if let Some(op) = op_option
                            && op.operation_id.as_deref() == Some(operation_id)
                        {
                            return Some(OperationRef {
                                source_name: source_name.clone(),
                                method: method.to_string(),
                                path: path.clone(),
                            });
                        }
                    }
                }
            }
        }
        None
    }

    /// operationIdで操作を検索（完全な情報を含む）
    pub fn find_operation_with_details(
        &self,
        operation_id: &str,
    ) -> Option<(OperationRef, Operation)> {
        for (source_name, spec) in &self.specs {
            if let Some(paths) = &spec.paths {
                for (path, path_item) in paths.iter() {
                    // 各HTTPメソッドをチェック
                    let operations = [
                        ("GET", &path_item.get),
                        ("POST", &path_item.post),
                        ("PUT", &path_item.put),
                        ("DELETE", &path_item.delete),
                        ("PATCH", &path_item.patch),
                        ("OPTIONS", &path_item.options),
                        ("HEAD", &path_item.head),
                        ("TRACE", &path_item.trace),
                    ];

                    for (method, op_option) in &operations {
                        if let Some(op) = op_option
                            && op.operation_id.as_deref() == Some(operation_id)
                        {
                            let op_ref = OperationRef {
                                source_name: source_name.clone(),
                                method: method.to_string(),
                                path: path.clone(),
                            };
                            return Some((op_ref, (*op).clone()));
                        }
                    }
                }
            }
        }
        None
    }

    /// operationPathとmethodで操作を検索（完全な情報を含む）
    pub fn find_operation_by_path_with_details(
        &self,
        operation_path: &str,
        method: &str,
    ) -> Option<(OperationRef, Operation)> {
        let method_upper = method.to_uppercase();

        for (source_name, spec) in &self.specs {
            if let Some(paths) = &spec.paths
                && let Some(path_item) = paths.get(operation_path)
            {
                let op_option = match method_upper.as_str() {
                    "GET" => &path_item.get,
                    "POST" => &path_item.post,
                    "PUT" => &path_item.put,
                    "DELETE" => &path_item.delete,
                    "PATCH" => &path_item.patch,
                    "OPTIONS" => &path_item.options,
                    "HEAD" => &path_item.head,
                    "TRACE" => &path_item.trace,
                    _ => &None,
                };

                if let Some(op) = op_option {
                    let op_ref = OperationRef {
                        source_name: source_name.clone(),
                        method: method_upper,
                        path: operation_path.to_string(),
                    };
                    return Some((op_ref, op.clone()));
                }
            }
        }
        None
    }

    /// ファイル一覧を取得
    pub fn list_files(&self) -> Vec<String> {
        self.specs.keys().cloned().collect()
    }
}

/// 操作への参照
#[derive(Debug, Clone)]
pub struct OperationRef {
    pub source_name: String,
    pub method: String,
    pub path: String,
}
