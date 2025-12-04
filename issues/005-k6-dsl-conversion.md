# #005 外部ツール (k6) への DSL 変換

**Phase**: 2 (テスト実行)
**Priority**: Medium
**Status**: Done
**Depends on**: #001, #002
**Completed**: 2024-12-04

## 概要

Arazzo ワークフローから k6 スクリプト（JavaScript）を自動生成し、既存の k6 エンジンでテストを実行できるようにする。

## 背景

Phase 1 では可視化のみを実装したが、Phase 2 では実際に API テストを実行する必要がある。
自前のテスト実行エンジンを作る前に、まずは k6 のような成熟したツールを活用することで、早期に価値を提供できる。

## 実装内容

### 1. k6 スクリプトの構造

```javascript
// 生成する k6 スクリプトの例
import http from 'k6/http';
import { check, sleep } from 'k6';

export let options = {
  vus: 1,
  iterations: 1,
};

export default function () {
  // Step 1: ユーザー登録
  let registerRes = http.post('https://api.example.com/register', JSON.stringify({
    username: 'testuser',
    email: 'test@example.com',
  }), {
    headers: { 'Content-Type': 'application/json' },
  });
  check(registerRes, { 'register success': (r) => r.status === 201 });

  // Step 2: ログイン
  let loginRes = http.post('https://api.example.com/login', JSON.stringify({
    username: 'testuser',
    password: 'password123',
  }), {
    headers: { 'Content-Type': 'application/json' },
  });
  check(loginRes, { 'login success': (r) => r.status === 200 });
  let token = loginRes.json('token');

  // Step 3: プロフィール取得
  let profileRes = http.get('https://api.example.com/profile', {
    headers: { 'Authorization': `Bearer ${token}` },
  });
  check(profileRes, { 'profile fetch success': (r) => r.status === 200 });
}
```

### 2. 変換ロジック

```rust
// src/converters/k6.rs
pub struct K6Converter {
    openapi: OpenApiSpec,
    arazzo: ArazzoSpec,
}

impl K6Converter {
    pub fn convert(&self, workflow: &Workflow) -> Result<String, Error> {
        // 1. ワークフローのステップを走査
        // 2. 各ステップを k6 の http リクエストに変換
        // 3. successCriteria を check() に変換
        // 4. outputs の参照を JavaScript 変数に変換
    }
}
```

### 3. データ参照の変換

Arazzo の `$steps.login.outputs.token` を k6 の変数参照に変換：

```yaml
# Arazzo
steps:
  - stepId: login
    outputs:
      token: $response.body.token
  - stepId: getProfile
    parameters:
      - name: Authorization
        in: header
        value: $steps.login.outputs.token
```

↓

```javascript
// k6
let loginRes = http.post(...);
let token = loginRes.json('token');

let profileRes = http.get(..., {
  headers: { 'Authorization': `Bearer ${token}` },
});
```

### 4. 実行オプションの変換

```yaml
# Arazzo (将来的な拡張)
x-load-test:
  vus: 10
  duration: 30s
  rampUp: 5s
```

↓

```javascript
// k6
export let options = {
  vus: 10,
  duration: '30s',
  rampUp: '5s',
};
```

### 5. k6 の実行

```rust
// src/runner/k6.rs
use std::process::Command;

pub fn run_k6_script(script_path: &Path) -> Result<TestResult, Error> {
    let output = Command::new("k6")
        .arg("run")
        .arg(script_path)
        .output()?;

    // k6 の出力をパースして結果を返す
    parse_k6_output(&output.stdout)
}
```

## 成果物

- [ ] `src/converters/k6.rs`: k6 スクリプト生成ロジック
- [ ] `src/converters/mod.rs`: 変換器のトレイト定義
- [ ] `src/runner/k6.rs`: k6 実行ロジック
- [ ] `tests/fixtures/k6/`: 生成される k6 スクリプトのサンプル
- [ ] CLI コマンド: `hornet2 convert --to k6`

## テストケース

- 単純なワークフローを k6 スクリプトに変換できる
- データ依存のあるステップを変数参照に変換できる
- `successCriteria` を `check()` に変換できる
- 生成されたスクリプトが k6 で実行可能
- k6 の実行結果をパースして表示できる

## CLI コマンド

```bash
# k6 スクリプトを生成
hornet2 convert --arazzo workflow.yaml --openapi openapi.yaml --to k6 --output script.js

# 生成したスクリプトを k6 で実行
hornet2 run --arazzo workflow.yaml --openapi openapi.yaml --engine k6

# または直接 k6 を実行
k6 run script.js
```

## 制約事項

- k6 がインストールされている必要がある
- k6 でサポートされない Arazzo の機能は警告を出す
- 複雑な条件分岐は JavaScript の `if` 文に変換

## 参考資料

- [k6 documentation](https://k6.io/docs/)
- [k6 HTTP requests](https://k6.io/docs/using-k6/http-requests/)
- [k6 checks](https://k6.io/docs/using-k6/checks/)

## 次のステップ

このタスクが完了したら、**#006 テスト実行の自動化** に進む。
将来的には **#007 Rust 製 HTTP クライアントの実装** で自前エンジン化を目指す。
