# #001 OpenAPI/Arazzo パーサーの実装 - 完了報告

**Status**: ✅ Completed
**Date**: 2024-12-03

## 実装内容

### 1. プロジェクト構造

```
hornet2/
├── src/
│   ├── lib.rs                  # ライブラリのエントリーポイント
│   ├── main.rs                 # CLI のエントリーポイント
│   ├── error.rs                # エラー型の定義
│   ├── models/
│   │   ├── mod.rs              # モデルのエクスポート
│   │   └── arazzo.rs           # Arazzo の完全なデータ構造
│   └── loader/
│       ├── mod.rs              # ローダーのエクスポート
│       ├── openapi.rs          # OpenAPI ローダー
│       └── arazzo.rs           # Arazzo ローダー
├── tests/
│   ├── integration_test.rs     # 統合テスト
│   └── fixtures/
│       ├── openapi.yaml        # テスト用 OpenAPI 定義
│       └── arazzo.yaml         # テスト用 Arazzo 定義
└── Cargo.toml
```

### 2. 使用したクレート

- **serde / serde_yaml / serde_json**: YAML/JSON のパースと構造体へのデシリアライズ
- **oas3**: OpenAPI 3.x の既存パーサー（車輪の再発明を避ける）
- **thiserror / anyhow**: エラーハンドリング
- **tempfile**: テスト用の一時ファイル作成

### 3. 実装の詳細

#### 3.1 OpenAPI パーサー

`oas3` クレートを使用して OpenAPI 3.0/3.1 をパース。
- バージョンチェック（3.0.x / 3.1.x のみ対応）
- パスの存在チェック
- 基本的なバリデーション

#### 3.2 Arazzo パーサー

Arazzo Specification 1.0.0 に準拠した完全なデータ構造を自前実装。
- すべての必須フィールドとオプショナルフィールドをカバー
- `serde` の `Deserialize` / `Serialize` トレイトを実装
- バリデーション機能を内蔵：
  - Arazzo バージョンチェック
  - 重複する workflow ID の検出
  - 重複する step ID の検出
  - ステップに operation 参照が存在するかチェック

#### 3.3 CLI 実装

最小限の CLI コマンドを実装：

```bash
# OpenAPI のバリデーション
hornet2 validate-openapi tests/fixtures/openapi.yaml

# Arazzo のバリデーション
hornet2 validate-arazzo tests/fixtures/arazzo.yaml
```

成功時は `✓` マーク付きで情報を表示、失敗時は `✗` マークとエラーメッセージを表示。

### 4. テストケース

#### 4.1 単体テスト

**OpenAPI ローダー**:
- ✅ 正常な OpenAPI 3.0.0 ファイルを読み込める
- ✅ 不正なバージョン（2.0.0）を検出できる
- ✅ パスが空の場合にエラーを返す
- ✅ 存在しないファイルでエラーを返す

**Arazzo ローダー**:
- ✅ 正常な Arazzo 1.0.0 ファイルを読み込める
- ✅ 不正なバージョン（2.0.0）を検出できる
- ✅ 重複する workflow ID を検出できる
- ✅ 重複する step ID を検出できる
- ✅ operation 参照のないステップを検出できる
- ✅ 存在しないファイルでエラーを返す

**合計**: 10 個の単体テストが全て pass ✅

#### 4.2 統合テスト

- ✅ OpenAPI fixture を正しく読み込める
- ✅ Arazzo fixture を正しく読み込める
- ✅ Arazzo のステップパラメータを正しくパースできる
- ✅ Arazzo の outputs を正しくパースできる
- ✅ Arazzo の success criteria を正しくパースできる
- ✅ 複数のワークフローを正しく読み込める

**合計**: 6 個の統合テストが全て pass ✅

### 5. サンプルファイル

#### 5.1 OpenAPI サンプル

`tests/fixtures/openapi.yaml` にユーザー管理 API の定義を作成：
- `/register` (POST): ユーザー登録
- `/login` (POST): ログイン
- `/profile` (GET/PUT): プロフィール取得・更新

すべての操作に `operationId` を設定し、リクエスト/レスポンスの example を記載。

#### 5.2 Arazzo サンプル

`tests/fixtures/arazzo.yaml` に 2 つのワークフローを作成：

**1. user-onboarding-flow**:
- ステップ 1: ユーザー登録
- ステップ 2: ログイン（トークン取得）
- ステップ 3: プロフィール取得（トークンを使用）
- ステップ 4: プロフィール更新（トークンを使用）

データフローの例:
- `$inputs.username` → リクエストボディに注入
- `$response.body.token` → 次のステップの Authorization ヘッダーに使用
- `$steps.login.outputs.token` → 後続ステップで参照

**2. simple-login-flow**:
- ステップ 1: ログインのみ

### 6. 実行例

```bash
$ cargo run -- validate-openapi tests/fixtures/openapi.yaml
Loading OpenAPI from: tests/fixtures/openapi.yaml
✓ OpenAPI loaded successfully
  Title: User Management API
  Version: 1.0.0
  OpenAPI Version: 3.0.0
  Paths: 3
```

```bash
$ cargo run -- validate-arazzo tests/fixtures/arazzo.yaml
Loading Arazzo from: tests/fixtures/arazzo.yaml
✓ Arazzo loaded successfully
  Title: User Registration and Profile Update Flow
  Version: 1.0.0
  Arazzo Version: 1.0.0
  Workflows: 2
    - user-onboarding-flow (4 steps)
    - simple-login-flow (1 steps)
```

## 成果物チェックリスト

- ✅ `src/models/arazzo.rs`: Arazzo のデータ構造（完全実装）
- ✅ `src/models/mod.rs`: OpenAPI の re-export（oas3 クレート使用）
- ✅ `src/loader/openapi.rs`: OpenAPI ローダー + バリデーション + 単体テスト
- ✅ `src/loader/arazzo.rs`: Arazzo ローダー + 単体テスト
- ✅ `src/error.rs`: エラー型の定義
- ✅ `tests/fixtures/openapi.yaml`: 実用的なテスト用 OpenAPI
- ✅ `tests/fixtures/arazzo.yaml`: 実用的なテスト用 Arazzo（データフロー付き）
- ✅ `tests/integration_test.rs`: 6 個の統合テスト
- ✅ 単体テスト 10 個（すべて pass）
- ✅ 統合テスト 6 個（すべて pass）

## 設計上の判断

### 1. OpenAPI パーサーに既存クレートを使用

**理由**: OpenAPI 3.x は仕様が大きく複雑なため、既存の `oas3` クレートを使用することで実装コストを削減。oas3 は十分にメンテナンスされており、信頼性が高い。

### 2. Arazzo は自前実装

**理由**: Arazzo Specification は比較的新しく、Rust の既存ライブラリが存在しないため自前実装。serde を使うことで YAML パースは自動化され、実装負荷は低い。

### 3. バリデーションをデータ構造に内蔵

**理由**: `ArazzoSpec::validate()` のように、データ構造自身がバリデーションメソッドを持つことで、ロード時に自動的にチェックできる。

### 4. Runtime Expression は未実装

**理由**: Phase 1 はパーサーとバリデーションのみ。`$steps.xxx.outputs.yyy` などのランタイム式の評価は #002 以降で実装予定。

## 次のステップ

✅ **#001 完了**

次は **#002 フロー図の生成（グラフ構造への変換）** に進む準備が整いました。

#002 では：
- Arazzo のワークフローを有向グラフ（DAG）に変換
- `petgraph` を使用してグラフ構造を構築
- ステップ間の依存関係（データフロー、順次実行、条件分岐）を解析
- DOT / JSON 形式でのエクスポート

## 備考

- すべてのテストが通っており、実装は安定している
- OpenAPI と Arazzo の fixture は現実的なユースケースを反映している
- エラーメッセージは開発者にとって分かりやすい形式
- CLI は最小限だが、基本的な動作確認には十分
