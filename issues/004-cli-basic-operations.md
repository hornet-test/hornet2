# #004 CLI での基本操作

**Phase**: 1 (MVP - 可視化)
**Priority**: High
**Status**: ✅ Completed (2024-12-03)
**Depends on**: #001, #002

## 概要

コマンドラインから OpenAPI/Arazzo ファイルを読み込み、可視化やバリデーションを実行できる CLI を実装する。

## 背景

Web UI だけでなく、CI/CD パイプラインやローカル開発で使いやすい CLI も提供する。
シンプルで直感的なコマンド体系を目指す。

## 実装内容

### 1. CLI フレームワークの選定

- **clap**: Rust のデファクトスタンダード
  - Pros: derive マクロで簡潔、豊富な機能
  - Cons: 特になし

**決定**: `clap` (v4) を採用

### 2. コマンド設計

```bash
# ワークフロー一覧を表示
hornet2 list --arazzo workflow.yaml

# フローをバリデーション
hornet2 validate --openapi openapi.yaml --arazzo workflow.yaml

# フローを可視化（DOT 形式で出力）
hornet2 visualize --arazzo workflow.yaml --format dot > flow.dot

# フローを可視化（Mermaid 形式で出力）
hornet2 visualize --arazzo workflow.yaml --format mermaid

# Web サーバーを起動
hornet2 serve --openapi openapi.yaml --arazzo workflow.yaml --port 3000

# 将来的なコマンド (Phase 2 以降)
hornet2 run --arazzo workflow.yaml --env prod
hornet2 test --arazzo workflow.yaml
hornet2 convert --arazzo workflow.yaml --to k6
```

### 3. コマンドライン引数の構造

```rust
// src/cli.rs
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "hornet2")]
#[command(about = "Document-driven API testing tool", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List workflows in Arazzo file
    List {
        #[arg(short, long)]
        arazzo: PathBuf,
    },

    /// Validate OpenAPI and Arazzo files
    Validate {
        #[arg(short, long)]
        openapi: PathBuf,
        #[arg(short, long)]
        arazzo: PathBuf,
    },

    /// Visualize workflow as a graph
    Visualize {
        #[arg(short, long)]
        arazzo: PathBuf,
        #[arg(short, long)]
        openapi: Option<PathBuf>,
        #[arg(short, long, default_value = "dot")]
        format: OutputFormat,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Start web server for visualization
    Serve {
        #[arg(short, long)]
        openapi: PathBuf,
        #[arg(short, long)]
        arazzo: PathBuf,
        #[arg(short, long, default_value = "3000")]
        port: u16,
    },
}

#[derive(Clone, ValueEnum)]
enum OutputFormat {
    Dot,
    Json,
    Mermaid,
}
```

### 4. 出力形式

- **標準出力**: デフォルトで人間が読みやすい形式
- **JSON 出力**: `--format json` でマシンリーダブルに
- **ファイル出力**: `--output` オプションでファイルに書き出し
- **カラー出力**: `termcolor` または `colored` を使用

### 5. エラーハンドリング

- ファイルが見つからない場合の明確なエラーメッセージ
- YAML パースエラーの詳細表示（行番号付き）
- バリデーションエラーの一覧表示

```rust
// src/error.rs
use thiserror::Error;

#[derive(Error, Debug)]
pub enum HornetError {
    #[error("Failed to load OpenAPI file: {0}")]
    OpenApiLoadError(String),

    #[error("Failed to load Arazzo file: {0}")]
    ArazzoLoadError(String),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Graph generation error: {0}")]
    GraphError(String),
}
```

## 成果物

- [x] `src/cli.rs`: CLI の引数定義
- [x] `src/commands/list.rs`: list コマンドの実装
- [x] `src/commands/validate.rs`: validate コマンドの実装
- [x] `src/commands/visualize.rs`: visualize コマンドの実装
- [x] `src/commands/serve.rs`: serve コマンドの実装
- [x] `src/error.rs`: エラー型の定義
- [x] ヘルプメッセージの充実

## テストケース

- `hornet2 --help` でヘルプが表示される
- `hornet2 list --arazzo test.yaml` でワークフロー一覧が表示される
- `hornet2 validate` で正常なファイルが検証できる
- `hornet2 validate` で不正なファイルがエラーになる
- `hornet2 visualize --format dot` で DOT 形式が出力される
- `hornet2 serve` でサーバーが起動する

## UX の考慮事項

- **進捗表示**: 処理に時間がかかる場合はプログレスバーを表示
- **インタラクティブモード**: 将来的には対話的な選択肢を提供
- **カラー出力**: エラーは赤、成功は緑、警告は黄色
- **例文の提供**: `--help` に実際の使用例を記載

## 参考資料

- [clap documentation](https://docs.rs/clap/)
- [thiserror documentation](https://docs.rs/thiserror/)
- [colored](https://docs.rs/colored/)

## 次のステップ

Phase 1 が完了したら、**Phase 2: テスト実行** に進む。
次は **#005 外部ツール (k6 など) への DSL 変換** を検討。
