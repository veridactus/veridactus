# VERIDACTUS UI

**AI Execution Governance Dashboard** — A React-based web interface for managing and monitoring the VERIDACTUS trusted AI execution platform.

---

## Overview

veridactus-ui is the frontend component of the VERIDACTUS project, providing:

- **Pipeline Designer**: Visual drag-and-drop pipeline editor for governance plugin chains
- **Trace Explorer**: Real-time execution trace search and inspection
- **Model Router**: Configure model upstreams, API keys, and failover policies
- **API Key Manager**: Tenant-scoped API key generation and revocation
- **Plugin Library**: Browse and enable built-in governance plugins
- **Health Dashboard**: Monitor data plane and control plane status

---

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Framework | React 18 |
| Language | TypeScript 5.5 |
| Build Tool | Vite 5.4 |
| Styling | TailwindCSS 3.4 |
| Flow Editor | ReactFlow 11.11 |
| Icons | Lucide React |
| Animation | Framer Motion |
| Testing | Vitest + Testing Library |

---

## Quick Start

```bash
# Install dependencies
npm ci

# Start development server (default: http://localhost:5173)
npm run dev

# Type-check
npx tsc --noEmit

# Build for production
npm run build

# Preview production build
npm run preview

# Run tests
npm test
```

---

## Architecture

```
veridactus-ui/
├── src/
│   ├── api/           # API client for Control Plane REST endpoints
│   ├── components/    # Reusable UI components
│   ├── pages/         # Route-level page components
│   ├── hooks/         # Custom React hooks
│   ├── types/         # TypeScript type definitions
│   └── utils/         # Utility functions
├── public/            # Static assets
└── tests/             # Test files
```

The UI communicates exclusively with the **VERIDACTUS Control Plane** (port 8081) via REST API calls.

---

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `VITE_CP_URL` | `http://localhost:8081` | Control Plane base URL |
| `VITE_ADMIN_KEY` | (none) | Admin API key for management endpoints |

---

## License

Apache 2.0 — see the [LICENSE](../LICENSE) file in the monorepo root.

---
*Part of the [VERIDACTUS](https://github.com/veridactus/veridactus) project.*
