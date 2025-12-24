# #011 Arazzo Spec Web エディタ機能

**Phase**: 1 (可視化 + エディタ)
**Priority**: High
**Status**: ✅ Phase 1 MVP Completed (2024-12-05)
**Depends on**: #003, #004

## 概要

OpenAPI と Arazzo Specification を活用し、Web UI 上で直感的にワークフローを作成・編集できるエディタ機能を実装する。

**Phase 1 MVP は完了しました！** 🎉
- Operation リスト表示、検索、フィルタリング
- ビジュアルワークフロービュー
- YAML エディタ（Monaco Editor）との双方向同期
- リアルタイムバリデーション
- OAS responses から自動的に適切な status code を選択

詳細な使い方は [EDITOR_GUIDE.md](/EDITOR_GUIDE.md) を参照してください。

## 背景

Arazzo の YAML を手書きするのは学習コストが高く、特に以下の課題がある：

- OpenAPI の operationId を手動で確認しながら書く必要がある
- ステップ間のデータ受け渡し（`$steps.xxx.outputs.yyy`）の記述が煩雑
- スキーマに基づくパラメータの型チェックができない
- OAS 内の links 情報を活用できていない

これらを解決し、**ドキュメント駆動型 API テスト**の UX を向上させる。

## 実装内容

### 1. OAS Operation リストからの選択機能 ✅ 実装済み

**実装済み要件**:
- ✅ OpenAPI から全エンドポイントを抽出し、一覧表示
- ✅ HTTP メソッド、パス、operationId、summary、response_codes を表示
- ✅ 検索・フィルタリング機能（パス、メソッド）
- ✅ クリックでワークフローに追加
- ✅ OAS responses から 2xx 系の最初のコードを自動選択

**未実装（Phase 2以降）**:
- タグによるフィルタリング
- ドラッグ&ドロップでのステップ並び替え

**UI コンポーネント**:
```typescript
interface OperationListProps {
  operations: OpenAPIOperation[];
  onAddToWorkflow: (operation: OpenAPIOperation) => void;
  filter: {
    method?: HttpMethod[];
    tag?: string[];
    searchQuery?: string;
  };
}

interface OpenAPIOperation {
  operationId: string;
  method: HttpMethod;
  path: string;
  summary?: string;
  description?: string;
  parameters?: Parameter[];
  requestBody?: RequestBody;
  responses: Responses;
}
```

**実装例**:
- サイドバーに Operation リストを配置
- メインエリアにワークフローステップをビジュアル表示
- ドラッグ&ドロップでステップを並び替え可能

### 2. 同名パラメータ認識とデータ受け渡しサポート ✅ Phase 2 実装済み

**実装済み要件**:
- ✅ 基本的なステップ間データフロー（`$steps.xxx.outputs.yyy` の記述）
- ✅ 自動的な outputs と successCriteria の生成

**未実装（Phase 2以降）**:
- 前ステップの outputs と後ステップの inputs を自動マッチング
- 同名パラメータのハイライト表示
- サジェスト機能（`$steps.xxx.outputs.` の自動補完）
- 型の互換性チェック

**機能詳細**:

#### 2.1 自動マッチング検出
```typescript
interface DataFlowSuggestion {
  sourceStep: string;        // e.g., "login"
  sourceOutput: string;      // e.g., "token"
  targetStep: string;        // e.g., "getProfile"
  targetInput: string;       // e.g., "Authorization header"
  confidence: 'high' | 'medium' | 'low';  // マッチングの信頼度
  reason: string;            // マッチング理由
}

// 例: "token" という名前のoutputと、Authorizationヘッダーの関連性を検出
function detectDataFlowSuggestions(
  steps: ArazzoStep[]
): DataFlowSuggestion[] {
  // 1. 各ステップのoutputsを収集
  // 2. 次のステップのparameters/requestBodyと名前・型を比較
  // 3. 同名または意味的に関連するものを提案
}
```

#### 2.2 ビジュアルマッピングエディタ
- ステップ間の線でデータフローを表示
- クリックでマッピング編集ダイアログを表示
- ドロップダウンで利用可能な outputs から選択
- JSONPath エディタで複雑な参照をサポート

#### 2.3 インテリジェントサジェスト
```typescript
// エディタで $ を入力したときのサジェスト
const suggestions = [
  { label: '$inputs.username', type: 'string' },
  { label: '$steps.login.outputs.token', type: 'string' },
  { label: '$steps.register.outputs.userId', type: 'string' },
  { label: '$response.body.id', type: 'string' },
  { label: '$statusCode', type: 'number' },
];
```

### 3. OAS links 認識とワークフロー化候補の表示

**要件**:
- OpenAPI の `links` フィールドを解析
- 関連する Operation を自動検出
- ワンクリックでワークフロー生成

**OpenAPI links の例**:
```yaml
# openapi.yaml
paths:
  /users:
    post:
      operationId: createUser
      responses:
        '201':
          links:
            GetUserById:
              operationId: getUser
              parameters:
                userId: $response.body.id
  /users/{userId}:
    get:
      operationId: getUser
```

**UI 機能**:
```typescript
interface WorkflowSuggestion {
  name: string;
  description: string;
  steps: ArazzoStep[];
  source: 'links' | 'common-patterns' | 'ai-generated';
}

// links から自動生成
function generateWorkflowFromLinks(
  openapi: OpenAPISpec
): WorkflowSuggestion[] {
  // 1. links フィールドを持つ Operation を検索
  // 2. リンク先の Operation を解決
  // 3. Arazzo ステップに変換
}
```

**表示例**:
- "Suggested Workflows" セクションを表示
- "User Registration Flow (3 steps)" のようなカード表示
- クリックでワークフローをプレビュー
- "Use this workflow" ボタンで適用

### 4. YAML ライブ確認機能 ✅ 実装済み

**実装済み要件**:
- ✅ ビジュアルエディタと YAML の双方向同期
- ✅ シンタックスハイライト（Monaco Editor）
- ✅ リアルタイムバリデーション（500ms debounce）
- ✅ エラー箇所のハイライトとエラーパネル表示
- ✅ Visual / YAML / Split の3つの表示モード

**実装技術**:
```typescript
// YAML パーサー・シリアライザ
import yaml from 'js-yaml';
import { editor } from 'monaco-editor'; // または CodeMirror

interface YamlEditorProps {
  value: string;
  onChange: (value: string) => void;
  schema?: JSONSchema; // バリデーション用
  readOnly?: boolean;
}

// リアルタイムバリデーション
function validateArazzoYaml(yamlStr: string): ValidationError[] {
  try {
    const parsed = yaml.load(yamlStr);
    return validateArazzoSpec(parsed); // #001 のバリデータを使用
  } catch (e) {
    if (e.mark) {
        return [{ line: e.mark.line, message: e.message }];
    }
    return [{ message: e.message }];
  }
}

```

**UI レイアウト**:
- Split pane: 左側がビジュアルエディタ、右側が YAML エディタ
- トグルボタンで表示切り替え
- 双方向の変更を即座に反映
- エラーがある場合は該当行をハイライト

### 5. その他の有用な機能

#### 5.1 ビジュアルワークフローエディタ
- **ノードベースエディタ**: React Flow / Cytoscape.js を使用
- **ドラッグ&ドロップ**: ステップの追加・並び替え
- **条件分岐の可視化**: successCriteria に基づく分岐表示
- **データフローの可視化**: ステップ間の依存関係を線で表示

#### 5.2 スキーマベースのフォーム生成
```typescript
// OpenAPI スキーマから入力フォームを自動生成
interface StepFormProps {
  operation: OpenAPIOperation;
  step: ArazzoStep;
  onUpdate: (step: ArazzoStep) => void;
}

// requestBody のスキーマからフォームを生成
function generateFormFromSchema(schema: JSONSchema): FormField[] {
  // type, format, enum などから適切な input 要素を生成
}
```

**機能**:
- パラメータの型に応じた入力フォーム（text, number, select, checkbox など）
- `enum` の値をドロップダウンで選択
- `format: email` などのバリデーション
- スキーマの `example` を初期値に設定

#### 5.3 サンプルデータ自動生成
```typescript
// OpenAPI の example または schema から自動生成
function generateSampleData(schema: JSONSchema): any {
  // 1. example フィールドがあればそれを使用
  // 2. なければ type/format から生成
  //    - string → "sample string"
  //    - email → "user@example.com"
  //    - uuid → crypto.randomUUID()
  //    - integer → 123
}
```

#### 5.4 テンプレートライブラリ
```typescript
const templates: WorkflowTemplate[] = [
  {
    name: 'CRUD Operations',
    description: 'Create, Read, Update, Delete flow',
    steps: [/* ... */],
  },
  {
    name: 'Authentication Flow',
    description: 'Register → Login → Access Protected Resource',
    steps: [/* ... */],
  },
  {
    name: 'E-commerce Checkout',
    description: 'Add to cart → Checkout → Payment',
    steps: [/* ... */],
  },
];
```

#### 5.5 プレビュー実行機能
- **Dry Run**: 実際にリクエストを送らずにフロー検証
- **Mock Mode**: モックサーバーを使って実行
- **Live Test**: 実際の API に対して実行
- **ステップごとのブレークポイント**: デバッグ用

#### 5.6 エクスポート/インポート機能
- **YAML エクスポート**: 編集した Arazzo をファイルとして保存
- **k6 スクリプト生成**: #005 の変換機能を UI から実行
- **Postman Collection エクスポート**: 相互運用性
- **JSON/YAML インポート**: 既存ファイルを読み込み

#### 5.7 Undo/Redo 機能
```typescript
// State management with undo/redo
import { create } from 'zustand';
import { temporal } from 'zustand/middleware';

interface EditorStore {
  workflow: ArazzoWorkflow;
  updateWorkflow: (workflow: ArazzoWorkflow) => void;
  undo: () => void;
  redo: () => void;
}

const useEditorStore = create<EditorStore>()(
  temporal((set) => ({
    workflow: initialWorkflow,
    updateWorkflow: (workflow) => set({ workflow }),
  }))
);
```

#### 5.8 AI アシスタント機能
- **自然言語からワークフロー生成**:
  - 入力例: "ユーザーを登録してログインして、プロフィールを更新するフローを作って"
  - → Arazzo YAML を生成
- **ステップの説明生成**: 既存のステップに description を自動追加
- **エラー修正の提案**: バリデーションエラーに対する修正案を提示

#### 5.9 コラボレーション機能（将来的）
- **リアルタイム共同編集**: 複数人で同時編集（WebSocket 使用）
- **コメント機能**: ステップに対するコメント・レビュー
- **バージョン履歴**: Git のようなバージョン管理
- **権限管理**: 閲覧のみ/編集可能などの権限設定

#### 5.10 自動保存とローカルストレージ
```typescript
// 自動保存機能
useEffect(() => {
  const saveTimer = setTimeout(() => {
    localStorage.setItem('arazzo-draft', JSON.stringify(workflow));
  }, 1000); // 1秒後に保存

  return () => clearTimeout(saveTimer);
}, [workflow]);

// リカバリー機能
useEffect(() => {
  const draft = localStorage.getItem('arazzo-draft');
  if (draft && confirm('前回の編集内容を復元しますか？')) {
    setWorkflow(JSON.parse(draft));
  }
}, []);
```

### 6. アーキテクチャ設計

```
┌─────────────────────────────────────────────────────┐
│  Frontend (React + TypeScript)                      │
│                                                      │
│  ┌──────────────────┐  ┌──────────────────────┐    │
│  │ Operation List   │  │ Visual Workflow       │    │
│  │ (Sidebar)        │  │ Editor                │    │
│  │                  │  │ (React Flow)          │    │
│  │ - Filter         │  │                       │    │
│  │ - Search         │  │ - Drag & Drop         │    │
│  │ - Add to flow    │  │ - Data flow lines     │    │
│  └──────────────────┘  └──────────────────────┘    │
│                                                      │
│  ┌──────────────────────────────────────────────┐  │
│  │ YAML Editor (Monaco Editor)                  │  │
│  │ - Syntax highlighting                        │  │
│  │ - Real-time validation                       │  │
│  │ - Auto-completion                            │  │
│  └──────────────────────────────────────────────┘  │
│                                                      │
│  ┌──────────────────┐  ┌──────────────────────┐    │
│  │ Property Panel   │  │ Suggestions Panel    │    │
│  │ - Step config    │  │ - Workflow templates │    │
│  │ - Parameters     │  │ - Links detection    │    │
│  │ - Mappings       │  │ - Data flow hints    │    │
│  └──────────────────┘  └──────────────────────┘    │
└─────────────────────────────────────────────────────┘
                          ↕ REST API
┌─────────────────────────────────────────────────────┐
│  Backend (Rust + axum)                              │
│                                                      │
│  - POST /api/editor/parse-openapi                   │
│  - POST /api/editor/validate-arazzo                 │
│  - POST /api/editor/detect-links                    │
│  - POST /api/editor/suggest-workflow                │
│  - POST /api/editor/convert-to-k6                   │
│  - GET  /api/editor/templates                       │
└─────────────────────────────────────────────────────┘
```

## 成果物

### Backend (Rust) - Phase 1 MVP
- [x] `src/server/api.rs`: エディタ用 API エンドポイント ✅
  - `GET /api/editor/operations`: Operation 一覧取得
  - `POST /api/editor/validate`: Arazzo YAML バリデーション
  - response_codes 抽出機能 ✅
- [ ] `src/editor/links_detector.rs`: OAS links 解析ロジック (Phase 2)
- [ ] `src/editor/workflow_suggester.rs`: ワークフロー提案ロジック (Phase 2)
- [ ] `src/editor/data_flow_analyzer.rs`: データフロー解析 (Phase 2)

### Frontend (React/TypeScript) - Phase 1 MVP
- [x] `ui/src/components/OperationList.tsx`: Operation 一覧コンポーネント ✅
- [x] `ui/src/components/WorkflowView.tsx`: ビジュアルワークフロービュー ✅
- [x] `ui/src/components/YamlEditor.tsx`: YAML エディタコンポーネント ✅
- [x] `ui/src/pages/EditorPage.tsx`: エディタページ統合 ✅
- [x] `ui/src/pages/VisualizationPage.tsx`: 可視化ページ分離 ✅
- [x] `ui/src/stores/editorStore.ts`: エディタの状態管理 (Zustand) ✅
- [x] `ui/src/types/editor.ts`: エディタ型定義 ✅
- [x] `ui/src/App.tsx`: ナビゲーション統合 ✅
- [ ] `ui/src/components/DataFlowMapper.tsx`: データマッピング UI (Phase 2)
- [ ] `ui/src/components/PropertyPanel.tsx`: プロパティ編集パネル (Phase 2)
- [x] `ui/src/components/SuggestionPanel.tsx`: 提案パネル (Phase 2) ✅
- [ ] `ui/src/utils/arazzoGenerator.ts`: YAML 生成ユーティリティ (Phase 2)
- [ ] `ui/src/utils/schemaFormGenerator.ts`: スキーマベースフォーム生成 (Phase 2)

### ドキュメント
- [x] `EDITOR_GUIDE.md`: エディタ使用ガイド ✅
- [x] `README.md`: プロジェクト README 更新 ✅
- [ ] `docs/workflow-patterns.md`: ワークフローパターン集 (Phase 2)

## テストケース

### 機能テスト - Phase 1 MVP
- [x] OpenAPI から Operation を抽出できる ✅
- [x] Operation をワークフローに追加できる ✅
- [x] YAML とビジュアルエディタが同期する ✅
- [x] バリデーションエラーが表示される ✅
- [x] OAS responses から適切な status code が選択される ✅

### 機能テスト - Phase 2以降
- [ ] ステップ間のデータマッピングを設定できる
- [ ] 同名パラメータが自動検出される
- [ ] OAS links からワークフローが生成できる
- [ ] スキーマからフォームが生成される
- [ ] テンプレートを適用できる
- [ ] エクスポート/インポートが動作する
- [ ] Undo/Redo が動作する

### E2E テスト
- [x] ユーザーが Operation を選択してワークフローを作成できる ✅
- [x] YAML を編集してビジュアルエディタに反映される ✅
- [ ] データフローの提案が表示され、適用できる (Phase 2)
- [ ] 作成したワークフローを保存・実行できる (Phase 2)

## 技術スタック

### Frontend (実装済み)
- **Framework**: React 18 + TypeScript ✅
- **YAML Editor**: Monaco Editor (@monaco-editor/react) ✅
- **State Management**: Zustand ✅
- **YAML Parser**: js-yaml ✅
- **スタイル**: インラインCSS（将来的にTailwindやCSS-in-JSへ移行可能）

### Frontend (Phase 2以降)
- **Graph Library**: React Flow または Cytoscape.js
- **Form**: React Hook Form + Zod
- **UI Components**: shadcn/ui または MUI

### Backend
- **Web Framework**: axum
- **YAML Parser**: serde_yaml
- **JSON Schema**: jsonschema-rs
- **OpenAPI Parser**: 既存の #001 実装を活用

## 開発フェーズ

### Phase 1: 基本機能 (MVP) ✅ 完了
- [x] Operation リスト表示（検索・フィルタ）
- [x] ビジュアルエディタ基本機能（WorkflowView）
- [x] YAML エディタと同期（Monaco Editor）
- [x] 基本的なバリデーション（リアルタイム、エラー表示）
- [x] OAS responses からの status code 自動選択
- [x] 3つの表示モード（Visual / YAML / Split）
- [x] タブナビゲーション（Visualization / Editor）

### Phase 2: 高度な機能
- データフローの自動検出
- OAS links サポート
- サンプルデータ生成
- テンプレートライブラリ

### Phase 3: UX 向上
- AI アシスタント
- プレビュー実行
- Undo/Redo
- 自動保存

### Phase 4: コラボレーション（将来的）
- リアルタイム共同編集
- バージョン管理
- コメント機能

## 参考資料

### OpenAPI & Arazzo
- [Arazzo Specification](https://spec.openapis.org/arazzo/latest.html)
- [OpenAPI Specification - Links](https://swagger.io/docs/specification/links/)

### UI ライブラリ
- [React Flow](https://reactflow.dev/)
- [Monaco Editor](https://microsoft.github.io/monaco-editor/)
- [React Hook Form](https://react-hook-form.com/)
- [Zod](https://zod.dev/)

### 参考実装
- [Postman](https://www.postman.com/) - API ワークフロー管理
- [Stoplight Studio](https://stoplight.io/studio) - OpenAPI エディタ
- [Swagger Editor](https://editor.swagger.io/) - OpenAPI エディタ
- [Insomnia](https://insomnia.rest/) - API クライアント

## 現在の実装状況 (2024-12-05)

### ✅ Phase 1 MVP 完了

**実装されたAPI**:
- `GET /api/editor/operations` - OpenAPI から全 Operation を抽出
- `POST /api/editor/validate` - Arazzo YAML のバリデーション

**実装されたコンポーネント**:
- `OperationList` - 検索、HTTPメソッドフィルタ、"+ Add"ボタン
- `WorkflowView` - ステップの番号付き表示、parameters/successCriteria 表示
- `YamlEditor` - Monaco Editor統合、エラーマーカー、リアルタイムバリデーション
- `EditorPage` - 3つの表示モード切り替え（Visual/YAML/Split）

**実装された機能**:
1. OpenAPI Operations の抽出とフィルタリング
2. クリックでワークフローにステップ追加
3. OAS responses から 2xx 系の最初のコードを自動選択
4. Visual ↔ YAML の双方向同期（500ms debounce）
5. リアルタイムバリデーションとエラー表示
6. Visualization / Editor のタブナビゲーション

**使い方**:
```bash
make dev
# http://localhost:5173 を開き、「Editor」タブへ
```

詳細は [EDITOR_GUIDE.md](/EDITOR_GUIDE.md) を参照。

## 次のステップ (Phase 2以降)

### 優先度高
- OAS links 認識とワークフロー提案
- ドラッグ&ドロップでのステップ並び替え

### 優先度中
- テンプレートライブラリ
- プレビュー実行機能
- k6/Postman へのエクスポート

### 優先度低
- AI アシスタント機能
- Undo/Redo
- コラボレーション機能
- データフロー自動検出とサジェスト (実装済み)

### 関連タスク
- **#005 k6 DSL 変換**: エディタから k6 スクリプト生成
- **#006 テスト実行の自動化**: エディタから直接テスト実行
- **#007 結果レポート生成**: 実行結果をビジュアル表示

このエディタが **「ドキュメント = テスト」** という本プロジェクトのコアコンセプトを具現化する重要な UX となっています。
