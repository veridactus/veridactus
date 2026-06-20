//! # 异步任务分发器 (M05)
//!
//! 严格遵循 AI.md §2.1 架构: AsyncQueue → Redis Stream → Workers。
//! 将异步任务推入 Redis Stream，供 Python Worker 消费。

pub mod redis_dispatch;

pub use redis_dispatch::*;
