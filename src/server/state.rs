use crate::error::{HornetError, Result};
use crate::loader::{OpenApiResolver, ProjectMetadata, ProjectScanner};
use crate::models::arazzo::ArazzoSpec;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

/// アプリケーション全体の状態
#[derive(Clone)]
pub struct AppState {
    /// プロジェクトキャッシュ（読み取り多め、更新少なめ）
    pub projects: Arc<RwLock<ProjectCache>>,

    /// ルートディレクトリパス
    pub root_dir: PathBuf,
}

impl AppState {
    pub fn new(root_dir: PathBuf) -> Result<Self> {
        let cache = ProjectCache::new(Duration::from_secs(60));

        Ok(Self {
            projects: Arc::new(RwLock::new(cache)),
            root_dir,
        })
    }
}

/// プロジェクトのキャッシュ
pub struct ProjectCache {
    /// プロジェクト名 -> プロジェクトデータ
    projects: HashMap<String, ProjectData>,

    /// 最終スキャン時刻
    last_scan: Option<Instant>,

    /// キャッシュ有効期限
    cache_ttl: Duration,
}

impl ProjectCache {
    pub fn new(cache_ttl: Duration) -> Self {
        Self {
            projects: HashMap::new(),
            last_scan: None,
            cache_ttl,
        }
    }

    /// プロジェクトを取得（キャッシュから or ファイルシステムから再読み込み）
    pub fn get_project(&mut self, name: &str, root_dir: &Path) -> Result<&ProjectData> {
        // キャッシュが期限切れかチェック
        if self.is_expired() {
            self.refresh(root_dir)?;
        }

        // プロジェクトを取得
        self.projects
            .get(name)
            .ok_or_else(|| HornetError::ProjectNotFound(name.to_string()))
    }

    /// すべてのプロジェクトをリスト
    pub fn list_projects(&mut self, root_dir: &Path) -> Result<Vec<&ProjectData>> {
        if self.is_expired() {
            self.refresh(root_dir)?;
        }
        Ok(self.projects.values().collect())
    }

    /// キャッシュをリフレッシュ
    fn refresh(&mut self, root_dir: &Path) -> Result<()> {
        let scanner = ProjectScanner::new(root_dir);
        let projects = scanner.scan_projects()?;

        self.projects.clear();
        for meta in projects {
            let project_data = ProjectData::from_metadata(meta)?;
            self.projects
                .insert(project_data.name.clone(), project_data);
        }

        self.last_scan = Some(Instant::now());
        Ok(())
    }

    /// プロジェクトを個別に再読み込み（ワークフロー作成/更新後）
    pub fn reload_project(&mut self, name: &str, root_dir: &Path) -> Result<()> {
        let scanner = ProjectScanner::new(root_dir);

        // Check if this is single-project mode (root dir contains arazzo.yaml)
        let root_arazzo = root_dir.join("arazzo.yaml");
        let project_dir = if root_arazzo.exists() {
            // Single-project mode: use root_dir if the project name matches
            let root_name = root_dir
                .canonicalize()
                .ok()
                .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
                .unwrap_or_else(|| "default".to_string());

            if name == root_name {
                root_dir.to_path_buf()
            } else {
                root_dir.join(name)
            }
        } else {
            // Multi-project mode: project is in subdirectory
            root_dir.join(name)
        };

        let meta = scanner.load_project_metadata(&project_dir)?;
        let project_data = ProjectData::from_metadata(meta)?;

        self.projects.insert(name.to_string(), project_data);
        Ok(())
    }

    fn is_expired(&self) -> bool {
        match self.last_scan {
            None => true,
            Some(last) => last.elapsed() > self.cache_ttl,
        }
    }
}

/// 単一プロジェクトのデータ
#[derive(Clone)]
pub struct ProjectData {
    pub name: String,
    pub arazzo_path: PathBuf,
    pub arazzo_spec: ArazzoSpec,
    pub openapi_resolver: OpenApiResolver,
}

impl ProjectData {
    pub fn from_metadata(meta: ProjectMetadata) -> Result<Self> {
        let mut resolver = OpenApiResolver::new(&meta.directory);

        // Load specs with names from sourceDescriptions
        for path in &meta.openapi_paths {
            let name = meta
                .source_name_map
                .get(path)
                .map(|s| s.as_str())
                .unwrap_or_else(|| {
                    // Fallback to file stem if not in map (shouldn't happen)
                    path.file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("openapi")
                });

            resolver.load_spec(name, path)?;
        }

        Ok(Self {
            name: meta.name,
            arazzo_path: meta.arazzo_path,
            arazzo_spec: meta.arazzo_spec,
            openapi_resolver: resolver,
        })
    }
}
