# #003 Web UI での可視化

**Phase**: 1 (MVP - 可視化)
**Priority**: Medium
**Status**: ✅ Completed (2024-12-03)
**Depends on**: #002

## 概要

生成したフローグラフをブラウザ上でインタラクティブに可視化する Web UI を実装する。

## 背景

CLI でのテキスト出力だけでなく、グラフィカルな可視化により、開発者が API フローを直感的に理解できるようにする。
シングルバイナリで Web サーバーを起動し、SPA として UI を提供する。

## 実装内容

### 1. Web フレームワークの選定

- **axum**: 軽量・高速な Web フレームワーク
  - Pros: tokio ベース、モダンな API
  - Cons: やや新しい
- **actix-web**: 実績のあるフレームワーク
- **warp**: コンパクトなフレームワーク

**決定**: `axum` を採用（Rust の非同期エコシステムとの親和性が高い）

### 2. フロントエンドの構成

以下のいずれかを検討：

- **Option A**: Vanilla JS + D3.js / Cytoscape.js
  - Pros: 軽量、バンドルが簡単
  - Cons: 大規模になると管理が大変
- **Option B**: React / Vue + グラフライブラリ
  - Pros: コンポーネント化しやすい
  - Cons: ビルドプロセスが必要
- **Option C**: Leptos / Yew (Rust 製 SPA フレームワーク)
  - Pros: Rust で統一できる
  - Cons: エコシステムがまだ発展途上

**決定**: まずは **Option A (Vanilla JS + Cytoscape.js)** でシンプルに開始し、後で React などに移行する

### 3. アーキテクチャ

```
┌─────────────────────────────────────┐
│  Rust Backend (axum)                │
│  - /api/workflows                   │
│  - /api/graph/{workflow_id}         │
│  - /api/visualize/{workflow_id}     │
│  - Static file serving (SPA)        │
└─────────────────────────────────────┘
              ↕
┌─────────────────────────────────────┐
│  Frontend (SPA)                     │
│  - index.html                       │
│  - app.js (graph rendering)         │
│  - Cytoscape.js                     │
└─────────────────────────────────────┘
```

### 4. API エンドポイント

```rust
// src/server/api.rs
// GET /api/workflows - ワークフロー一覧を返す
// GET /api/graph/{workflow_id} - グラフ JSON を返す
// GET /api/visualize/{workflow_id} - 可視化ページを返す
// GET / - SPA の index.html を返す
```

### 5. グラフ可視化機能

- **ノードの表示**:
  - `step_id` をラベルに
  - `operationId` や HTTP メソッドを表示
  - ノードの色分け（GET=青、POST=緑、DELETE=赤など）

- **エッジの表示**:
  - 順次実行は実線
  - 条件分岐は点線
  - データ依存は破線 + ラベル

- **インタラクション**:
  - ノードをクリックで詳細表示
  - ズーム・パン操作
  - レイアウトアルゴリズムの選択（階層型、力学モデルなど）

### 6. 静的ファイルの埋め込み

バイナリに HTML/CSS/JS を埋め込むため、`rust-embed` または `include_str!` を使用：

```rust
// src/server/static.rs
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "ui/dist"]
struct Asset;
```

## 成果物

- [ ] `src/server/mod.rs`: axum サーバーの実装
- [ ] `src/server/api.rs`: API エンドポイント
- [ ] `ui/index.html`: SPA のエントリーポイント
- [ ] `ui/app.js`: グラフレンダリングロジック
- [ ] `ui/style.css`: スタイル定義

## テストケース

- サーバーが起動し、`http://localhost:3000` でアクセスできる
- `/api/workflows` でワークフロー一覧が取得できる
- `/api/graph/{workflow_id}` でグラフ JSON が取得できる
- ブラウザでグラフが正しく表示される
- ノードをクリックすると詳細が表示される

## 技術スタック

- **Backend**: axum + tokio
- **Frontend**: Vanilla JS + Cytoscape.js
- **Build**: `trunk` または手動ビルド

## 参考資料

- [axum documentation](https://docs.rs/axum/)
- [Cytoscape.js](https://js.cytoscape.org/)
- [rust-embed](https://crates.io/crates/rust-embed)

## 次のステップ

このタスクが完了したら、**#004 CLI での基本操作** に進む。
