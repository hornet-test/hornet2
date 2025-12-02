# #009 負荷試験機能の実装

**Phase**: 3 (高速エンジン化)
**Priority**: Medium
**Status**: Todo
**Depends on**: #008

## 概要

単一テスト実行だけでなく、RPS（Requests Per Second）や VU（Virtual Users）を指定した負荷試験機能を実装する。

## 背景

API のパフォーマンス測定や閾値の把握には、負荷をかけた状態でのテストが必要。
k6 のような負荷試験ツールと同等の機能を Rust で実装する。

## 実装内容

### 1. 負荷試験の設定

```yaml
# Arazzo の拡張フィールド（x- prefix）
x-load-test:
  mode: rps          # rps | vus | constant
  rps: 100           # 100 リクエスト/秒
  duration: 30s      # 30 秒間実行
  rampUp: 5s         # 5 秒かけて徐々に増加
  rampDown: 5s       # 5 秒かけて徐々に減少
```

または

```yaml
x-load-test:
  mode: vus
  vus: 10            # 10 並列ユーザー
  iterations: 100    # 各ユーザーが 100 回実行
```

### 2. 負荷生成エンジン

```rust
// src/load/mod.rs
pub struct LoadTestConfig {
    pub mode: LoadMode,
    pub duration: Duration,
    pub ramp_up: Option<Duration>,
    pub ramp_down: Option<Duration>,
}

pub enum LoadMode {
    /// 一定の RPS を維持
    ConstantRPS { rps: u64 },
    /// 一定数の仮想ユーザーを維持
    ConstantVUs { vus: usize, iterations: Option<usize> },
    /// RPS を段階的に増加
    RampingRPS { start_rps: u64, end_rps: u64 },
}

pub struct LoadTestRunner {
    executor: WorkflowExecutor,
    config: LoadTestConfig,
}

impl LoadTestRunner {
    pub async fn run(&mut self, workflow: &Workflow) -> Result<LoadTestResult, Error> {
        match self.config.mode {
            LoadMode::ConstantRPS { rps } => {
                self.run_constant_rps(workflow, rps).await
            }
            LoadMode::ConstantVUs { vus, iterations } => {
                self.run_constant_vus(workflow, vus, iterations).await
            }
            LoadMode::RampingRPS { start_rps, end_rps } => {
                self.run_ramping_rps(workflow, start_rps, end_rps).await
            }
        }
    }
}
```

### 3. RPS モードの実装

```rust
// src/load/rps.rs
async fn run_constant_rps(
    &mut self,
    workflow: &Workflow,
    rps: u64,
) -> Result<LoadTestResult, Error> {
    let interval = Duration::from_secs(1) / rps as u32;
    let mut ticker = tokio::time::interval(interval);
    let start = Instant::now();

    let mut tasks = Vec::new();

    while start.elapsed() < self.config.duration {
        ticker.tick().await;

        // 新しいタスクを起動
        let executor = self.executor.clone();
        let workflow = workflow.clone();
        let task = tokio::spawn(async move {
            executor.execute(&workflow).await
        });
        tasks.push(task);
    }

    // すべてのタスクが完了するまで待機
    let results = futures::future::join_all(tasks).await;

    Ok(LoadTestResult::from_results(results))
}
```

### 4. VU モードの実装

```rust
// src/load/vus.rs
async fn run_constant_vus(
    &mut self,
    workflow: &Workflow,
    vus: usize,
    iterations: Option<usize>,
) -> Result<LoadTestResult, Error> {
    let mut tasks = Vec::new();

    for _ in 0..vus {
        let executor = self.executor.clone();
        let workflow = workflow.clone();
        let iterations = iterations.unwrap_or(usize::MAX);

        let task = tokio::spawn(async move {
            let mut results = Vec::new();
            for _ in 0..iterations {
                let result = executor.execute(&workflow).await?;
                results.push(result);
            }
            Ok::<Vec<TestResult>, Error>(results)
        });
        tasks.push(task);
    }

    let results = futures::future::join_all(tasks).await;
    Ok(LoadTestResult::from_results(results))
}
```

### 5. Ramp-up/Ramp-down

```rust
// src/load/ramp.rs
async fn run_ramping_rps(
    &mut self,
    workflow: &Workflow,
    start_rps: u64,
    end_rps: u64,
) -> Result<LoadTestResult, Error> {
    let ramp_duration = self.config.ramp_up.unwrap_or(Duration::from_secs(0));
    let steps = 10; // 10 段階で増加
    let step_duration = ramp_duration / steps;
    let rps_step = (end_rps - start_rps) / steps as u64;

    let mut current_rps = start_rps;
    let mut all_results = Vec::new();

    for _ in 0..steps {
        let results = self.run_constant_rps_for_duration(
            workflow,
            current_rps,
            step_duration,
        ).await?;
        all_results.extend(results);
        current_rps += rps_step;
    }

    Ok(LoadTestResult::from_results(all_results))
}
```

### 6. メトリクスの収集

```rust
// src/load/metrics.rs
#[derive(Debug, Serialize)]
pub struct LoadTestResult {
    pub total_requests: usize,
    pub successful_requests: usize,
    pub failed_requests: usize,
    pub duration: Duration,
    pub avg_response_time: Duration,
    pub min_response_time: Duration,
    pub max_response_time: Duration,
    pub p50_response_time: Duration,
    pub p95_response_time: Duration,
    pub p99_response_time: Duration,
    pub requests_per_second: f64,
    pub error_rate: f64,
}

impl LoadTestResult {
    pub fn from_results(results: Vec<TestResult>) -> Self {
        // レスポンスタイムを集計
        let mut response_times: Vec<Duration> = results
            .iter()
            .map(|r| Duration::from_millis(r.duration_ms))
            .collect();
        response_times.sort();

        let p50 = percentile(&response_times, 0.5);
        let p95 = percentile(&response_times, 0.95);
        let p99 = percentile(&response_times, 0.99);

        // ...
    }
}

fn percentile(sorted: &[Duration], p: f64) -> Duration {
    let idx = (sorted.len() as f64 * p) as usize;
    sorted[idx.min(sorted.len() - 1)]
}
```

## 成果物

- [ ] `src/load/mod.rs`: 負荷試験のエントリーポイント
- [ ] `src/load/rps.rs`: RPS モードの実装
- [ ] `src/load/vus.rs`: VU モードの実装
- [ ] `src/load/ramp.rs`: Ramp-up/Ramp-down の実装
- [ ] `src/load/metrics.rs`: メトリクスの収集・集計
- [ ] `src/commands/load.rs`: load コマンドの実装

## テストケース

- 一定の RPS で負荷をかけられる
- 一定数の VU で負荷をかけられる
- Ramp-up で徐々に負荷を増加できる
- Ramp-down で徐々に負荷を減少できる
- メトリクスが正しく集計される
- パーセンタイル値が正しく計算される

## CLI コマンド

```bash
# RPS モードで負荷試験
hornet2 load --arazzo workflow.yaml --rps 100 --duration 30s

# VU モードで負荷試験
hornet2 load --arazzo workflow.yaml --vus 10 --iterations 100

# Ramp-up 付き
hornet2 load --arazzo workflow.yaml --rps 100 --duration 30s --ramp-up 5s --ramp-down 5s

# 結果をリアルタイムで表示
hornet2 load --arazzo workflow.yaml --rps 100 --duration 30s --live

# 結果を JSON で出力
hornet2 load --arazzo workflow.yaml --rps 100 --duration 30s --output results.json
```

## リアルタイム表示

```rust
// src/load/live.rs
pub struct LiveReporter {
    tx: mpsc::Sender<TestResult>,
}

impl LiveReporter {
    pub async fn start(&mut self) {
        let mut rx = self.tx.subscribe();
        let mut interval = tokio::time::interval(Duration::from_secs(1));
        let mut metrics = Metrics::new();

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    // 1 秒ごとに統計を表示
                    println!("{}", metrics.format());
                }
                result = rx.recv() => {
                    metrics.add(result);
                }
            }
        }
    }
}
```

出力例：

```
Time: 5s | RPS: 98.2 | Success: 491 | Failed: 0 | Avg: 120ms | P95: 250ms | P99: 350ms
Time: 6s | RPS: 99.5 | Success: 589 | Failed: 1 | Avg: 125ms | P95: 260ms | P99: 380ms
```

## 参考資料

- [k6 load testing](https://k6.io/docs/using-k6/scenarios/)
- [tokio async programming](https://tokio.rs/tokio/tutorial)

## 次のステップ

このタスクが完了したら、**#010 並列実行・非同期最適化** に進む。
