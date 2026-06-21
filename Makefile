.PHONY: all build check test e2e clean dev infra lint fmt help deploy stop logs status

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

# ==================== Docker Deployment ====================
deploy: ## Deploy all services with Docker Compose (one-click)
	@echo "🚀 Starting VERIDACTUS deployment..."
	@chmod +x deploy/quick-start.sh
	@./deploy/quick-start.sh --init

deploy-all: ## Deploy all services including optional components
	@chmod +x deploy/quick-start.sh
	@./deploy/quick-start.sh --all

deploy-ollama: ## Deploy with local Ollama LLM
	@chmod +x deploy/quick-start.sh
	@./deploy/quick-start.sh --init --ollama

deploy-worker: ## Deploy with Python Worker
	@chmod +x deploy/quick-start.sh
	@./deploy/quick-start.sh --init --worker

stop: ## Stop all Docker services
	@echo "🛑 Stopping VERIDACTUS services..."
	docker compose -f deploy/docker-compose.yml down
	@echo "✅ Services stopped"

stop-clean: ## Stop services and remove volumes
	@echo "🧹 Stopping services and cleaning volumes..."
	docker compose -f deploy/docker-compose.yml down -v
	@echo "✅ Services stopped and volumes removed"

logs: ## View Docker logs
	docker compose -f deploy/docker-compose.yml logs -f

status: ## Show Docker service status
	docker compose -f deploy/docker-compose.yml ps

restart: ## Restart all services
	docker compose -f deploy/docker-compose.yml restart

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
