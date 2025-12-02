# #006 テスト実行の自動化

**Phase**: 2 (テスト実行)
**Priority**: Medium
**Status**: Todo
**Depends on**: #005

## 概要

生成した k6 スクリプトを自動実行し、結果を収集・レポート化する仕組みを構築する。

## 背景

Phase 2 では Arazzo から k6 スクリプトを生成できるようになったが、手動実行では使い勝手が悪い。
CI/CD パイプラインでの自動実行や、複数のワークフローを一括実行できるようにする。

## 実装内容

### 1. テスト実行フロー

```
Arazzo YAML
    ↓
[Convert to k6]
    ↓
k6 Script (JS)
    ↓
[Execute k6]
    ↓
k6 Output (JSON)
    ↓
[Parse Results]
    ↓
Test Report (JSON/HTML)
```

### 2. テスト結果の構造

```rust
// src/runner/result.rs
#[derive(Debug, Serialize, Deserialize)]
pub struct TestResult {
    pub workflow_id: String,
    pub status: TestStatus,
    pub duration_ms: u64,
    pub steps: Vec<StepResult>,
    pub summary: TestSummary,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum TestStatus {
    Passed,
    Failed,
    Skipped,
    Error,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StepResult {
    pub step_id: String,
    pub status: TestStatus,
    pub duration_ms: u64,
    pub http_status: Option<u16>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TestSummary {
    pub total_requests: usize,
    pub passed_requests: usize,
    pub failed_requests: usize,
    pub avg_response_time_ms: f64,
    pub min_response_time_ms: u64,
    pub max_response_time_ms: u64,
}
```

### 3. k6 出力のパース

k6 は `--out json=results.json` オプションで結果を出力できる：

```rust
// src/runner/k6.rs
pub fn parse_k6_output(json_path: &Path) -> Result<TestResult, Error> {
    let file = File::open(json_path)?;
    let reader = BufReader::new(file);

    // k6 の JSON 形式をパース
    // metrics, checks, thresholds などを抽出
}
```

### 4. 複数ワークフローの実行

```rust
// src/runner/batch.rs
pub struct BatchRunner {
    workflows: Vec<Workflow>,
    openapi: OpenApiSpec,
}

impl BatchRunner {
    pub fn run_all(&self) -> Result<Vec<TestResult>, Error> {
        let mut results = Vec::new();
        for workflow in &self.workflows {
            let result = self.run_workflow(workflow)?;
            results.push(result);
        }
        Ok(results)
    }
}
```

### 5. 環境変数の注入

```yaml
# Arazzo
workflows:
  - workflowId: login-test
    inputs:
      baseUrl: $env.BASE_URL
      apiKey: $env.API_KEY
```

```rust
// src/runner/env.rs
pub fn resolve_env_vars(workflow: &mut Workflow) -> Result<(), Error> {
    // $env.XXX を環境変数から解決
    // .env ファイルのサポート
}
```

### 6. CLI コマンド

```bash
# 単一ワークフローを実行
hornet2 run --arazzo workflow.yaml --openapi openapi.yaml

# 環境変数を指定して実行
hornet2 run --arazzo workflow.yaml --env BASE_URL=https://staging.example.com

# .env ファイルを読み込んで実行
hornet2 run --arazzo workflow.yaml --env-file .env.staging

# 複数ワークフローを一括実行
hornet2 run --arazzo workflow.yaml --all

# 結果を JSON で出力
hornet2 run --arazzo workflow.yaml --output results.json

# 失敗時に即座に停止
hornet2 run --arazzo workflow.yaml --fail-fast
```

## 成果物

- [ ] `src/runner/mod.rs`: テスト実行のエントリーポイント
- [ ] `src/runner/result.rs`: テスト結果の構造
- [ ] `src/runner/batch.rs`: 複数ワークフローの実行
- [ ] `src/runner/env.rs`: 環境変数の解決
- [ ] `src/commands/run.rs`: run コマンドの実装
- [ ] `.env.example`: 環境変数のサンプル

## テストケース

- 単一ワークフローを実行して結果が取得できる
- 複数ワークフローを一括実行できる
- 環境変数が正しく注入される
- .env ファイルから設定を読み込める
- テスト失敗時に適切なエラーメッセージが表示される
- 結果を JSON ファイルに出力できる

## CI/CD での利用

```yaml
# .github/workflows/api-test.yml
name: API Test
on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Install hornet2
        run: |
          curl -L https://github.com/user/hornet2/releases/latest/download/hornet2-linux -o hornet2
          chmod +x hornet2
      - name: Run API tests
        run: |
          ./hornet2 run --arazzo tests/workflow.yaml --openapi api/openapi.yaml --output results.json
        env:
          BASE_URL: ${{ secrets.API_BASE_URL }}
          API_KEY: ${{ secrets.API_KEY }}
      - name: Upload results
        uses: actions/upload-artifact@v3
        with:
          name: test-results
          path: results.json
```

## 参考資料

- [k6 output formats](https://k6.io/docs/get-started/results-output/)
- [dotenv-rs](https://crates.io/crates/dotenv)

## 次のステップ

このタスクが完了したら、**#007 結果レポートの生成** に進む。
