# #001 OpenAPI / Arazzo YAML パーサーの実装

**Phase**: 1 (MVP - 可視化)
**Priority**: High
**Status**: ✅ Completed (2024-12-03)

## 概要

OpenAPI 3.x と Arazzo Specification の YAML ファイルをパースし、Rust のデータ構造に変換する基盤を構築する。

## 背景

hornet2 の全機能は OpenAPI/Arazzo の定義を読み込むことから始まる。
Phase 1 の可視化機能を実現するには、まず YAML を適切にパースし、内部データ構造に変換する必要がある。

## 実装内容

### 1. 使用するクレートの選定

以下のいずれかを検討：

- **oapiv3**: OpenAPI 3.x パーサー（既存のクレート）
  - Pros: メンテナンスされている、serde ベース
  - Cons: Arazzo は別途実装が必要
- **openapiv3**: 別の OpenAPI パーサー
- **自前実装**: `serde` + `serde_yaml` でスクラッチ実装
  - Pros: Arazzo も同じ設計で実装できる
  - Cons: 実装コストが高い

**決定**: まずは `oapiv3` を試し、Arazzo は自前で `serde` 実装する方針で進める

### 2. データ構造の定義

```rust
// src/models/openapi.rs
pub struct OpenApiSpec {
    pub openapi: String,
    pub info: Info,
    pub servers: Vec<Server>,
    pub paths: BTreeMap<String, PathItem>,
    pub components: Option<Components>,
}

// src/models/arazzo.rs
pub struct ArazzoSpec {
    pub arazzo: String,
    pub info: Info,
    pub source_descriptions: Vec<SourceDescription>,
    pub workflows: Vec<Workflow>,
}

pub struct Workflow {
    pub workflow_id: String,
    pub summary: Option<String>,
    pub description: Option<String>,
    pub inputs: Option<serde_json::Value>,
    pub steps: Vec<Step>,
    pub outputs: Option<serde_json::Value>,
}

pub struct Step {
    pub step_id: String,
    pub description: Option<String>,
    pub operation_id: Option<String>,
    pub operation_path: Option<String>,
    pub parameters: Option<Vec<Parameter>>,
    pub request_body: Option<RequestBody>,
    pub success_criteria: Option<Vec<SuccessCriteria>>,
    pub outputs: Option<serde_json::Value>,
}
```

### 3. YAML ローダーの実装

```rust
// src/loader.rs
pub fn load_openapi(path: &Path) -> Result<OpenApiSpec, Error>;
pub fn load_arazzo(path: &Path) -> Result<ArazzoSpec, Error>;
```

### 4. バリデーション

- 必須フィールドのチェック
- スキーマバージョンの確認（OpenAPI 3.0/3.1、Arazzo 1.0）
- `operationId` や `operationPath` の参照解決

## 成果物

- [ ] `src/models/openapi.rs`: OpenAPI のデータ構造
- [ ] `src/models/arazzo.rs`: Arazzo のデータ構造
- [ ] `src/loader.rs`: YAML ローダー
- [ ] `tests/fixtures/`: テスト用の OpenAPI/Arazzo サンプルファイル
- [ ] 単体テスト（正常系・異常系）

## テストケース

- OpenAPI 3.0/3.1 の基本的な定義をパースできる
- Arazzo 1.0 の基本的な定義をパースできる
- 不正な YAML を読み込んだ時に適切なエラーが返る
- `operationId` による OpenAPI の操作参照が解決できる

## 参考資料

- [OpenAPI Specification 3.1.0](https://spec.openapis.org/oas/v3.1.0)
- [Arazzo Specification 1.0.0](https://spec.openapis.org/arazzo/latest.html)
- [oapiv3 crate](https://crates.io/crates/oapiv3)
- [serde documentation](https://serde.rs/)

## 次のステップ

このタスクが完了したら、**#002 フロー図の生成（グラフ構造への変換）** に進む。
