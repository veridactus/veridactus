# Python Worker for VERIDACTUS

**VERIDACTUS Python Worker** — Background task processing for AI governance operations, including semantic drift detection, differential privacy guarantees, and PII scanning.

---

## Overview

The Python Worker is a FastAPI-based microservice that handles compute-intensive governance tasks asynchronously. It integrates with the VERIDACTUS Data Plane via Redis Streams.

### Capabilities

| Worker | Description | Status |
|--------|-------------|--------|
| **Drift Detector** | Embedding-based semantic consistency analysis using cosine similarity | 🟢 Active |
| **DP Guarantee** | Differential privacy budget computation (ε, δ) | 🟢 Active |
| **PII Scanner** | Pattern-based sensitive data detection and masking | 🟢 Active |

---

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Framework | FastAPI |
| Server | Uvicorn |
| Message Queue | Redis Streams |
| ML/Numeric | NumPy, scikit-learn |
| HTTP Client | httpx |

---

## Quick Start

```bash
# Create virtual environment
python3 -m venv .venv
source .venv/bin/activate

# Install dependencies
pip install -r requirements.txt

# Start the worker
uvicorn app.main:app --host 0.0.0.0 --port 8002 --reload

# Health check
curl http://localhost:8002/health
```

---

## Architecture

```
Data Plane (Rust) ──Redis Streams──▶ Python Worker
                                       ├── drift-detection
                                       ├── compute-guarantee
                                       └── pii-detection
```

The worker consumes tasks from Redis Streams published by the Rust data plane and returns results via the same channel.

---

## API Endpoints

| Method | Path | Description |
|--------|------|-------------|
| GET | `/health` | Service health check |
| POST | `/compute-guarantee` | Compute differential privacy guarantee |
| POST | `/drift-detection` | Run semantic drift analysis |
| POST | `/pii-detection` | Scan text for PII patterns |

---

## License

Apache 2.0 — see the [LICENSE](../LICENSE) file in the monorepo root.

---

*Part of the [VERIDACTUS](https://github.com/veridactus/veridactus) project.*
