# eBPF OpenAPI Tracing - 最終設計書

## 1. エグゼクティブサマリー

本設計書は、hornet2にeBPFベースのHTTPトレーシング機能を追加し、以下を実現する最終設計を定義する：

1. **OpenTelemetry eBPF Instrumentation (OBI)** を活用したゼロコード・トレーシング
2. トレースデータからの**OpenAPI仕様の自動推論・補完**
3. API呼び出しシーケンスからの**Arazzoワークフロー自動生成**
4. 依存APIの**スタブ/モック自動生成**（WireMock, Prism形式対応）

---

## 2. アーキテクチャ概要

### 2.1 システム全体図

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              hornet2 System                                  │
│                                                                              │
│  ┌────────────────────────────────────────────────────────────────────────┐ │
│  │                        Target Environment                               │ │
│  │                                                                         │ │
│  │    ┌──────────────┐         ┌──────────────┐         ┌──────────────┐  │ │
│  │    │  Service A   │────────▶│  Service B   │────────▶│ External API │  │ │
│  │    │  (Target)    │         │  (Dependency)│         │ (3rd Party)  │  │ │
│  │    └──────────────┘         └──────────────┘         └──────────────┘  │ │
│  │           │                        │                        │           │ │
│  │           └────────────────────────┴────────────────────────┘           │ │
│  │                                    │                                    │ │
│  │                          eBPF Probes (OBI)                              │ │
│  └────────────────────────────────────┬───────────────────────────────────┘ │
│                                       │                                      │
│                                       ▼ OTLP                                 │
│  ┌────────────────────────────────────────────────────────────────────────┐ │
│  │                         hornet2 Core                                    │ │
│  │                                                                         │ │
│  │   ┌─────────────┐    ┌─────────────┐    ┌───────────────────────────┐  │ │
│  │   │    OTLP     │    │    Span     │    │    Inference Engine       │  │ │
│  │   │  Receiver   │───▶│   Store     │───▶│                           │  │ │
│  │   │ (4317/4318) │    │  (SQLite)   │    │  ┌─────────────────────┐  │  │ │
│  │   └─────────────┘    └─────────────┘    │  │ Path Analyzer       │  │  │ │
│  │                                          │  ├─────────────────────┤  │  │ │
│  │                                          │  │ Schema Inferrer     │  │  │ │
│  │                                          │  ├─────────────────────┤  │  │ │
│  │                                          │  │ Workflow Detector   │  │  │ │
│  │                                          │  ├─────────────────────┤  │  │ │
│  │                                          │  │ Dependency Analyzer │  │  │ │
│  │                                          │  └─────────────────────┘  │  │ │
│  │                                          └───────────────────────────┘  │ │
│  │                                                       │                  │ │
│  │                    ┌──────────────────────────────────┼──────────────┐  │ │
│  │                    │                                  │              │  │ │
│  │                    ▼                                  ▼              ▼  │ │
│  │   ┌────────────────────┐  ┌────────────────────┐  ┌───────────────┐   │ │
│  │   │     OpenAPI        │  │      Arazzo        │  │    Stubs      │   │ │
│  │   │    Generator       │  │    Generator       │  │  Generator    │   │ │
│  │   │                    │  │                    │  │               │   │ │
│  │   │  • Paths           │  │  • Workflows       │  │  • WireMock   │   │ │
│  │   │  • Schemas         │  │  • Steps           │  │  • Prism      │   │ │
│  │   │  • Parameters      │  │  • Dependencies    │  │  • OpenAPI    │   │ │
│  │   │  • Responses       │  │  • Criteria        │  │    Examples   │   │ │
│  │   └────────────────────┘  └────────────────────┘  └───────────────┘   │ │
│  │                                                                         │ │
│  └─────────────────────────────────────────────────────────────────────────┘ │
│                                                                              │
│  ┌─────────────────────────────────────────────────────────────────────────┐│
│  │                           Web UI                                         ││
│  │   ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐ ││
│  │   │  Trace   │  │ OpenAPI  │  │ Workflow │  │  Stub    │  │  Test    │ ││
│  │   │  Viewer  │  │  Editor  │  │  Editor  │  │ Manager  │  │  Runner  │ ││
│  │   └──────────┘  └──────────┘  └──────────┘  └──────────┘  └──────────┘ ││
│  └─────────────────────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────────────────┘
```

### 2.2 コンポーネント責務

| コンポーネント | 責務 |
|---------------|------|
| **OBI (外部)** | eBPFによるHTTP/gRPCトレース、OTLP出力 |
| **OTLP Receiver** | OBI/Collectorからのトレースデータ受信 |
| **Span Store** | トレースデータの永続化、クエリ |
| **Inference Engine** | パス推論、スキーマ推論、依存関係解析 |
| **OpenAPI Generator** | OpenAPI 3.x仕様の生成・マージ |
| **Arazzo Generator** | Arazzo 1.0.0ワークフローの生成 |
| **Stub Generator** | 依存APIのモック/スタブ生成 |

---

## 3. データモデル

### 3.1 トレースデータ構造

```rust
// src/tracer/types.rs

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// トレースセッション
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceSession {
    pub id: String,
    pub name: Option<String>,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub config: TraceConfig,
    pub statistics: TraceStatistics,
}

/// トレース設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceConfig {
    /// 対象サービス名（OBIのservice.name）
    pub target_services: Vec<String>,
    /// 対象ポート
    pub target_ports: Vec<u16>,
    /// 外部依存として扱うホストパターン
    pub external_hosts: Vec<String>,
    /// ボディキャプチャの有効化
    pub capture_bodies: bool,
    /// 最大ボディサイズ
    pub max_body_size: usize,
}

/// トレース統計
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceStatistics {
    pub total_spans: u64,
    pub incoming_requests: u64,
    pub outgoing_requests: u64,
    pub unique_endpoints: u64,
    pub unique_dependencies: u64,
}

/// 正規化されたHTTPスパン
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpSpan {
    // === 識別子 ===
    pub trace_id: String,
    pub span_id: String,
    pub parent_span_id: Option<String>,

    // === 時間 ===
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub duration_ms: f64,

    // === サービス情報 ===
    pub service_name: String,
    pub service_version: Option<String>,

    // === HTTP情報（Semantic Conventions準拠） ===
    pub direction: SpanDirection,
    pub method: String,
    pub path: String,
    pub route: Option<String>,  // http.route (テンプレート化されたパス)
    pub query: Option<String>,
    pub scheme: String,
    pub status_code: u16,

    // === ホスト情報 ===
    pub server_address: String,
    pub server_port: u16,
    pub client_address: Option<String>,

    // === ボディ情報 ===
    pub request_body: Option<CapturedBody>,
    pub response_body: Option<CapturedBody>,

    // === ヘッダー情報 ===
    pub request_headers: HashMap<String, String>,
    pub response_headers: HashMap<String, String>,

    // === 追加属性 ===
    pub attributes: HashMap<String, serde_json::Value>,
}

/// スパンの方向
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SpanDirection {
    /// 受信リクエスト（サーバー側）
    Incoming,
    /// 送信リクエスト（クライアント側、依存API呼び出し）
    Outgoing,
}

/// キャプチャされたボディ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapturedBody {
    pub content_type: Option<String>,
    pub size: usize,
    pub data: BodyData,
}

/// ボディデータ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BodyData {
    /// JSON（パース済み）
    Json(serde_json::Value),
    /// テキスト
    Text(String),
    /// バイナリ（Base64）
    Binary(String),
    /// サイズ超過でキャプチャされず
    TooLarge,
    /// キャプチャ無効
    NotCaptured,
}
```

### 3.2 推論結果データ構造

```rust
// src/inference/types.rs

/// 推論されたエンドポイント
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferredEndpoint {
    /// HTTPメソッド
    pub method: String,
    /// パステンプレート（例: /users/{userId}）
    pub path_template: String,
    /// 推論されたoperationId
    pub operation_id: String,
    /// 観測回数
    pub observation_count: usize,
    /// パスパラメータ
    pub path_parameters: Vec<InferredParameter>,
    /// クエリパラメータ
    pub query_parameters: Vec<InferredParameter>,
    /// リクエストボディスキーマ
    pub request_body: Option<InferredRequestBody>,
    /// レスポンス（ステータスコード別）
    pub responses: HashMap<u16, InferredResponse>,
    /// サンプルスパン
    pub sample_spans: Vec<String>,  // span_ids
}

/// 推論されたパラメータ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferredParameter {
    pub name: String,
    pub location: ParameterLocation,
    pub required: bool,
    pub schema: InferredSchema,
    pub examples: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ParameterLocation {
    Path,
    Query,
    Header,
}

/// 推論されたスキーマ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferredSchema {
    pub schema_type: SchemaType,
    pub format: Option<String>,
    pub nullable: bool,
    pub properties: Option<HashMap<String, Box<InferredSchema>>>,
    pub items: Option<Box<InferredSchema>>,
    pub required_properties: Vec<String>,
    pub enum_values: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SchemaType {
    String,
    Integer,
    Number,
    Boolean,
    Array,
    Object,
    Null,
    OneOf(Vec<SchemaType>),
}

/// 推論されたリクエストボディ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferredRequestBody {
    pub content_type: String,
    pub required: bool,
    pub schema: InferredSchema,
    pub examples: Vec<serde_json::Value>,
}

/// 推論されたレスポンス
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferredResponse {
    pub status_code: u16,
    pub content_type: Option<String>,
    pub schema: Option<InferredSchema>,
    pub headers: Vec<InferredParameter>,
    pub examples: Vec<serde_json::Value>,
}
```

### 3.3 依存関係データ構造

```rust
// src/inference/dependency.rs

/// 検出された依存API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedDependency {
    /// 依存先ホスト
    pub host: String,
    /// 依存先ポート
    pub port: u16,
    /// 呼び出し元サービス
    pub caller_service: String,
    /// エンドポイント一覧
    pub endpoints: Vec<InferredEndpoint>,
    /// 呼び出しパターン（どのような順序で呼ばれるか）
    pub call_patterns: Vec<CallPattern>,
}

/// 呼び出しパターン
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallPattern {
    /// 呼び出し元エンドポイント
    pub caller_endpoint: EndpointRef,
    /// 呼び出し先エンドポイント
    pub callee_endpoint: EndpointRef,
    /// データ依存関係
    pub data_mappings: Vec<DataMapping>,
    /// 観測回数
    pub observation_count: usize,
}

/// エンドポイント参照
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointRef {
    pub method: String,
    pub path_template: String,
}

/// データマッピング（あるレスポンスフィールドが次のリクエストで使われる）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataMapping {
    /// ソース（レスポンス内のパス）
    pub source_path: String,
    /// ターゲット（リクエスト内のパス）
    pub target_path: String,
    /// マッピングの種類
    pub mapping_type: MappingType,
    /// 信頼度スコア (0.0-1.0)
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MappingType {
    /// レスポンスボディ → リクエストボディ
    BodyToBody,
    /// レスポンスボディ → パスパラメータ
    BodyToPath,
    /// レスポンスボディ → クエリパラメータ
    BodyToQuery,
    /// レスポンスボディ → ヘッダー
    BodyToHeader,
    /// レスポンスヘッダー → リクエストヘッダー
    HeaderToHeader,
}
```

---

## 4. スタブ生成機能

### 4.1 概要

トレースから検出された依存API（Outgoing呼び出し）のモック/スタブを自動生成し、テスト時に実際の外部APIなしでテストを実行可能にする。

### 4.2 サポートするスタブ形式

| 形式 | 用途 | 特徴 |
|------|------|------|
| **WireMock** | Java/エンタープライズ環境 | 柔軟なマッチング、状態管理 |
| **Prism** | OpenAPIファースト開発 | OpenAPIネイティブ、軽量 |
| **hornet2 Native** | hornet2エコシステム | Arazzo連携、動的レスポンス |

### 4.3 スタブデータ構造

```rust
// src/stubs/types.rs

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// スタブ定義
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StubDefinition {
    /// スタブID
    pub id: String,
    /// 対象依存API
    pub target_host: String,
    pub target_port: u16,
    /// エンドポイント別スタブ
    pub endpoints: Vec<EndpointStub>,
    /// メタデータ
    pub metadata: StubMetadata,
}

/// スタブメタデータ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StubMetadata {
    pub generated_at: chrono::DateTime<chrono::Utc>,
    pub source_trace_session: String,
    pub observation_count: usize,
    pub generator_version: String,
}

/// エンドポイントスタブ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointStub {
    /// HTTPメソッド
    pub method: String,
    /// パスパターン（正規表現またはテンプレート）
    pub path_pattern: PathPattern,
    /// リクエストマッチャー
    pub request_matcher: RequestMatcher,
    /// レスポンス定義
    pub responses: Vec<StubResponse>,
    /// デフォルトレスポンス
    pub default_response: StubResponse,
}

/// パスパターン
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PathPattern {
    /// 完全一致
    Exact(String),
    /// テンプレート（例: /users/{id}）
    Template(String),
    /// 正規表現
    Regex(String),
}

/// リクエストマッチャー
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestMatcher {
    /// ヘッダーマッチング
    pub headers: HashMap<String, HeaderMatcher>,
    /// クエリパラメータマッチング
    pub query_params: HashMap<String, ValueMatcher>,
    /// ボディマッチング
    pub body: Option<BodyMatcher>,
}

/// ヘッダーマッチャー
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HeaderMatcher {
    Exact(String),
    Contains(String),
    Regex(String),
    Present,
    Absent,
}

/// 値マッチャー
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValueMatcher {
    Exact(serde_json::Value),
    Contains(String),
    Regex(String),
    AnyOf(Vec<serde_json::Value>),
    Any,
}

/// ボディマッチャー
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BodyMatcher {
    /// JSON完全一致
    JsonExact(serde_json::Value),
    /// JSONパス存在確認
    JsonPathExists(Vec<String>),
    /// JSONパス値マッチング
    JsonPathMatches(HashMap<String, ValueMatcher>),
    /// 正規表現
    Regex(String),
    /// 任意
    Any,
}

/// スタブレスポンス
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StubResponse {
    /// このレスポンスを返す条件
    pub condition: Option<ResponseCondition>,
    /// ステータスコード
    pub status_code: u16,
    /// レスポンスヘッダー
    pub headers: HashMap<String, String>,
    /// レスポンスボディ
    pub body: ResponseBody,
    /// 遅延（ミリ秒）
    pub delay_ms: Option<u64>,
}

/// レスポンス条件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResponseCondition {
    /// リクエストボディのJSONパスマッチング
    JsonPathMatch { path: String, value: serde_json::Value },
    /// ヘッダーマッチング
    HeaderMatch { name: String, value: String },
    /// 呼び出し回数
    CallCount { min: Option<u32>, max: Option<u32> },
}

/// レスポンスボディ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResponseBody {
    /// 固定JSON
    Json(serde_json::Value),
    /// テンプレート（リクエストからの値参照）
    Template(String),
    /// 動的生成（Faker等）
    Dynamic(DynamicBodyConfig),
    /// 空
    Empty,
}

/// 動的ボディ設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicBodyConfig {
    /// スキーマ（OpenAPI形式）
    pub schema: serde_json::Value,
    /// 固定フィールド
    pub fixed_fields: HashMap<String, serde_json::Value>,
    /// リクエストからコピーするフィールド
    pub copy_from_request: Vec<FieldCopy>,
}

/// フィールドコピー設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldCopy {
    pub source_path: String,
    pub target_path: String,
}
```

### 4.4 WireMock形式エクスポート

```rust
// src/stubs/exporters/wiremock.rs

use serde::Serialize;

/// WireMockマッピング形式
#[derive(Debug, Serialize)]
pub struct WireMockMapping {
    pub id: String,
    pub name: Option<String>,
    pub request: WireMockRequest,
    pub response: WireMockResponse,
    pub priority: Option<i32>,
    #[serde(rename = "scenarioName")]
    pub scenario_name: Option<String>,
    #[serde(rename = "requiredScenarioState")]
    pub required_scenario_state: Option<String>,
    #[serde(rename = "newScenarioState")]
    pub new_scenario_state: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct WireMockRequest {
    pub method: String,
    #[serde(rename = "urlPathPattern")]
    pub url_path_pattern: Option<String>,
    #[serde(rename = "urlPath")]
    pub url_path: Option<String>,
    pub headers: Option<HashMap<String, WireMockMatcher>>,
    #[serde(rename = "queryParameters")]
    pub query_parameters: Option<HashMap<String, WireMockMatcher>>,
    #[serde(rename = "bodyPatterns")]
    pub body_patterns: Option<Vec<WireMockBodyPattern>>,
}

#[derive(Debug, Serialize)]
pub struct WireMockMatcher {
    #[serde(rename = "equalTo")]
    pub equal_to: Option<String>,
    pub contains: Option<String>,
    pub matches: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct WireMockBodyPattern {
    #[serde(rename = "equalToJson")]
    pub equal_to_json: Option<serde_json::Value>,
    #[serde(rename = "matchesJsonPath")]
    pub matches_json_path: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct WireMockResponse {
    pub status: u16,
    pub headers: Option<HashMap<String, String>>,
    #[serde(rename = "jsonBody")]
    pub json_body: Option<serde_json::Value>,
    pub body: Option<String>,
    #[serde(rename = "fixedDelayMilliseconds")]
    pub fixed_delay_milliseconds: Option<u64>,
}

/// WireMockエクスポーター
pub struct WireMockExporter;

impl WireMockExporter {
    /// StubDefinitionをWireMock形式に変換
    pub fn export(stub: &StubDefinition) -> Vec<WireMockMapping> {
        stub.endpoints.iter().flat_map(|endpoint| {
            Self::endpoint_to_mappings(endpoint)
        }).collect()
    }

    fn endpoint_to_mappings(endpoint: &EndpointStub) -> Vec<WireMockMapping> {
        // 各レスポンス条件に対してマッピングを生成
        let mut mappings: Vec<WireMockMapping> = endpoint.responses.iter()
            .enumerate()
            .map(|(i, resp)| {
                WireMockMapping {
                    id: uuid::Uuid::new_v4().to_string(),
                    name: Some(format!("{} {} - Response {}",
                        endpoint.method,
                        Self::path_pattern_to_string(&endpoint.path_pattern),
                        i + 1
                    )),
                    request: Self::convert_request(endpoint, resp.condition.as_ref()),
                    response: Self::convert_response(resp),
                    priority: Some((i + 1) as i32),
                    scenario_name: None,
                    required_scenario_state: None,
                    new_scenario_state: None,
                }
            })
            .collect();

        // デフォルトレスポンス（最低優先度）
        mappings.push(WireMockMapping {
            id: uuid::Uuid::new_v4().to_string(),
            name: Some(format!("{} {} - Default",
                endpoint.method,
                Self::path_pattern_to_string(&endpoint.path_pattern)
            )),
            request: Self::convert_request(endpoint, None),
            response: Self::convert_response(&endpoint.default_response),
            priority: Some(100),
            scenario_name: None,
            required_scenario_state: None,
            new_scenario_state: None,
        });

        mappings
    }

    fn path_pattern_to_string(pattern: &PathPattern) -> String {
        match pattern {
            PathPattern::Exact(s) => s.clone(),
            PathPattern::Template(s) => s.replace("{", "(?<").replace("}", ">[^/]+)"),
            PathPattern::Regex(s) => s.clone(),
        }
    }

    fn convert_request(endpoint: &EndpointStub, _condition: Option<&ResponseCondition>) -> WireMockRequest {
        WireMockRequest {
            method: endpoint.method.clone(),
            url_path_pattern: match &endpoint.path_pattern {
                PathPattern::Exact(s) => None,
                PathPattern::Template(s) | PathPattern::Regex(s) => {
                    Some(Self::path_pattern_to_string(&endpoint.path_pattern))
                }
            },
            url_path: match &endpoint.path_pattern {
                PathPattern::Exact(s) => Some(s.clone()),
                _ => None,
            },
            headers: None,
            query_parameters: None,
            body_patterns: None,
        }
    }

    fn convert_response(resp: &StubResponse) -> WireMockResponse {
        WireMockResponse {
            status: resp.status_code,
            headers: Some(resp.headers.clone()),
            json_body: match &resp.body {
                ResponseBody::Json(v) => Some(v.clone()),
                _ => None,
            },
            body: match &resp.body {
                ResponseBody::Template(t) => Some(t.clone()),
                _ => None,
            },
            fixed_delay_milliseconds: resp.delay_ms,
        }
    }
}
```

### 4.5 Prism/OpenAPI Examples形式エクスポート

```rust
// src/stubs/exporters/prism.rs

/// OpenAPI Examples形式でエクスポート（Prism互換）
pub struct PrismExporter;

impl PrismExporter {
    /// StubDefinitionをOpenAPI形式（examples付き）に変換
    pub fn export(stub: &StubDefinition) -> oas3::OpenApi {
        let mut paths = oas3::spec::PathMap::new();

        for endpoint in &stub.endpoints {
            let path_item = Self::endpoint_to_path_item(endpoint);
            let path = Self::normalize_path(&endpoint.path_pattern);
            paths.insert(path, path_item);
        }

        oas3::OpenApi {
            openapi: "3.0.3".to_string(),
            info: oas3::spec::Info {
                title: format!("Stub API for {}", stub.target_host),
                version: "1.0.0".to_string(),
                description: Some(format!(
                    "Auto-generated stub from trace session: {}",
                    stub.metadata.source_trace_session
                )),
                ..Default::default()
            },
            servers: Some(vec![oas3::spec::Server {
                url: format!("http://{}:{}", stub.target_host, stub.target_port),
                description: Some("Stub server".to_string()),
                ..Default::default()
            }]),
            paths: Some(paths),
            ..Default::default()
        }
    }

    fn endpoint_to_path_item(endpoint: &EndpointStub) -> oas3::spec::PathItem {
        let operation = oas3::spec::Operation {
            operation_id: Some(Self::generate_operation_id(endpoint)),
            summary: Some(format!("{} {}", endpoint.method,
                Self::normalize_path(&endpoint.path_pattern))),
            responses: Self::build_responses(endpoint),
            ..Default::default()
        };

        let mut path_item = oas3::spec::PathItem::default();
        match endpoint.method.to_uppercase().as_str() {
            "GET" => path_item.get = Some(operation),
            "POST" => path_item.post = Some(operation),
            "PUT" => path_item.put = Some(operation),
            "DELETE" => path_item.delete = Some(operation),
            "PATCH" => path_item.patch = Some(operation),
            _ => {}
        }

        path_item
    }

    fn build_responses(endpoint: &EndpointStub) -> oas3::spec::Responses {
        let mut responses = oas3::spec::Responses::default();

        // 各レスポンスをexamplesとして追加
        for (i, resp) in endpoint.responses.iter().enumerate() {
            let response = oas3::spec::Response {
                description: format!("Response variant {}", i + 1),
                content: Self::build_content(&resp.body, &resp.headers),
                ..Default::default()
            };
            responses.insert(
                resp.status_code.to_string(),
                oas3::spec::ObjectOrReference::Object(response),
            );
        }

        // デフォルトレスポンス
        let default_resp = oas3::spec::Response {
            description: "Default response".to_string(),
            content: Self::build_content(
                &endpoint.default_response.body,
                &endpoint.default_response.headers,
            ),
            ..Default::default()
        };
        responses.insert(
            endpoint.default_response.status_code.to_string(),
            oas3::spec::ObjectOrReference::Object(default_resp),
        );

        responses
    }

    fn build_content(
        body: &ResponseBody,
        _headers: &HashMap<String, String>,
    ) -> Option<oas3::spec::MediaTypeMap> {
        match body {
            ResponseBody::Json(value) => {
                let mut content = oas3::spec::MediaTypeMap::new();
                content.insert(
                    "application/json".to_string(),
                    oas3::spec::MediaType {
                        example: Some(value.clone()),
                        ..Default::default()
                    },
                );
                Some(content)
            }
            ResponseBody::Empty => None,
            _ => None,
        }
    }

    fn normalize_path(pattern: &PathPattern) -> String {
        match pattern {
            PathPattern::Exact(s) => s.clone(),
            PathPattern::Template(s) => s.clone(),
            PathPattern::Regex(s) => s.replace("(?<", "{").replace(">[^/]+)", "}"),
        }
    }

    fn generate_operation_id(endpoint: &EndpointStub) -> String {
        let path = Self::normalize_path(&endpoint.path_pattern);
        let parts: Vec<&str> = path.split('/')
            .filter(|s| !s.is_empty() && !s.starts_with('{'))
            .collect();

        let resource = parts.last().unwrap_or(&"resource");
        let action = match endpoint.method.to_uppercase().as_str() {
            "GET" => "get",
            "POST" => "create",
            "PUT" => "update",
            "DELETE" => "delete",
            "PATCH" => "patch",
            _ => "handle",
        };

        format!("{}{}", action, to_pascal_case(resource))
    }
}

fn to_pascal_case(s: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = true;

    for c in s.chars() {
        if c == '_' || c == '-' {
            capitalize_next = true;
        } else if capitalize_next {
            result.push(c.to_ascii_uppercase());
            capitalize_next = false;
        } else {
            result.push(c);
        }
    }

    result
}
```

### 4.6 hornet2 Native形式

```rust
// src/stubs/exporters/native.rs

/// hornet2ネイティブスタブ形式
/// Arazzoワークフローと連携した動的スタブ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hornet2StubConfig {
    pub version: String,
    pub stubs: Vec<Hornet2Stub>,
    /// ステートフルモックの状態定義
    pub states: Option<Vec<StateDefinition>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hornet2Stub {
    pub id: String,
    pub endpoint: EndpointMatcher,
    pub responses: Vec<ConditionalResponse>,
    pub default_response: ResponseTemplate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointMatcher {
    pub method: String,
    pub path: String,  // OpenAPI形式のパステンプレート
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConditionalResponse {
    pub when: Condition,
    pub then: ResponseTemplate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Condition {
    /// JSONパスの値が一致
    #[serde(rename = "jsonPath")]
    JsonPath { path: String, equals: serde_json::Value },

    /// ヘッダーの値が一致
    #[serde(rename = "header")]
    Header { name: String, equals: String },

    /// 状態が一致（ステートフルモック）
    #[serde(rename = "state")]
    State { name: String, equals: String },

    /// 複数条件のAND
    #[serde(rename = "and")]
    And { conditions: Vec<Condition> },

    /// 複数条件のOR
    #[serde(rename = "or")]
    Or { conditions: Vec<Condition> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseTemplate {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: BodyTemplate,
    /// レスポンス後に状態を変更
    pub set_state: Option<StateTransition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum BodyTemplate {
    /// 固定値
    #[serde(rename = "fixed")]
    Fixed { value: serde_json::Value },

    /// テンプレート（Handlebars形式）
    /// {{request.body.userId}}, {{request.path.id}}, {{state.counter}}
    #[serde(rename = "template")]
    Template { template: String },

    /// スキーマからの動的生成
    #[serde(rename = "fromSchema")]
    FromSchema {
        schema: serde_json::Value,
        overrides: HashMap<String, serde_json::Value>,
    },

    /// Arazzoステップ出力の参照
    #[serde(rename = "arazzoRef")]
    ArazzoRef {
        workflow_id: String,
        step_id: String,
        output_path: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateDefinition {
    pub name: String,
    pub initial: String,
    pub transitions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateTransition {
    pub name: String,
    pub value: String,
}
```

---

## 5. モジュール構造（最終版）

```
src/
├── cli.rs                      # CLIエントリポイント
├── error.rs                    # エラー型定義
├── lib.rs                      # ライブラリルート
│
├── tracer/                     # OTLPトレースレシーバー
│   ├── mod.rs
│   ├── config.rs               # トレーサー設定
│   ├── otlp_receiver.rs        # OTLP gRPC/HTTPサーバー
│   ├── semantic_conventions.rs # OTel Semantic Conventions
│   ├── span_processor.rs       # Span処理パイプライン
│   ├── span_classifier.rs      # Incoming/Outgoing分類
│   ├── types.rs                # データ構造
│   └── store/
│       ├── mod.rs
│       ├── sqlite.rs           # SQLite永続化
│       └── memory.rs           # インメモリストア（テスト用）
│
├── inference/                  # 推論エンジン
│   ├── mod.rs
│   ├── path_analyzer.rs        # パスパラメータ検出
│   ├── schema_inferrer.rs      # JSONスキーマ推論
│   ├── dependency_detector.rs  # 依存API検出
│   ├── data_flow_analyzer.rs   # データフロー解析
│   ├── workflow_detector.rs    # ワークフローパターン検出
│   └── types.rs                # 推論結果データ構造
│
├── generators/                 # 生成器
│   ├── mod.rs
│   ├── openapi/
│   │   ├── mod.rs
│   │   ├── generator.rs        # OpenAPI生成
│   │   └── merger.rs           # 既存specとのマージ
│   ├── arazzo/
│   │   ├── mod.rs
│   │   └── generator.rs        # Arazzo生成
│   └── stubs/
│       ├── mod.rs
│       ├── generator.rs        # スタブ定義生成
│       └── exporters/
│           ├── mod.rs
│           ├── wiremock.rs     # WireMock形式
│           ├── prism.rs        # Prism/OpenAPI Examples形式
│           └── native.rs       # hornet2ネイティブ形式
│
├── commands/                   # CLIコマンド実装
│   ├── mod.rs
│   ├── trace.rs                # trace collect
│   ├── infer.rs                # infer openapi/arazzo
│   ├── stubs.rs                # stubs generate/export
│   └── ... (既存コマンド)
│
├── server/                     # Webサーバー（既存 + 拡張）
│   ├── mod.rs
│   ├── api.rs                  # 既存API
│   ├── api_trace.rs            # トレースAPI
│   ├── api_inference.rs        # 推論API
│   ├── api_stubs.rs            # スタブAPI
│   └── state.rs                # アプリケーション状態
│
└── ... (既存モジュール)
```

---

## 6. CLIコマンド設計

```rust
// src/cli.rs

#[derive(Parser)]
#[command(name = "hornet2")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    // === 既存コマンド ===
    Serve { ... },
    Validate { ... },
    Visualize { ... },
    Convert { ... },
    Run { ... },

    // === 新規: トレースコマンド ===
    /// Collect traces from OBI/OpenTelemetry
    Trace {
        #[command(subcommand)]
        command: TraceCommands,
    },

    // === 新規: 推論コマンド ===
    /// Infer specifications from traces
    Infer {
        #[command(subcommand)]
        command: InferCommands,
    },

    // === 新規: スタブコマンド ===
    /// Generate and manage API stubs
    Stubs {
        #[command(subcommand)]
        command: StubCommands,
    },
}

#[derive(Subcommand)]
pub enum TraceCommands {
    /// Start collecting traces (runs OTLP receiver)
    Collect {
        /// OTLP gRPC port
        #[arg(long, default_value = "4317")]
        grpc_port: u16,

        /// OTLP HTTP port
        #[arg(long, default_value = "4318")]
        http_port: u16,

        /// Session name
        #[arg(short, long)]
        name: Option<String>,

        /// Duration to collect (e.g., "10m", "1h")
        #[arg(short, long)]
        duration: Option<String>,

        /// Target service names to filter
        #[arg(long)]
        service: Vec<String>,

        /// External hosts (treated as dependencies)
        #[arg(long)]
        external: Vec<String>,

        /// Capture request/response bodies
        #[arg(long, default_value = "true")]
        capture_bodies: bool,

        /// Storage path
        #[arg(long, default_value = ".hornet2/traces")]
        store_path: PathBuf,
    },

    /// List trace sessions
    List {
        /// Storage path
        #[arg(long, default_value = ".hornet2/traces")]
        store_path: PathBuf,
    },

    /// Show trace session details
    Show {
        /// Session ID
        session_id: String,

        /// Storage path
        #[arg(long, default_value = ".hornet2/traces")]
        store_path: PathBuf,

        /// Output format
        #[arg(short, long, default_value = "table")]
        format: OutputFormat,
    },

    /// Export trace data
    Export {
        /// Session ID
        session_id: String,

        /// Output file
        #[arg(short, long)]
        output: PathBuf,

        /// Export format (otlp-json, csv, parquet)
        #[arg(short, long, default_value = "otlp-json")]
        format: TraceExportFormat,

        /// Storage path
        #[arg(long, default_value = ".hornet2/traces")]
        store_path: PathBuf,
    },
}

#[derive(Subcommand)]
pub enum InferCommands {
    /// Infer OpenAPI specification from traces
    Openapi {
        /// Trace session ID
        #[arg(long)]
        session: String,

        /// Base OpenAPI spec to merge with
        #[arg(long)]
        base: Option<PathBuf>,

        /// Output file
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Output format (yaml, json)
        #[arg(short, long, default_value = "yaml")]
        format: SpecFormat,

        /// Minimum observations per endpoint
        #[arg(long, default_value = "1")]
        min_observations: usize,

        /// Include dependency APIs
        #[arg(long)]
        include_dependencies: bool,

        /// Storage path
        #[arg(long, default_value = ".hornet2/traces")]
        store_path: PathBuf,
    },

    /// Infer Arazzo workflows from traces
    Arazzo {
        /// Trace session ID
        #[arg(long)]
        session: String,

        /// OpenAPI spec to reference
        #[arg(long)]
        openapi: PathBuf,

        /// Output file
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Output format (yaml, json)
        #[arg(short, long, default_value = "yaml")]
        format: SpecFormat,

        /// Minimum pattern occurrences
        #[arg(long, default_value = "2")]
        min_occurrences: usize,

        /// Storage path
        #[arg(long, default_value = ".hornet2/traces")]
        store_path: PathBuf,
    },
}

#[derive(Subcommand)]
pub enum StubCommands {
    /// Generate stubs from traces
    Generate {
        /// Trace session ID
        #[arg(long)]
        session: String,

        /// Target host to generate stubs for (dependency)
        #[arg(long)]
        target: Option<String>,

        /// Output directory
        #[arg(short, long, default_value = ".hornet2/stubs")]
        output: PathBuf,

        /// Storage path
        #[arg(long, default_value = ".hornet2/traces")]
        store_path: PathBuf,
    },

    /// Export stubs to specific format
    Export {
        /// Stub definition file or directory
        #[arg(long)]
        input: PathBuf,

        /// Output file or directory
        #[arg(short, long)]
        output: PathBuf,

        /// Export format
        #[arg(short, long)]
        format: StubExportFormat,
    },

    /// Run stub server
    Serve {
        /// Stub definition file or directory
        #[arg(long)]
        stubs: PathBuf,

        /// Port to listen on
        #[arg(short, long, default_value = "8080")]
        port: u16,

        /// Enable recording mode (update stubs from real responses)
        #[arg(long)]
        record: bool,

        /// Proxy unmatched requests to real server
        #[arg(long)]
        proxy: Option<String>,
    },

    /// List generated stubs
    List {
        /// Stubs directory
        #[arg(long, default_value = ".hornet2/stubs")]
        path: PathBuf,
    },
}

#[derive(Clone, ValueEnum)]
pub enum StubExportFormat {
    /// WireMock JSON mappings
    Wiremock,
    /// OpenAPI with examples (Prism compatible)
    Prism,
    /// hornet2 native format
    Native,
    /// All formats
    All,
}

#[derive(Clone, ValueEnum)]
pub enum TraceExportFormat {
    /// OTLP JSON
    OtlpJson,
    /// CSV (flattened)
    Csv,
    /// Parquet (columnar)
    Parquet,
}

#[derive(Clone, ValueEnum)]
pub enum SpecFormat {
    Yaml,
    Json,
}
```

---

## 7. API エンドポイント設計

```rust
// src/server/mod.rs (ルート追加)

let app = Router::new()
    // === 既存ルート ===
    .route("/api/projects", get(api::list_projects))
    // ...

    // === トレースAPI ===
    // OTLP Receiver
    .route("/v1/traces", post(api_trace::receive_otlp_http))

    // Session管理
    .route("/api/traces/sessions", get(api_trace::list_sessions))
    .route("/api/traces/sessions", post(api_trace::create_session))
    .route("/api/traces/sessions/:id", get(api_trace::get_session))
    .route("/api/traces/sessions/:id", delete(api_trace::delete_session))
    .route("/api/traces/sessions/:id/spans", get(api_trace::get_spans))
    .route("/api/traces/sessions/:id/spans/stream", get(api_trace::stream_spans))
    .route("/api/traces/sessions/:id/dependencies", get(api_trace::get_dependencies))
    .route("/api/traces/sessions/:id/endpoints", get(api_trace::get_endpoints))

    // === 推論API ===
    .route("/api/infer/openapi", post(api_inference::infer_openapi))
    .route("/api/infer/arazzo", post(api_inference::infer_arazzo))
    .route("/api/infer/preview", post(api_inference::preview_inference))

    // === スタブAPI ===
    .route("/api/stubs", get(api_stubs::list_stubs))
    .route("/api/stubs", post(api_stubs::create_stub))
    .route("/api/stubs/:id", get(api_stubs::get_stub))
    .route("/api/stubs/:id", put(api_stubs::update_stub))
    .route("/api/stubs/:id", delete(api_stubs::delete_stub))
    .route("/api/stubs/:id/export/:format", get(api_stubs::export_stub))
    .route("/api/stubs/generate", post(api_stubs::generate_from_traces))
```

### 7.1 APIリクエスト/レスポンス型

```rust
// src/server/api_trace.rs

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateSessionRequest {
    pub name: Option<String>,
    pub config: TraceConfig,
}

#[derive(Debug, Serialize)]
pub struct SessionResponse {
    pub id: String,
    pub name: Option<String>,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub statistics: TraceStatistics,
}

#[derive(Debug, Serialize)]
pub struct SpansResponse {
    pub spans: Vec<HttpSpan>,
    pub total: u64,
    pub page: u32,
    pub page_size: u32,
}

#[derive(Debug, Serialize)]
pub struct DependenciesResponse {
    pub dependencies: Vec<DependencySummary>,
}

#[derive(Debug, Serialize)]
pub struct DependencySummary {
    pub host: String,
    pub port: u16,
    pub endpoint_count: usize,
    pub total_calls: u64,
}

// src/server/api_inference.rs

#[derive(Debug, Deserialize)]
pub struct InferOpenApiRequest {
    pub session_id: String,
    pub base_spec: Option<serde_json::Value>,
    pub options: InferenceOptions,
}

#[derive(Debug, Deserialize)]
pub struct InferenceOptions {
    pub min_observations: usize,
    pub include_examples: bool,
    pub include_dependencies: bool,
    pub path_parameter_patterns: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct InferOpenApiResponse {
    pub spec: serde_json::Value,
    pub statistics: InferenceStatistics,
}

#[derive(Debug, Serialize)]
pub struct InferenceStatistics {
    pub endpoints_inferred: usize,
    pub schemas_inferred: usize,
    pub parameters_detected: usize,
    pub merged_with_base: bool,
}

#[derive(Debug, Deserialize)]
pub struct InferArazzoRequest {
    pub session_id: String,
    pub openapi_spec: serde_json::Value,
    pub options: ArazzoInferenceOptions,
}

#[derive(Debug, Deserialize)]
pub struct ArazzoInferenceOptions {
    pub min_occurrences: usize,
    pub detect_data_dependencies: bool,
}

// src/server/api_stubs.rs

#[derive(Debug, Deserialize)]
pub struct GenerateStubsRequest {
    pub session_id: String,
    pub target_hosts: Option<Vec<String>>,
    pub options: StubGenerationOptions,
}

#[derive(Debug, Deserialize)]
pub struct StubGenerationOptions {
    pub include_all_responses: bool,
    pub generate_dynamic_responses: bool,
    pub response_delay_ms: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct StubResponse {
    pub id: String,
    pub target_host: String,
    pub target_port: u16,
    pub endpoint_count: usize,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct ExportStubRequest {
    pub format: StubExportFormat,
}
```

---

## 8. ストレージ設計

### 8.1 ディレクトリ構造

```
.hornet2/
├── traces/
│   ├── sessions.db              # セッションメタデータ (SQLite)
│   └── spans/
│       ├── {session_id}/
│       │   ├── spans.db         # Spanデータ (SQLite)
│       │   └── bodies/          # 大きなボディデータ
│       │       ├── {span_id}_req.json
│       │       └── {span_id}_res.json
│       └── ...
│
├── inferred/
│   ├── openapi/
│   │   └── {session_id}.yaml
│   └── arazzo/
│       └── {session_id}.yaml
│
└── stubs/
    ├── definitions/
    │   └── {stub_id}.json       # hornet2ネイティブ形式
    └── exports/
        ├── wiremock/
        │   └── {stub_id}/
        │       └── mappings/
        │           └── *.json
        └── prism/
            └── {stub_id}.yaml   # OpenAPI with examples
```

### 8.2 SQLiteスキーマ

```sql
-- sessions.db

CREATE TABLE sessions (
    id TEXT PRIMARY KEY,
    name TEXT,
    started_at TEXT NOT NULL,
    ended_at TEXT,
    config_json TEXT NOT NULL,
    statistics_json TEXT
);

CREATE INDEX idx_sessions_started_at ON sessions(started_at);

-- spans.db (per session)

CREATE TABLE spans (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    trace_id TEXT NOT NULL,
    span_id TEXT NOT NULL UNIQUE,
    parent_span_id TEXT,

    -- Timing
    start_time TEXT NOT NULL,
    end_time TEXT NOT NULL,
    duration_ms REAL NOT NULL,

    -- Service
    service_name TEXT NOT NULL,
    service_version TEXT,

    -- HTTP
    direction TEXT NOT NULL,  -- 'incoming' or 'outgoing'
    method TEXT NOT NULL,
    path TEXT NOT NULL,
    route TEXT,
    query TEXT,
    scheme TEXT NOT NULL,
    status_code INTEGER NOT NULL,

    -- Host
    server_address TEXT NOT NULL,
    server_port INTEGER NOT NULL,
    client_address TEXT,

    -- Body references (null if inline, path if external)
    request_body_json TEXT,
    request_body_path TEXT,
    response_body_json TEXT,
    response_body_path TEXT,

    -- Headers (JSON)
    request_headers_json TEXT,
    response_headers_json TEXT,

    -- Additional attributes (JSON)
    attributes_json TEXT
);

CREATE INDEX idx_spans_trace_id ON spans(trace_id);
CREATE INDEX idx_spans_direction ON spans(direction);
CREATE INDEX idx_spans_method_path ON spans(method, path);
CREATE INDEX idx_spans_server ON spans(server_address, server_port);
CREATE INDEX idx_spans_start_time ON spans(start_time);
```

---

## 9. OBI連携設定

### 9.1 OBI (Beyla) 設定例

```yaml
# obi-config.yaml

# 基本設定
open_port: 8080              # 監視対象ポート
service_name: my-api         # サービス名

# OTLP出力設定
otel_exporter_otlp_endpoint: http://hornet2:4318
otel_exporter_otlp_protocol: http/protobuf

# トレース設定
trace:
  # リクエスト/レスポンスボディのキャプチャ
  # 注: OBI標準ではボディキャプチャは限定的
  # Collectorのhttp_check receiverと併用推奨

# メトリクス設定
metrics:
  enabled: true

# ログ設定
log:
  level: info
```

### 9.2 OpenTelemetry Collector設定例

```yaml
# otel-collector-config.yaml

receivers:
  otlp:
    protocols:
      grpc:
        endpoint: 0.0.0.0:4317
      http:
        endpoint: 0.0.0.0:4318
        include_metadata: true

processors:
  batch:
    timeout: 1s
    send_batch_size: 1000

  # HTTPボディキャプチャ用プロセッサ（カスタム）
  # 注: 標準Collectorではボディキャプチャは限定的
  # hornet2側でのプロキシモード推奨

  attributes:
    actions:
      - key: hornet2.captured
        value: true
        action: upsert

exporters:
  otlphttp:
    endpoint: http://hornet2:4318

  debug:
    verbosity: detailed

service:
  pipelines:
    traces:
      receivers: [otlp]
      processors: [batch, attributes]
      exporters: [otlphttp, debug]
```

### 9.3 Docker Compose例

```yaml
# docker-compose.yaml

version: '3.8'

services:
  # 対象アプリケーション
  target-app:
    image: my-api:latest
    ports:
      - "8080:8080"
    networks:
      - tracing

  # OBI (eBPFトレーサー)
  obi:
    image: grafana/beyla:latest
    privileged: true
    pid: "host"
    environment:
      BEYLA_OPEN_PORT: "8080"
      BEYLA_SERVICE_NAME: "target-api"
      OTEL_EXPORTER_OTLP_ENDPOINT: "http://collector:4318"
    volumes:
      - /sys:/sys:ro
      - /proc:/proc:ro
    networks:
      - tracing
    depends_on:
      - collector

  # OpenTelemetry Collector
  collector:
    image: otel/opentelemetry-collector-contrib:latest
    command: ["--config=/etc/otel-collector-config.yaml"]
    volumes:
      - ./otel-collector-config.yaml:/etc/otel-collector-config.yaml:ro
    ports:
      - "4317:4317"
      - "4318:4318"
    networks:
      - tracing
    depends_on:
      - hornet2

  # hornet2
  hornet2:
    image: hornet2:latest
    command: ["serve", "--port", "3000", "--otlp-port", "4318"]
    ports:
      - "3000:3000"
      - "4318:4318"
    volumes:
      - hornet2-data:/app/.hornet2
    networks:
      - tracing

networks:
  tracing:
    driver: bridge

volumes:
  hornet2-data:
```

---

## 10. 実装フェーズ

| Phase | 内容 | 成果物 | 期間 |
|-------|------|--------|------|
| **1** | OTLP Receiver実装 | `tracer/otlp_receiver.rs`, `tracer/store/` | 1週間 |
| **2** | Span処理・分類 | `tracer/span_processor.rs`, `span_classifier.rs` | 1週間 |
| **3** | パス・スキーマ推論 | `inference/path_analyzer.rs`, `schema_inferrer.rs` | 2週間 |
| **4** | OpenAPI生成 | `generators/openapi/` | 1週間 |
| **5** | 依存関係検出 | `inference/dependency_detector.rs`, `data_flow_analyzer.rs` | 1週間 |
| **6** | Arazzo生成 | `generators/arazzo/` | 1週間 |
| **7** | スタブ生成 | `generators/stubs/` | 2週間 |
| **8** | CLI実装 | `commands/trace.rs`, `infer.rs`, `stubs.rs` | 1週間 |
| **9** | Web UI | `ui/src/pages/TracePage.tsx`, etc. | 2週間 |
| **10** | 統合テスト・ドキュメント | テスト、README | 1週間 |

**合計: 約13週間**

---

## 11. 依存クレート

```toml
# Cargo.toml

[dependencies]
# 既存依存関係...

# OTLP Protocol
opentelemetry = { version = "0.27", features = ["trace"] }
opentelemetry-proto = { version = "0.27", features = ["gen-tonic"] }
opentelemetry-otlp = { version = "0.27", features = ["http-proto", "grpc-tonic"] }
tonic = { version = "0.12", features = ["transport"] }
prost = "0.13"

# ストレージ
rusqlite = { version = "0.32", features = ["bundled"] }

# 推論
regex = "1.10"
json-patch = "3.0"

# スタブ生成
handlebars = "6.0"
fake = { version = "3.0", features = ["derive"] }

# ユーティリティ
chrono = { version = "0.4", features = ["serde"] }
```

---

## 12. 成功指標

| 指標 | 目標値 |
|------|--------|
| OTLPスループット | 10,000 spans/sec |
| 推論精度（パスパラメータ） | >95% |
| 推論精度（スキーマ型） | >90% |
| スタブ生成時間 | <5秒/100エンドポイント |
| Web UIレスポンス | <200ms (P95) |

---

## 13. リスクと対策

| リスク | 影響 | 対策 |
|--------|------|------|
| OBIがボディキャプチャ非対応 | スキーマ推論精度低下 | Collectorプラグインまたはプロキシモード実装 |
| 大量トレースでメモリ枯渇 | システム不安定 | サンプリング、SQLite永続化、TTL |
| パスパラメータ誤検出 | OpenAPI不正確 | ユーザー確認UI、手動修正機能 |
| HTTPS通信解読不可 | 外部API解析不可 | 証明書設定ガイド、プロキシモード |
