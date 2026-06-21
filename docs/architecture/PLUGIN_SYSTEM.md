# VERIDACTUS Plugin System

This document describes the plugin architecture, development guide, and runtime behavior for VERIDACTUS plugins.

## Table of Contents

1. [Overview](#overview)
2. [Plugin Types](#plugin-types)
3. [Plugin Interface](#plugin-interface)
4. [Plugin Lifecycle](#plugin-lifecycle)
5. [Development Guide](#development-guide)
6. [Configuration](#configuration)
7. [Best Practices](#best-practices)

---

## Overview

VERIDACTUS implements a **three-tier plugin architecture** that supports different latency/isolation tradeoffs:

```
┌─────────────────────────────────────────────────────────────────────────┐
│                      Plugin Architecture                                 │
└─────────────────────────────────────────────────────────────────────────┘

  Native Plugins (<10μs)          WASM Plugins (50-200μs)       gRPC Plugins (5-500ms)
  ┌─────────────────┐            ┌─────────────────┐        ┌─────────────────┐
  │ Compiled into   │            │ Sandboxed WASM   │        │ External HTTP   │
  │ Rust binary     │            │ runtime          │        │ service         │
  │                 │            │                 │        │                 │
  │ Direct function │            │ WASMer/WASMTime │        │ Python Worker   │
  │ calls           │            │ engine          │        │ or custom       │
  │                 │            │                 │        │ service         │
  └────────┬────────┘            └────────┬────────┘        └────────┬────────┘
           │                              │                         │
           └──────────────────────────────┴─────────────────────────┘
                                        │
                                        ▼
                        ┌───────────────────────────────┐
                        │    Plugin Registry             │
                        │   (Runtime container)          │
                        └───────────────────────────────┘
                                        │
                                        ▼
                        ┌───────────────────────────────┐
                        │    Pipeline Executor           │
                        │    (Orchestrates execution)   │
                        └───────────────────────────────┘
```

---

## Plugin Types

### Native Plugins

**Characteristics:**
- Compiled directly into the `veridactus-core` binary
- Zero serialization overhead
- Direct memory access
- Cannot be hot-swapped without restart

**Use Cases:**
- Budget control (high-frequency checks)
- Authentication validation
- Core routing logic

**Example Native Plugins:**
| Plugin | Stage | Description |
|--------|-------|-------------|
| `BudgetGuardPlugin` | pre_request | Budget enforcement |
| `AuthValidatorPlugin` | pre_request | API key validation |
| `RouteSelectorPlugin` | pre_request | Model routing |
| `TraceFinalizerPlugin` | post_response | L0 signature computation |
| `ResponseValidatorPlugin` | post_response | Format validation |

### WASM Plugins

**Characteristics:**
- Sandboxed execution environment
- Hot-swappable without restart
- Limited system access
- Cross-platform compatibility

**Use Cases:**
- Content filtering rules
- Custom PII patterns
- Third-party security plugins

**Runtime Dependencies:**
```toml
# Cargo.toml
[dependencies]
wasmer = "4.0"
```

**Example WASM Plugins:**
| Plugin | Stage | Description |
|--------|-------|-------------|
| `KeywordGuardrail` | streaming | Real-time keyword filtering |
| `PiiMasking` | streaming | Regex-based PII masking |

### gRPC Plugins

**Characteristics:**
- External service communication
- Full language flexibility
- Network latency overhead
- Independent scaling

**Use Cases:**
- ML-based content analysis
- Complex compliance computation
- Integration with existing services

**Example gRPC Plugins:**
| Plugin | Stage | Description |
|--------|-------|-------------|
| `DriftDetector` | async | Semantic drift detection |
| `C-SafeGen` | async | Conformal guarantee computation |
| `TEEAttestation` | async | Hardware proof generation |

---

## Plugin Interface

All plugins must implement the `GovernancePlugin` trait:

```rust
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[async_trait]
pub trait GovernancePlugin: Send + Sync {
    /// Plugin metadata
    fn metadata(&self) -> PluginMetadata;
    
    /// Supported protocol versions
    fn supported_protocol_versions(&self) -> VersionRange {
        VersionRange::兼容旧版本
    }
    
    /// Required runtime capabilities
    fn required_capabilities(&self) -> Vec<String> {
        vec![]
    }
    
    /// Pre-request execution (synchronous)
    async fn on_request(
        &self,
        ctx: &mut RequestContext,
        journal: &mut ExecutionJournal,
    ) -> Result<Action, PluginError>;
    
    /// Streaming execution (per chunk)
    async fn on_stream_chunk(
        &self,
        ctx: &mut StreamChunkContext,
        journal: &mut ExecutionJournal,
    ) -> Result<Action, PluginError>;
    
    /// Post-response execution (synchronous)
    async fn on_response(
        &self,
        ctx: &mut ResponseContext,
        journal: &mut ExecutionJournal,
    ) -> Result<Action, PluginError>;
    
    /// Async execution (background)
    async fn on_async(
        &self,
        ctx: &mut AsyncContext,
        journal: &mut ExecutionJournal,
    ) -> Result<Action, PluginError>;
}
```

### Action Types

```rust
pub enum Action {
    /// Continue to next plugin
    Continue,
    
    /// Block request with response
    Block(BlockResponse),
    
    /// Modify content and continue
    Modify(ModifiedContent),
    
    /// Log but continue
    Log(LogLevel),
    
    /// Degrade to fallback
    Degrade(String),
}
```

### Context Types

```rust
// Request context - available in on_request
pub struct RequestContext {
    pub tenant_id: String,
    pub api_key: String,
    pub model: String,
    pub messages: Vec<Message>,
    pub headers: HashMap<String, String>,
    pub constraints: ConstraintSet,
}

// Stream context - available in on_stream_chunk
pub struct StreamChunkContext {
    pub chunk: String,
    pub chunk_index: usize,
    pub total_chunks: Option<usize>,
}

// Response context - available in on_response
pub struct ResponseContext {
    pub response: String,
    pub metadata: ResponseMetadata,
}

// Async context - available in on_async
pub struct AsyncContext {
    pub trace_id: String,
    pub trace: Arc<Trace>,
}
```

---

## Plugin Lifecycle

### Execution Order

```
┌─────────────────────────────────────────────────────────────────────────┐
│                      Pipeline Execution Flow                             │
└─────────────────────────────────────────────────────────────────────────┘

  Request ──┬─► Pre-Request Stage (Serial)
            │   ┌─────────────────────────────────────────────────────┐
            │   │ Plugin A ──► Plugin B ──► Plugin C ──► ...          │
            │   │   │           │           │                         │
            │   │   ▼           ▼           ▼                         │
            │   │ Block? ──► Modify? ──► Continue?                     │
            │   └─────────────────────────────────────────────────────┘
            │
            ▼─► Upstream LLM Proxy ◄────► Streaming Stage (Parallel)
            │   ┌─────────────────────────────────────────────────────┐
            │   │ Plugin A ║ Plugin B ║ Plugin C ║ ...                │
            │   │   │          │          │                         │
            │   │   └──────────┴──────────┴───────► Continue?         │
            │   └─────────────────────────────────────────────────────┘
            │
            ▼─► Post-Response Stage (Serial)
            │   ┌─────────────────────────────────────────────────────┐
            │   │ Plugin A ──► Plugin B ──► Plugin C ──► ...          │
            │   └─────────────────────────────────────────────────────┘
            │
            ▼─► Async Stage (Background)
                ┌─────────────────────────────────────────────────────┐
                │ Plugin A ║ Plugin B ║ Plugin C ║ ...                │
                └─────────────────────────────────────────────────────┘

  Response ◄──
```

### Version Negotiation

Plugins declare supported protocol versions:

```rust
impl GovernancePlugin for MyPlugin {
    fn supported_protocol_versions(&self) -> VersionRange {
        VersionRange::兼容(0, 2)
    }
}
```

If versions don't match:
| Policy | Behavior |
|--------|----------|
| `skip` | Skip plugin, continue |
| `fail` | Return error |

---

## Development Guide

### Creating a Native Plugin

**1. Define the Plugin:**

```rust
// src/plugin/my_plugin.rs

use async_trait::async_trait;
use crate::plugin::governance::{
    GovernancePlugin, PluginMetadata, PluginType,
    RequestContext, ResponseContext, ExecutionJournal, Action,
};
use crate::types::errors::PluginError;

pub struct MyPlugin {
    config: MyPluginConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MyPluginConfig {
    pub threshold: f64,
    pub action: String,
}

#[async_trait]
impl GovernancePlugin for MyPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            name: "my-plugin".to_string(),
            version: "1.0.0".to_string(),
            plugin_type: PluginType::Native,
            description: "My custom plugin".to_string(),
        }
    }
    
    async fn on_request(
        &self,
        ctx: &mut RequestContext,
        _journal: &mut ExecutionJournal,
    ) -> Result<Action, PluginError> {
        // Implementation
        if self.check_condition(&ctx.messages) {
            Ok(Action::Continue)
        } else {
            Ok(Action::Block(BlockResponse {
                status: 400,
                message: "Condition not met".to_string(),
            }))
        }
    }
    
    async fn on_response(
        &self,
        ctx: &mut ResponseContext,
        _journal: &mut ExecutionJournal,
    ) -> Result<Action, PluginError> {
        // Process response
        Ok(Action::Continue)
    }
    
    async fn on_stream_chunk(
        &self,
        ctx: &mut StreamChunkContext,
        _journal: &mut ExecutionJournal,
    ) -> Result<Action, PluginError> {
        // Stream processing
        Ok(Action::Continue)
    }
    
    async fn on_async(
        &self,
        ctx: &mut AsyncContext,
        _journal: &mut ExecutionJournal,
    ) -> Result<Action, PluginError> {
        // Background processing
        Ok(Action::Continue)
    }
}
```

**2. Register the Plugin:**

```rust
// src/plugin/mod.rs

mod my_plugin;

pub fn register_plugins(registry: &mut PluginRegistry) {
    registry.register("my-plugin", MyPlugin::new(config));
}
```

**3. Add to Pipeline:**

Via UI or API:
```json
POST /api/v1/pipelines
{
  "name": "my-pipeline",
  "stages": [
    {
      "placement": "pre_request",
      "plugins": [
        {
          "name": "my-plugin",
          "type": "native",
          "config": {
            "threshold": 0.8,
            "action": "block"
          },
          "enabled": true
        }
      ]
    }
  ]
}
```

### Creating a gRPC Plugin

**1. Define Protocol Buffers:**

```protobuf
// proto/my_plugin.proto
syntax = "proto3";

package veridactus.plugins;

service MyPluginService {
    rpc Process(Request) returns (Response);
}

message Request {
    string content = 1;
    map<string, string> metadata = 2;
}

message Response {
    bool success = 1;
    string result = 2;
}
```

**2. Implement gRPC Server:**

```python
# python_worker/plugins/my_plugin.py
import grpc
from concurrent import futures
import my_plugin_pb2, my_plugin_pb2_grpc

class MyPluginServicer(my_plugin_pb2_grpc.MyPluginServiceServicer):
    def Process(self, request, context):
        # Implementation
        return my_plugin_pb2.Response(
            success=True,
            result="processed"
        )
```

**3. Configure in Pipeline:**

```json
{
  "placement": "async",
  "plugins": [
    {
      "name": "my-grpc-plugin",
      "type": "grpc",
      "endpoint": "http://python-worker:8002/my-plugin",
      "config": {},
      "enabled": true
    }
  ]
}
```

---

## Configuration

### Pipeline Configuration Schema

```json
{
  "plan_id": "string",
  "tenant": "string",
  "stages": [
    {
      "placement": "pre_request | streaming | post_response | async_finalize",
      "parallel": false,
      "plugins": [
        {
          "name": "string",
          "type": "native | wasm | grpc",
          "config": {},
          "endpoint": "string (for gRPC only)",
          "enabled": true,
          "depends_on": ["string"]
        }
      ],
      "on_version_mismatch": "skip | fail"
    }
  ]
}
```

### Plugin Metadata

| Field | Type | Description |
|-------|------|-------------|
| `name` | string | Unique plugin identifier |
| `version` | string | Semantic version |
| `type` | enum | native/wasm/grpc |
| `description` | string | Human-readable description |

---

## Best Practices

### 1. Fail-Safe Design

```rust
async fn on_request(&self, ctx: &mut RequestContext, journal: &mut ExecutionJournal) -> Result<Action, PluginError> {
    // Always handle errors gracefully
    let result = match self.process(ctx).await {
        Ok(r) => r,
        Err(e) => {
            journal.log_warning(&format!("Plugin {} error: {}", self.name(), e));
            return Ok(Action::Continue); // Fail open by default
        }
    };
    // ...
}
```

### 2. Efficient Journaling

```rust
// Batch journal entries when possible
journal.log_events(vec![
    JournalEvent::SafetyEvent { .. },
    JournalEvent::ConstraintViolation { .. },
]);
```

### 3. Streaming Optimization

```rust
async fn on_stream_chunk(&self, ctx: &mut StreamChunkContext, journal: &mut ExecutionJournal) -> Result<Action, PluginError> {
    // For high-frequency checks, cache state
    if let Some(cached) = self.cache.get(&ctx.trace_id) {
        return cached.check(&ctx.chunk);
    }
    // ...
}
```

### 4. Graceful Degradation

```rust
async fn on_async(&self, ctx: &mut AsyncContext, journal: &mut ExecutionJournal) -> Result<Action, PluginError> {
    // If external service is unavailable, don't block
    match self.call_external_service(&ctx.trace).await {
        Ok(result) => { /* process */ }
        Err(e) => {
            journal.log_warning("External service unavailable, skipping");
            return Ok(Action::Continue);
        }
    }
}
```

---

## Next Steps

- [Deployment Guide](../deployment/DEPLOYMENT.md) - Deploy plugins
- [API Reference](../api/OVERVIEW.md) - Pipeline configuration API
- [Contributing Guide](../development/CONTRIBUTING.md) - Submit plugins
