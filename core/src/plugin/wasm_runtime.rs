//! # WASM 插件运行时 — 可选 wasmtime 引擎
//!
//! 启用 `wasm-runtime` feature 后可使用 wasmtime 加载 .wasm 插件。
//! 未启用时降级为 Flag（不阻断请求）。

use async_trait::async_trait;
use std::path::Path;
use tracing::{info, warn};

use crate::plugin::{
    AsyncContext, GovernancePlugin, PluginMetadata, PluginType, RequestContext,
    ResponseContext, StreamChunkContext,
};
use crate::types::journal::ExecutionJournal;
use crate::types::Action;

pub struct WasmPlugin {
    metadata: PluginMetadata,
    module_path: String,
    memory_pages: u32,
    fuel: u64,
    has_runtime: bool,
}

impl WasmPlugin {
    pub fn load(path: impl Into<String>) -> Result<Self, String> {
        let path_str: String = path.into();
        let path = Path::new(&path_str);
        let name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("wasm-plugin").to_string();

        if !path.exists() {
            return Err(format!("WASM module not found: {}", path_str));
        }

        let (has_runtime, desc) = Self::try_load_wasmtime(path, &path_str);
        Ok(Self {
            metadata: PluginMetadata {
                name, plugin_type: PluginType::Wasm, version: "1.0".into(),
                description: desc, author: None,
                supported_protocol_versions: crate::types::VersionRange { min: "0.2.0".into(), max: "0.3.0".into() },
            },
            module_path: path_str, memory_pages: 256, fuel: 10_000_000, has_runtime,
        })
    }

    #[cfg(feature = "wasm-runtime")]
    fn try_load_wasmtime(path: &Path, path_str: &str) -> (bool, String) {
        let engine = wasmtime::Engine::default();
        match wasmtime::Module::from_file(&engine, path) {
            Ok(_) => {
                info!("WASM module loaded: {}", path_str);
                (true, format!("WASM plugin from {}", path_str))
            }
            Err(e) => {
                warn!("WASM module compile failed: {}", e);
                (false, format!("WASM compile failed: {}", e))
            }
        }
    }

    #[cfg(not(feature = "wasm-runtime"))]
    fn try_load_wasmtime(_path: &Path, path_str: &str) -> (bool, String) {
        warn!("WASM runtime not available");
        (false, format!("WASM plugin from {} (no runtime)", path_str))
    }

    fn call_guest(&self, _func_name: &str, ctx_json: &str) -> Result<i32, String> {
        if !self.has_runtime {
            return Err("WASM runtime not available".into());
        }
        #[cfg(feature = "wasm-runtime")] {
            let engine = wasmtime::Engine::default();
            let module = wasmtime::Module::from_file(&engine, Path::new(&self.module_path))
                .map_err(|e| format!("Module load error: {}", e))?;
            let mut store = wasmtime::Store::new(&engine, ());
            store.set_fuel(self.fuel).map_err(|e| format!("Fuel: {}", e))?;
            let memory_ty = wasmtime::MemoryType::new(1, Some(self.memory_pages));
            let memory = wasmtime::Memory::new(&mut store, memory_ty).map_err(|e| format!("Mem: {}", e))?;
            let json_bytes = ctx_json.as_bytes();
            memory.write(&mut store, 0, json_bytes).map_err(|e| format!("Write: {}", e))?;
            let linker = wasmtime::Linker::new(&engine);
            let instance = linker.instantiate(&mut store, &module).map_err(|e| format!("Inst: {}", e))?;
            let func = instance.get_func(&mut store, _func_name)
                .ok_or_else(|| format!("Export '{}' not found", _func_name))?;
            let mut result = [wasmtime::Val::I32(0)];
            func.call(&mut store, &[wasmtime::Val::I32(0), wasmtime::Val::I32(json_bytes.len() as i32)], &mut result)
                .map_err(|e| format!("Call: {}", e))?;
            match result[0] { wasmtime::Val::I32(c) => return Ok(c), _ => return Err("bad result".into()) }
        }
        #[cfg(not(feature = "wasm-runtime"))]
        Err("wasm-runtime feature not enabled".into())
    }
}

fn code_to_action(code: i32) -> Action {
    match code { 1 => Action::Block, 2 => Action::Degrade, 3 => Action::Flag, _ => Action::Continue }
}

#[async_trait]
impl GovernancePlugin for WasmPlugin {
    fn metadata(&self) -> PluginMetadata { self.metadata.clone() }

    async fn on_request(&self, ctx: &mut RequestContext, _j: &mut ExecutionJournal) -> Result<Action, String> {
        let json = serde_json::to_string(&serde_json::json!({
            "headers": ctx.headers, "body": ctx.body,
            "trace_id": ctx.trace_id.to_string(), "tenant_id": ctx.tenant_id,
        })).unwrap_or_default();
        match self.call_guest("on_request", &json) {
            Ok(c) => Ok(code_to_action(c)),
            Err(_) => { warn!("WASM {} on_request failed", self.metadata.name); Ok(Action::Flag) }
        }
    }

    async fn on_stream_chunk(&self, _c: &mut StreamChunkContext, _j: &mut ExecutionJournal) -> Result<Action, String> {
        Ok(Action::Continue) // streaming 阶段 WASM 跳过
    }

    async fn on_response(&self, ctx: &mut ResponseContext, _j: &mut ExecutionJournal) -> Result<Action, String> {
        let json = serde_json::to_string(&serde_json::json!({
            "response": ctx.response, "actual_cost": ctx.actual_cost,
            "trace_id": ctx.trace_id.to_string(),
        })).unwrap_or_default();
        match self.call_guest("on_response", &json) {
            Ok(c) => Ok(code_to_action(c)),
            Err(_) => { warn!("WASM {} on_response failed", self.metadata.name); Ok(Action::Flag) }
        }
    }

    async fn on_async_finalize(&self, ctx: &mut AsyncContext) -> Result<serde_json::Value, String> {
        let json = serde_json::to_string(&serde_json::json!({
            "trace_id": ctx.trace_id.to_string(),
            "task_type": ctx.task_type, "params": ctx.params,
        })).unwrap_or_default();
        match self.call_guest("on_async_finalize", &json) {
            Ok(_) => Ok(serde_json::json!({"status":"completed"})),
            Err(e) => Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wasm_load_nonexistent() { assert!(WasmPlugin::load("/nonexistent.wasm").is_err()); }

    #[test]
    fn test_code_actions() {
        assert_eq!(code_to_action(0), Action::Continue);
        assert_eq!(code_to_action(1), Action::Block);
        assert_eq!(code_to_action(2), Action::Degrade);
        assert_eq!(code_to_action(3), Action::Flag);
    }
}
