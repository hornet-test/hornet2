# #008 Rust 製 HTTP クライアントの実装

**Phase**: 3 (高速エンジン化)
**Priority**: High
**Status**: Todo
**Depends on**: #001, #002

## 概要

k6 への依存を排除し、Rust 製の HTTP クライアントを使って直接 API リクエストを実行できるようにする。

## 背景

Phase 2 では k6 を使ってテストを実行していたが、以下の課題がある：

- k6 のインストールが必要
- JavaScript への変換オーバーヘッド
- k6 の出力フォーマットに依存

Phase 3 では Rust の非同期ランタイムを活用し、高速かつメモリ効率の良い実行エンジンを実装する。

## 実装内容

### 1. HTTP クライアントライブラリの選定

- **reqwest**: Rust のデファクトスタンダード
  - Pros: 豊富な機能、async/await 対応
  - Cons: やや重い
- **hyper**: 低レベルな HTTP ライブラリ
  - Pros: 高速、柔軟
  - Cons: 低レベルすぎる
- **ureq**: シンプルな同期 HTTP クライアント
  - Pros: 軽量
  - Cons: 非同期に対応していない

**決定**: `reqwest` + `tokio` を採用（機能と性能のバランスが良い）

### 2. ステップ実行エンジン

```rust
// src/executor/mod.rs
pub struct WorkflowExecutor {
    openapi: OpenApiSpec,
    client: reqwest::Client,
    context: ExecutionContext,
}

pub struct ExecutionContext {
    /// ステップの出力を保存
    outputs: HashMap<String, serde_json::Value>,
    /// 環境変数
    env: HashMap<String, String>,
}

impl WorkflowExecutor {
    pub async fn execute(&mut self, workflow: &Workflow) -> Result<TestResult, Error> {
        let mut step_results = Vec::new();

        for step in &workflow.steps {
            let result = self.execute_step(step).await?;
            step_results.push(result);

            // successCriteria をチェック
            if !self.check_success_criteria(step, &result)? {
                break;
            }
        }

        Ok(TestResult { /* ... */ })
    }

    async fn execute_step(&mut self, step: &Step) -> Result<StepResult, Error> {
        // 1. operationId または operationPath から OpenAPI の操作を取得
        let operation = self.resolve_operation(step)?;

        // 2. パラメータを解決（$steps.xxx.outputs.yyy など）
        let params = self.resolve_parameters(step)?;

        // 3. HTTP リクエストを構築
        let request = self.build_request(operation, params)?;

        // 4. リクエストを送信
        let start = Instant::now();
        let response = self.client.execute(request).await?;
        let duration = start.elapsed();

        // 5. レスポンスを保存
        let body = response.json::<serde_json::Value>().await?;
        self.save_outputs(step, &body)?;

        Ok(StepResult { /* ... */ })
    }
}
```

### 3. ランタイム式の評価

Arazzo の `$steps.xxx.outputs.yyy` や `$response.body.token` を評価：

```rust
// src/executor/runtime_expr.rs
pub fn evaluate_runtime_expr(
    expr: &str,
    context: &ExecutionContext,
) -> Result<serde_json::Value, Error> {
    if expr.starts_with("$steps.") {
        // $steps.login.outputs.token
        let parts: Vec<&str> = expr.split('.').collect();
        let step_id = parts[1];
        let output_path = &parts[3..].join(".");

        context.outputs
            .get(step_id)
            .and_then(|v| v.pointer(output_path))
            .cloned()
            .ok_or_else(|| Error::RuntimeExprError(expr.to_string()))
    } else if expr.starts_with("$env.") {
        // $env.API_KEY
        let key = expr.strip_prefix("$env.").unwrap();
        context.env
            .get(key)
            .cloned()
            .map(serde_json::Value::String)
            .ok_or_else(|| Error::EnvVarNotFound(key.to_string()))
    } else if expr.starts_with("$response.") {
        // これは execute_step 内で処理
        unimplemented!()
    } else {
        // リテラル値
        Ok(serde_json::Value::String(expr.to_string()))
    }
}
```

### 4. 並列実行

依存関係のないステップを並列実行：

```rust
// src/executor/parallel.rs
pub async fn execute_parallel(
    &mut self,
    steps: Vec<&Step>,
) -> Result<Vec<StepResult>, Error> {
    let futures = steps.into_iter().map(|step| {
        self.execute_step(step)
    });

    // すべてのステップを並列実行
    let results = futures::future::try_join_all(futures).await?;
    Ok(results)
}
```

### 5. リクエスト/レスポンスのロギング

```rust
// src/executor/logger.rs
pub fn log_request(request: &reqwest::Request) {
    tracing::info!(
        method = %request.method(),
        url = %request.url(),
        "Sending request"
    );
}

pub fn log_response(response: &reqwest::Response, duration: Duration) {
    tracing::info!(
        status = %response.status(),
        duration_ms = duration.as_millis(),
        "Received response"
    );
}
```

## 成果物

- [ ] `src/executor/mod.rs`: ワークフロー実行エンジン
- [ ] `src/executor/runtime_expr.rs`: ランタイム式の評価
- [ ] `src/executor/parallel.rs`: 並列実行ロジック
- [ ] `src/executor/logger.rs`: リクエスト/レスポンスのロギング
- [ ] `Cargo.toml`: `reqwest`, `tokio`, `tracing` の追加

## テストケース

- 単一ステップを実行できる
- 複数ステップを順次実行できる
- `$steps.xxx.outputs.yyy` が正しく評価される
- `$env.XXX` が環境変数から取得される
- `successCriteria` が失敗したらステップを停止する
- 並列実行可能なステップが並列実行される

## パフォーマンス目標

- 1000 リクエスト/秒を安定して処理
- メモリ使用量を 100MB 以下に抑える
- CPU 使用率を 50% 以下に抑える（4 コアマシンの場合）

## CLI コマンド

```bash
# Rust エンジンで実行（デフォルト）
hornet2 run --arazzo workflow.yaml --engine rust

# k6 エンジンで実行
hornet2 run --arazzo workflow.yaml --engine k6

# 並列実行数を指定
hornet2 run --arazzo workflow.yaml --concurrency 10

# ログレベルを指定
hornet2 run --arazzo workflow.yaml --log-level debug
```

## 参考資料

- [reqwest documentation](https://docs.rs/reqwest/)
- [tokio documentation](https://tokio.rs/)
- [tracing documentation](https://docs.rs/tracing/)
- [Arazzo Runtime Expressions](https://spec.openapis.org/arazzo/latest.html#runtime-expressions)

## 次のステップ

このタスクが完了したら、**#009 負荷試験機能の実装** に進む。
