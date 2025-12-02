# #002 フロー図の生成（グラフ構造への変換）

**Phase**: 1 (MVP - 可視化)
**Priority**: High
**Status**: ✅ Completed (2024-12-03)
**Depends on**: #001

## 概要

Arazzo のワークフロー定義を有向グラフ（Directed Graph）に変換し、API コールフローを可視化可能な形式にする。

## 背景

Arazzo は YAML でステップ間の依存関係を定義しているが、そのままでは可視化できない。
グラフ構造（ノード = ステップ、エッジ = データフロー・依存関係）に変換することで、可視化や分析が可能になる。

## 実装内容

### 1. グラフライブラリの選定

以下のいずれかを検討：

- **petgraph**: Rust の標準的なグラフライブラリ
  - Pros: 豊富なアルゴリズム、メンテナンスされている
  - Cons: やや学習コストあり
- **graphlib**: よりシンプルなグラフライブラリ
- **自前実装**: `HashMap` + `Vec` で簡易実装

**決定**: `petgraph` を採用（トポロジカルソートなどのアルゴリズムが必要になるため）

### 2. データ構造の定義

```rust
// src/graph.rs
use petgraph::graph::{DiGraph, NodeIndex};

pub struct FlowGraph {
    pub graph: DiGraph<FlowNode, FlowEdge>,
    pub workflow_id: String,
}

#[derive(Debug, Clone)]
pub struct FlowNode {
    pub step_id: String,
    pub operation_id: Option<String>,
    pub operation_path: Option<String>,
    pub method: Option<String>, // GET, POST, etc.
    pub description: Option<String>,
}

#[derive(Debug, Clone)]
pub struct FlowEdge {
    pub edge_type: EdgeType,
    pub data_ref: Option<String>, // e.g., "$steps.login.outputs.token"
}

#[derive(Debug, Clone, PartialEq)]
pub enum EdgeType {
    Sequential,    // 順次実行
    Conditional,   // 条件分岐
    DataDependency, // データの依存関係
}
```

### 3. グラフ生成ロジック

```rust
// src/graph.rs
pub fn build_flow_graph(
    workflow: &Workflow,
    openapi: &OpenApiSpec,
) -> Result<FlowGraph, Error> {
    // 1. ステップごとにノードを作成
    // 2. ステップ間の依存関係をエッジとして追加
    // 3. successCriteria による条件分岐を解析
    // 4. outputs の参照を解析してデータ依存を追加
}
```

### 4. 依存関係の解析

- **順次実行**: ステップの定義順にエッジを張る
- **データ依存**: `$steps.xxx.outputs.yyy` の参照を解析
- **条件分岐**: `successCriteria` の `condition` を解析

### 5. グラフのバリデーション

- 循環参照がないか（DAG であることの確認）
- 未解決の参照がないか
- 到達不可能なノードがないか

## 成果物

- [ ] `src/graph.rs`: グラフ構造の定義と生成ロジック
- [ ] `src/graph/builder.rs`: グラフ構築のヘルパー関数
- [ ] `src/graph/validator.rs`: グラフのバリデーション
- [ ] 単体テスト（様々なワークフローパターン）

## テストケース

- 単純な順次ステップをグラフ化できる
- データ依存のあるステップをエッジで繋げられる
- 条件分岐を含むワークフローをグラフ化できる
- 循環参照がある場合にエラーを返す
- トポロジカルソートで実行順序を決定できる

## 出力形式

Phase 1 では以下の形式でグラフをエクスポート：

- **DOT 形式**: Graphviz で可視化可能
- **JSON 形式**: Web UI でのレンダリング用
- **Mermaid 形式**: Markdown での可視化用（オプション）

```rust
pub trait GraphExporter {
    fn export_dot(&self) -> String;
    fn export_json(&self) -> serde_json::Value;
}
```

## 参考資料

- [petgraph documentation](https://docs.rs/petgraph/)
- [Graphviz DOT language](https://graphviz.org/doc/info/lang.html)
- [Arazzo Specification - Runtime Expressions](https://spec.openapis.org/arazzo/latest.html#runtime-expressions)

## 次のステップ

このタスクが完了したら、**#003 Web UI での可視化** に進む。
