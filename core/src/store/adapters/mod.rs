//! # 存储适配器实现
//!
//! 提供各种存储后端的适配器实现。

pub mod clickhouse;
pub mod file_trace;
pub mod memory;
pub mod postgres;
pub mod redis;
pub mod s3;

pub use clickhouse::ClickHouseTraceStore;
pub use file_trace::FileTraceStore;
pub use memory::InMemoryCacheStore;
pub use memory::InMemoryTraceStore;
pub use memory::LocalFileStore;
pub use postgres::PostgresTraceStore;
pub use redis::RedisCacheStore;

#[cfg(feature = "s3")]
pub use s3::S3ObjectStore;
