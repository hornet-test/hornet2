# #007 結果レポートの生成

**Phase**: 2 (テスト実行)
**Priority**: Low
**Status**: Todo
**Depends on**: #006

## 概要

テスト実行結果を HTML レポートとして出力し、ブラウザで視覚的に確認できるようにする。

## 背景

JSON 形式の結果だけでは、エンジニア以外のステークホルダーに共有しづらい。
HTML レポートを生成することで、テスト結果を誰でも理解しやすくする。

## 実装内容

### 1. レポートの構成

```
┌────────────────────────────────────┐
│  Test Report                       │
│  ================================  │
│  Workflow: login-test              │
│  Status: ✅ Passed                 │
│  Duration: 1.23s                   │
│  ────────────────────────────────  │
│  Summary                           │
│  - Total Requests: 10              │
│  - Passed: 10                      │
│  - Failed: 0                       │
│  - Avg Response Time: 120ms        │
│  ────────────────────────────────  │
│  Steps                             │
│  1. ✅ register (POST /register)   │
│     - Status: 201                  │
│     - Duration: 150ms              │
│  2. ✅ login (POST /login)         │
│     - Status: 200                  │
│     - Duration: 100ms              │
│  ────────────────────────────────  │
│  Response Time Chart               │
│  [Bar Chart]                       │
└────────────────────────────────────┘
```

### 2. テンプレートエンジンの選定

- **tera**: Jinja2 風のテンプレートエンジン
  - Pros: 柔軟、学習コストが低い
- **askama**: コンパイル時にテンプレートを検証
  - Pros: 型安全、高速
- **handlebars**: Handlebars.js 互換
- **手動生成**: `format!` で HTML を生成

**決定**: `askama` を採用（型安全で Rust らしい）

### 3. HTML テンプレート

```rust
// src/report/template.rs
use askama::Template;

#[derive(Template)]
#[template(path = "report.html")]
pub struct ReportTemplate {
    pub workflow_id: String,
    pub status: TestStatus,
    pub duration_ms: u64,
    pub summary: TestSummary,
    pub steps: Vec<StepResult>,
    pub chart_data: ChartData,
}
```

```html
<!-- templates/report.html -->
<!DOCTYPE html>
<html>
<head>
  <title>Test Report - {{ workflow_id }}</title>
  <link rel="stylesheet" href="style.css">
</head>
<body>
  <h1>Test Report: {{ workflow_id }}</h1>
  <div class="status {{ status }}">
    Status: {{ status }}
  </div>
  <div class="summary">
    <h2>Summary</h2>
    <ul>
      <li>Total Requests: {{ summary.total_requests }}</li>
      <li>Passed: {{ summary.passed_requests }}</li>
      <li>Failed: {{ summary.failed_requests }}</li>
      <li>Avg Response Time: {{ summary.avg_response_time_ms }}ms</li>
    </ul>
  </div>
  <div class="steps">
    <h2>Steps</h2>
    {% for step in steps %}
    <div class="step {{ step.status }}">
      <h3>{{ step.step_id }}</h3>
      <p>Status: {{ step.http_status }}</p>
      <p>Duration: {{ step.duration_ms }}ms</p>
    </div>
    {% endfor %}
  </div>
  <div class="chart">
    <h2>Response Time</h2>
    <canvas id="responseTimeChart"></canvas>
  </div>
  <script src="https://cdn.jsdelivr.net/npm/chart.js"></script>
  <script>
    const ctx = document.getElementById('responseTimeChart');
    new Chart(ctx, {
      type: 'bar',
      data: {{ chart_data | json }}
    });
  </script>
</body>
</html>
```

### 4. チャートの生成

Chart.js を使ってレスポンスタイムをグラフ化：

```rust
// src/report/chart.rs
#[derive(Serialize)]
pub struct ChartData {
    pub labels: Vec<String>,
    pub datasets: Vec<Dataset>,
}

#[derive(Serialize)]
pub struct Dataset {
    pub label: String,
    pub data: Vec<f64>,
    pub background_color: String,
}

pub fn generate_chart_data(steps: &[StepResult]) -> ChartData {
    ChartData {
        labels: steps.iter().map(|s| s.step_id.clone()).collect(),
        datasets: vec![
            Dataset {
                label: "Response Time (ms)".to_string(),
                data: steps.iter().map(|s| s.duration_ms as f64).collect(),
                background_color: "rgba(75, 192, 192, 0.2)".to_string(),
            }
        ],
    }
}
```

### 5. CSS スタイル

```css
/* templates/style.css */
body {
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
  max-width: 1200px;
  margin: 0 auto;
  padding: 20px;
}

.status.Passed {
  color: green;
  font-weight: bold;
}

.status.Failed {
  color: red;
  font-weight: bold;
}

.step {
  border: 1px solid #ddd;
  padding: 10px;
  margin: 10px 0;
  border-radius: 5px;
}

.step.Passed {
  border-left: 4px solid green;
}

.step.Failed {
  border-left: 4px solid red;
}
```

### 6. CLI コマンド

```bash
# レポートを HTML で生成
hornet2 report --input results.json --output report.html

# テスト実行と同時にレポート生成
hornet2 run --arazzo workflow.yaml --report report.html

# レポートをブラウザで開く
hornet2 report --input results.json --open
```

## 成果物

- [ ] `src/report/mod.rs`: レポート生成のエントリーポイント
- [ ] `src/report/template.rs`: Askama テンプレート
- [ ] `src/report/chart.rs`: チャートデータの生成
- [ ] `templates/report.html`: HTML テンプレート
- [ ] `templates/style.css`: CSS スタイル
- [ ] `src/commands/report.rs`: report コマンドの実装

## テストケース

- JSON 結果から HTML レポートが生成できる
- ブラウザで HTML が正しく表示される
- チャートが正しく描画される
- ステップごとのステータスが色分けされる
- 失敗したステップのエラーメッセージが表示される

## 拡張案

- PDF エクスポート機能
- Markdown レポート
- Slack/Discord への通知
- 過去のレポートとの比較機能

## 参考資料

- [askama documentation](https://docs.rs/askama/)
- [Chart.js](https://www.chartjs.org/)

## 次のステップ

Phase 2 が完了したら、**Phase 3: 高速エンジン化** に進む。
次は **#008 Rust 製 HTTP クライアントの実装** を検討。
