.PHONY: help install build dev test clean ui-dev cli-dev ui-build ui-test cli-build cli-test stop-dev ui-lint ui-format ui-typecheck lint

# デフォルトターゲット
.DEFAULT_GOAL := help

# カラー出力用
BLUE := \033[0;34m
GREEN := \033[0;32m
YELLOW := \033[0;33m
NC := \033[0m # No Color

# テストファイル（必要に応じて変更）
TEST_ARAZZO := tests/fixtures/arazzo.yaml
TEST_OPENAPI := tests/fixtures/openapi.yaml

help: ## このヘルプメッセージを表示
	@echo "$(BLUE)hornet2 - Makefile コマンド$(NC)"
	@echo ""
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "  $(GREEN)%-15s$(NC) %s\n", $$1, $$2}'
	@echo ""

install: ## 依存関係をすべてインストール
	@echo "$(BLUE)Installing Rust dependencies...$(NC)"
	@cargo fetch
	@echo "$(BLUE)Installing UI dependencies...$(NC)"
	@cd ui && pnpm install
	@echo "$(GREEN)✓ All dependencies installed$(NC)"

build: cli-build ui-build ## CLIとUIを両方ビルド
	@echo "$(GREEN)✓ Build complete$(NC)"

cli-build: ## Rust CLIをビルド
	@echo "$(BLUE)Building Rust CLI...$(NC)"
	@cargo build --release
	@echo "$(GREEN)✓ CLI built: target/release/hornet2$(NC)"

ui-build: ## UIをビルド
	@echo "$(BLUE)Building UI...$(NC)"
	@cd ui && pnpm build
	@echo "$(GREEN)✓ UI built: ui/dist/$(NC)"

ui-lint: ## UIのLintを実行
	@echo "$(BLUE)Running UI lint...$(NC)"
	@cd ui && pnpm lint
	@echo "$(GREEN)✓ UI lint passed$(NC)"

ui-typecheck: ## UIの型チェックを実行
	@echo "$(BLUE)Running UI typecheck...$(NC)"
	@cd ui && pnpm typecheck
	@echo "$(GREEN)✓ UI typecheck passed$(NC)"

ui-format: ## UIのコードを整形
	@echo "$(BLUE)Formatting UI code...$(NC)"
	@cd ui && pnpm format
	@echo "$(GREEN)✓ UI formatted$(NC)"

lint: ui-lint ## すべてのLintを実行
	@echo "$(GREEN)✓ All lint checks passed$(NC)"

dev: ## 開発モード: CLIサーバーとUIを同時起動（Ctrl+Cで両方停止）
	@echo "$(BLUE)Starting development servers...$(NC)"
	@echo "$(YELLOW)CLI Server: http://localhost:3000$(NC)"
	@echo "$(YELLOW)UI Dev Server: http://localhost:5173$(NC)"
	@echo ""
	@echo "$(YELLOW)Press Ctrl+C to stop all servers$(NC)"
	@echo ""
	@trap 'kill 0' EXIT; \
		(cd ui && pnpm dev) & \
		cargo run -- serve --arazzo $(TEST_ARAZZO) --openapi $(TEST_OPENAPI) --port 3000

cli-dev: ## CLIサーバーのみ起動
	@echo "$(BLUE)Starting CLI server on http://localhost:3000$(NC)"
	@cargo run -- serve --arazzo $(TEST_ARAZZO) --openapi $(TEST_OPENAPI) --port 3000

ui-dev: ## UI開発サーバーのみ起動
	@echo "$(BLUE)Starting UI dev server on http://localhost:5173$(NC)"
	@cd ui && pnpm dev

test: cli-test ui-test ## すべてのテストを実行
	@echo "$(GREEN)✓ All tests passed$(NC)"

cli-test: ## Rustのテストを実行
	@echo "$(BLUE)Running Rust tests...$(NC)"
	@cargo test

ui-test: ## UIのテストを実行
	@echo "$(BLUE)Running UI tests...$(NC)"
	@cd ui && pnpm test -- --run

ui-test-watch: ## UIのテストをwatchモードで実行
	@cd ui && pnpm test

clean: ## ビルド成果物をクリーンアップ
	@echo "$(BLUE)Cleaning build artifacts...$(NC)"
	@cargo clean
	@rm -rf ui/dist
	@echo "$(GREEN)✓ Clean complete$(NC)"

check: ## コードのチェック（フォーマット・lint）
	@echo "$(BLUE)Checking Rust code...$(NC)"
	@cargo fmt --check
	@cargo clippy -- -D warnings
	@echo "$(BLUE)Checking UI code...$(NC)"
	@cd ui && pnpm typecheck
	@cd ui && pnpm lint
	@echo "$(GREEN)✓ Code check passed$(NC)"

fmt: ## コードをフォーマット
	@echo "$(BLUE)Formatting code...$(NC)"
	@cargo fmt
	@cd ui && pnpm format
	@echo "$(GREEN)✓ Code formatted$(NC)"

run: ## CLIを実行（引数: ARGS="..."）
	@cargo run -- $(ARGS)

# 例: make visualize ARGS="--format json"
visualize: ## フロー図を生成（引数: ARGS="--format json"）
	@cargo run -- visualize --arazzo $(TEST_ARAZZO) --openapi $(TEST_OPENAPI) $(ARGS)

validate: ## OpenAPI/Arazzoを検証
	@echo "$(BLUE)Validating OpenAPI and Arazzo...$(NC)"
	@cargo run -- validate --openapi $(TEST_OPENAPI) --arazzo $(TEST_ARAZZO)
	@echo "$(GREEN)✓ Validation passed$(NC)"
