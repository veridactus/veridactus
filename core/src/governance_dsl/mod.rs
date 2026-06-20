//! # Governance DSL Parser
//!
//! 严格遵循 VERIDACTUS v0.2.1 §5.8 Governance DSL Specification.
//!
//! 提供人类可读的 YAML 格式策略语言，用于声明式定义治理策略。
//! DSL 策略文件在 CONSTRAINT_EVAL 阶段编译为内部约束对象。

pub mod compiler;
pub mod parser;
pub mod validator;

pub use compiler::DslCompiler;
pub use parser::GovernanceDsl;
pub use validator::DslValidator;