# VERIDACTUS Control Plane (Go)

[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](../LICENSE)
[![Go](https://img.shields.io/badge/Go-1.21%2B-00ADD8)](https://go.dev/)

REST API server for managing VERIDACTUS configuration: pipelines, plugins, policies, API keys, model routes, and data plane configurations. Uses SQLite for persistent storage.

## Features

- **Pipeline CRUD** — Create, read, update, delete governance pipelines
- **Plugin Registry** — Register and manage plugins
- **Policy Management** — YAML-based governance policies
- **API Key Management** — Random key generation, rotation, revocation
- **Model Route Management** — Multi-model routing with auth configuration
- **Configuration Push** — Auto-push config changes to data plane
- **Config Poll** — Long-polling endpoint for data plane config sync

## Quick Start

```bash
# Build
go build -o bin/control-plane ./cmd/server/

# Run
VERIDACTUS_ADMIN_KEY="your-admin-key" ./bin/control-plane
# API available at http://localhost:8081

# Or with custom port
PORT=9090 ./bin/control-plane
```

## Environment Variables

| Variable | Default | Description |
|:---|:---|:---|
| `VERIDACTUS_ADMIN_KEY` | *(unprotected)* | Admin key for management API |
| `VERIDACTUS_CORS_ORIGIN` | `http://localhost:3000` | CORS allowed origin |
| `PORT` | `8081` | Server port |
| `DB_PATH` | `./veridactus.db` | SQLite database path |
| `VERIDACTUS_DEFAULT_UPSTREAM_URL` | `http://localhost:11434` | Default upstream LLM URL |
| `VERIDACTUS_GEMINI_API_KEY` | — | Google Gemini API key |
| `VERIDACTUS_AZURE_AI_API_KEY` | — | Azure AI API key |
| `VERIDACTUS_PROXY_URL` | — | HTTP proxy for upstream calls |

## API Endpoints

| Endpoint | Method | Auth | Description |
|:---|:---|:---|:---|
| `/api/v1/health` | GET | — | Health check |
| `/api/v1/pipelines` | GET/POST | Admin | Pipeline CRUD |
| `/api/v1/pipelines/:id` | GET/PUT/DELETE | Admin | Single pipeline |
| `/api/v1/plugins` | GET/POST | Admin | Plugin registry |
| `/api/v1/policies` | GET/POST | Admin | Policy CRUD |
| `/api/v1/apikeys` | GET/POST | Admin | API key CRUD |
| `/api/v1/apikeys/:id` | GET/PUT/DELETE | Admin | Single API key |
| `/api/v1/models` | GET/POST | Admin | Model route CRUD |
| `/api/v1/models/:id` | GET/PUT/DELETE | Admin | Single model route |
| `/api/v1/dataplane-configs` | GET/POST | Admin | Data plane config |
| `/api/v1/config/poll` | GET | — | Config long-poll (for data plane) |

All endpoints except `/health` and `/config/poll` require `X-Admin-Key` header.

## Database

SQLite with WAL mode enabled. Tables are auto-created on first run:
- `pipelines` — Governance pipeline definitions
- `plugins` — Plugin metadata
- `policies` — Governance policies
- `apikeys` — API keys for data plane auth
- `models` — Model routing configuration
- `traces` — Trace metadata (reference only)
- `config_versions` — Configuration version tracking
- `data_plane_configs` — Data plane storage config
