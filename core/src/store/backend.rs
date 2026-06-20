//! # 存储后端自动选择
//!
//! 根据环境变量自动选择存储后端，支持：
//! - `memory` (默认): 内存存储，开发/测试
//! - `file`: 本地文件存储，单机部署
//! - `postgres`: PostgreSQL 存储，生产集群部署
//!
//! 环境变量:
//!   VERIDACTUS_STORE_BACKEND = memory | file | postgres
//!   VERIDACTUS_DATA_DIR       = 文件存储目录（默认 "data/traces"）
//!   DATABASE_URL              = PostgreSQL 连接字符串
//!   POSTGRES_URL              = PostgreSQL 连接字符串（备选）

use std::sync::Arc;
use tracing::info;

use crate::store::{FileTraceStore, InMemoryTraceStore, PostgresTraceStore, TraceStore};

/// 存储后端枚举
#[derive(Debug, Clone, PartialEq)]
pub enum StoreBackend {
    Memory,
    File,
    Postgres,
}

impl StoreBackend {
    /// 从环境变量检测后端类型
    pub fn detect() -> Self {
        let backend = std::env::var("VERIDACTUS_STORE_BACKEND")
            .unwrap_or_default()
            .to_lowercase();

        match backend.as_str() {
            "postgres" | "pg" | "postgresql" => {
                info!("Storage backend detected: PostgreSQL");
                StoreBackend::Postgres
            }
            "file" | "local" | "disk" => {
                info!("Storage backend detected: local file");
                StoreBackend::File
            }
            _ => {
                info!("Storage backend detected: memory (default)");
                StoreBackend::Memory
            }
        }
    }
}

/// 根据后端类型创建 TraceStore
pub async fn create_trace_store(backend: &StoreBackend) -> Arc<dyn TraceStore> {
    match backend {
        StoreBackend::Memory => Arc::new(InMemoryTraceStore::new()),
        StoreBackend::File => {
            let data_dir =
                std::env::var("VERIDACTUS_DATA_DIR").unwrap_or_else(|_| "data/traces".to_string());
            Arc::new(FileTraceStore::new(&data_dir))
        }
        StoreBackend::Postgres => {
            let pg_url = std::env::var("DATABASE_URL")
                .or_else(|_| std::env::var("POSTGRES_URL"))
                .unwrap_or_else(|_| "postgres://localhost:5432/veridactus".to_string());

            let pool = sqlx::postgres::PgPoolOptions::new()
                .max_connections(10)
                .connect(&pg_url)
                .await
                .expect("Failed to connect to PostgreSQL");

            let pool = Arc::new(pool);
            PostgresTraceStore::init_schema(&pool)
                .await
                .expect("Failed to initialize PostgreSQL schema");

            Arc::new(PostgresTraceStore::new(pool))
        }
    }
}
