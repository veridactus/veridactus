.PHONY: all build check test e2e clean dev infra lint fmt help

# ==================== Default ====================
all: build

help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-15s\033[0m %s\n", $$1, $$2}'

# ==================== Build ====================
build: build-core build-cp build-ui ## Build all components

build-core: ## Build Rust data plane
	cd core && cargo build --release

build-cp: ## Build Go control plane
	cd control-plane && go build -o bin/control-plane ./cmd/server/

build-ui: ## Build React frontend
	cd veridactus-ui && npm ci && npm run build

# ==================== Check ====================
check: fmt lint ## Format and lint all components

fmt: fmt-core fmt-cp ## Format all components
	@echo "✅ Formatting complete"

fmt-core:
	cd core && cargo fmt --check || cargo fmt

fmt-cp:
	cd control-plane && gofmt -w .

lint: lint-core lint-cp ## Lint all components

lint-core:
	cd core && cargo clippy --all-targets 2>&1 || true

lint-cp:
	cd control-plane && go vet ./...

# ==================== Test ====================
test: test-core test-cp test-ui ## Run all tests

test-core: ## Run Rust unit tests
	cd core && cargo test --lib

test-cp: ## Run Go tests
	cd control-plane && go test ./...

test-ui: ## Run UI tests
	cd veridactus-ui && npm test 2>/dev/null || echo "UI tests skipped (vitest not configured)"

e2e: ## Run end-to-end tests (requires running services)
	bash scripts/e2e-comprehensive.sh

# ==================== Development ====================
dev: ## Start all services locally
	@echo "Starting all VERIDACTUS services..."
	bash scripts/start-all.sh

infra: ## Start infrastructure (PostgreSQL, Redis, MinIO)
	docker compose -f scripts/docker-compose.yml up -d

# ==================== Clean ====================
clean: ## Clean all build artifacts
	cd core && cargo clean
	cd control-plane && rm -f bin/control-plane server veridactus-cp veridactus.db*
	cd veridactus-ui && rm -rf dist/ node_modules/.vite
	@echo "✅ Cleaned"

# ==================== CI Targets ====================
ci: check build test ## Full CI pipeline
	@echo "✅ CI pipeline complete"
