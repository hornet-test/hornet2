# #010 並列実行・非同期最適化

**Phase**: 3 (高速エンジン化)
**Priority**: Medium
**Status**: Todo
**Depends on**: #008, #009

## 概要

パフォーマンスを最大化するため、並列実行の最適化、メモリ効率の改善、CPU 使用率の最適化を行う。

## 背景

Phase 3 では Rust 製の実行エンジンを実装したが、さらなる性能向上のためにボトルネックを特定し、最適化する。

## 実装内容

### 1. 並列実行の最適化

#### 1.1 接続プールの調整

```rust
// src/executor/client.rs
pub fn create_optimized_client() -> Result<reqwest::Client, Error> {
    reqwest::Client::builder()
        .pool_max_idle_per_host(100)  // アイドル接続を増やす
        .pool_idle_timeout(Duration::from_secs(90))
        .timeout(Duration::from_secs(30))
        .tcp_nodelay(true)  // Nagle アルゴリズムを無効化
        .build()
}
```

#### 1.2 並列度の自動調整

```rust
// src/executor/adaptive.rs
pub struct AdaptiveExecutor {
    max_concurrency: usize,
    current_concurrency: AtomicUsize,
    response_time_threshold: Duration,
}

impl AdaptiveExecutor {
    /// レスポンスタイムが閾値を超えたら並列度を下げる
    pub fn adjust_concurrency(&self, avg_response_time: Duration) {
        let current = self.current_concurrency.load(Ordering::Relaxed);

        if avg_response_time > self.response_time_threshold {
            // レスポンスが遅い → 並列度を下げる
            let new = (current * 9 / 10).max(1);
            self.current_concurrency.store(new, Ordering::Relaxed);
            tracing::warn!("Reducing concurrency to {}", new);
        } else if current < self.max_concurrency {
            // レスポンスが速い → 並列度を上げる
            let new = (current * 11 / 10).min(self.max_concurrency);
            self.current_concurrency.store(new, Ordering::Relaxed);
            tracing::info!("Increasing concurrency to {}", new);
        }
    }
}
```

### 2. メモリ効率の改善

#### 2.1 ストリーミングレスポンス

```rust
// src/executor/streaming.rs
pub async fn execute_step_streaming(
    &mut self,
    step: &Step,
) -> Result<StepResult, Error> {
    let response = self.client.execute(request).await?;

    // 大きなレスポンスはストリーミング処理
    if response.content_length() > Some(1_000_000) {
        let mut stream = response.bytes_stream();
        let mut body = Vec::new();

        while let Some(chunk) = stream.next().await {
            body.extend_from_slice(&chunk?);
        }

        // 必要な部分だけ抽出
        let json: serde_json::Value = serde_json::from_slice(&body)?;
        self.save_outputs(step, &json)?;
    } else {
        // 小さいレスポンスは通常通り
        let body = response.json().await?;
        self.save_outputs(step, &body)?;
    }

    Ok(/* ... */)
}
```

#### 2.2 出力の選択的保存

```rust
// src/executor/outputs.rs
pub fn save_outputs_selective(
    &mut self,
    step: &Step,
    response: &serde_json::Value,
) -> Result<(), Error> {
    // outputs に定義されたフィールドのみ保存
    if let Some(outputs) = &step.outputs {
        let mut saved = serde_json::Map::new();

        for (key, expr) in outputs.as_object().unwrap() {
            if let Some(value) = evaluate_json_path(response, expr) {
                saved.insert(key.clone(), value);
            }
        }

        self.context.outputs.insert(
            step.step_id.clone(),
            serde_json::Value::Object(saved),
        );
    }

    Ok(())
}
```

### 3. CPU 最適化

#### 3.1 専用スレッドプールの使用

```rust
// src/executor/threadpool.rs
pub fn create_custom_runtime() -> Result<tokio::runtime::Runtime, Error> {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(num_cpus::get())
        .thread_name("hornet-worker")
        .thread_stack_size(2 * 1024 * 1024)  // 2MB スタック
        .enable_all()
        .build()
}
```

#### 3.2 JSON パースの最適化

```rust
// src/executor/parsing.rs
use simd_json::OwnedValue;

pub fn parse_json_fast(bytes: &mut [u8]) -> Result<OwnedValue, Error> {
    // SIMD JSON パーサーを使用（通常の serde_json より高速）
    simd_json::to_owned_value(bytes)
        .map_err(|e| Error::JsonParseError(e.to_string()))
}
```

### 4. プロファイリングとベンチマーク

```rust
// benches/executor_bench.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_execute_workflow(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let executor = WorkflowExecutor::new(/* ... */);
    let workflow = load_test_workflow();

    c.bench_function("execute_workflow", |b| {
        b.to_async(&runtime).iter(|| async {
            executor.execute(black_box(&workflow)).await
        })
    });
}

criterion_group!(benches, bench_execute_workflow);
criterion_main!(benches);
```

### 5. メトリクスとモニタリング

```rust
// src/metrics/mod.rs
use prometheus::{Counter, Histogram, Registry};

pub struct Metrics {
    pub requests_total: Counter,
    pub request_duration: Histogram,
    pub errors_total: Counter,
}

impl Metrics {
    pub fn new(registry: &Registry) -> Self {
        Self {
            requests_total: Counter::new("requests_total", "Total requests")
                .unwrap(),
            request_duration: Histogram::new("request_duration_seconds", "Request duration")
                .unwrap(),
            errors_total: Counter::new("errors_total", "Total errors")
                .unwrap(),
        }
    }

    pub fn record_request(&self, duration: Duration, success: bool) {
        self.requests_total.inc();
        self.request_duration.observe(duration.as_secs_f64());
        if !success {
            self.errors_total.inc();
        }
    }
}
```

## 成果物

- [ ] `src/executor/adaptive.rs`: 並列度の自動調整
- [ ] `src/executor/streaming.rs`: ストリーミングレスポンス処理
- [ ] `src/executor/threadpool.rs`: カスタムランタイム
- [ ] `src/metrics/mod.rs`: Prometheus メトリクス
- [ ] `benches/`: ベンチマークスイート
- [ ] パフォーマンスレポート

## パフォーマンス目標

| 項目 | 目標 | 現状 |
|------|------|------|
| RPS | 10,000+ | - |
| レイテンシ (P99) | < 100ms | - |
| メモリ使用量 | < 200MB | - |
| CPU 使用率 | < 70% | - |

## ベンチマーク

```bash
# ベンチマークを実行
cargo bench

# プロファイリング
cargo flamegraph --bench executor_bench

# メモリプロファイリング
heaptrack target/release/hornet2 run --arazzo workflow.yaml
```

## 最適化のチェックリスト

- [ ] HTTP 接続プールの設定を調整
- [ ] 並列度を自動調整する機構を実装
- [ ] 大きなレスポンスをストリーミング処理
- [ ] 不要な出力を保存しない
- [ ] SIMD JSON パーサーを導入
- [ ] 専用スレッドプールを使用
- [ ] Prometheus メトリクスを実装
- [ ] ベンチマークを作成
- [ ] プロファイリングでボトルネックを特定

## 参考資料

- [tokio performance tuning](https://tokio.rs/tokio/topics/performance)
- [reqwest performance](https://docs.rs/reqwest/latest/reqwest/#performance)
- [simd-json](https://crates.io/crates/simd-json)
- [criterion benchmarking](https://docs.rs/criterion/)
- [flamegraph profiling](https://github.com/flamegraph-rs/flamegraph)

## 次のステップ

Phase 3 が完了したら、将来的な拡張として：
- SaaS 版の開発
- プラグイン機構の実装
- 分散負荷試験機能
