//! # Governance Plugin Framework (M03)
//!
//! Strictly follows AI.md §6.0 plugin-based pipeline design.
//! Supports three plugin types: Native (Rust), Wasm, and External gRPC.
//!
//! ## Plugin Categories
//!
//! | Category | Plugin | Purpose | Execution Stage |
//! |----------|--------|---------|-----------------|
//! | **Budget Control** | BudgetGuard | Micro-dollar precision spending limits | Pre-Request |
//! | **Privacy Protection** | PiiDetector | PII detection and masking | Pre-Request |
//! | **Input Safety** | InputSanitizer | Prompt injection/jailbreak detection | Pre-Request |
//! | **Input Safety** | G1InputFilter | OWASP ASI G1 input guardrail | Pre-Request |
//! | **Output Safety** | G2OutputFilter | Harmful content detection | Post-Response |
//! | **Semantic Safety** | G3SemanticGuard | Factuality and consistency validation | Post-Response |
//! | **Adversarial Defense** | G4MultiAgentDefense | Red-team attack detection | All Stages |
//! | **Schema Validation** | ResponseValidator | OpenAI-format response validation | Post-Response |
//!
//! ## Plugin Types
//!
//! 1. **Native**: Compiled Rust plugins with highest performance
//! 2. **WASM**: WebAssembly plugins for sandboxed execution
//! 3. **gRPC**: External services for specialized processing

pub mod governance;
pub mod guardrails;
pub mod output_filter;
pub mod pii_detector;
pub mod production_plugins;
pub mod semantic_guard;

pub use governance::*;
pub use guardrails::*;
pub use output_filter::*;
pub use pii_detector::*;
pub use production_plugins::*;
pub use semantic_guard::*;
