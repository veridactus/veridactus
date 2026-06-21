//! # Storage Backend Auto-Selection
//!
//! Automatically selects storage backend based on environment variables.
//! Supports:
//! - `memory` (default): In-memory storage for development/testing
//! - `file`: Local file storage for single-node deployment
//! - `postgres`: PostgreSQL storage for production clusters
//!
//! Environment Variables:
//!   VERIDACTUS_STORE_BACKEND = memory | file | postgres
//!   VERIDACTUS_DATA_DIR       = File storage directory (default: "data/traces")
//!   DATABASE_URL              = PostgreSQL connection string
//!   POSTGRES_URL              = PostgreSQL connection string (alternative)
//!
//! Redis Configuration:
//!   VERIDACTUS_STORE_REDIS_HOST = Redis host (default: "localhost")
//!   VERIDACTUS_STORE_REDIS_PORT = Redis port (default: 6379)
//!
//! S3/MinIO Configuration:
//!   VERIDACTUS_STORE_S3_ENDPOINT   = S3 endpoint (e.g., http://minio:9000)
//!   VERIDACTUS_STORE_S3_BUCKET     = S3 bucket name
//!   VERIDACTUS_STORE_S3_ACCESS_KEY = S3 access key
//!   VERIDACTUS_STORE_S3_SECRET_KEY = S3 secret key
//!   VERIDACTUS_STORE_S3_REGION     = S3 region (default: "us-east-1")

use std::sync::Arc;
use tracing::{info, warn};

use crate::store::{FileTraceStore, InMemoryTraceStore, PostgresTraceStore, TraceStore};

use crate::store::traits::ObjectStore;

#[cfg(feature = "s3")]
use crate::store::S3ObjectStore;

use crate::store::adapters::redis::{RedisBudgetStore, RedisCacheStore};
use crate::store::traits::{BudgetStore, CacheStore};

/// Storage backend enum
#[derive(Debug, Clone, PartialEq)]
pub enum StoreBackend {
    Memory,
    File,
    Postgres,
}

impl StoreBackend {
    /// Detect backend type from environment variables
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

/// Create TraceStore based on backend type
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

/// Create Redis cache store (optional)
///
/// Returns None if Redis is not configured or connection fails.
pub async fn create_cache_store() -> Option<Arc<dyn CacheStore>> {
    let redis_host =
        std::env::var("VERIDACTUS_STORE_REDIS_HOST").unwrap_or_else(|_| "localhost".to_string());
    let redis_port = std::env::var("VERIDACTUS_STORE_REDIS_PORT")
        .unwrap_or_else(|_| "6379".to_string())
        .parse()
        .unwrap_or(6379);

    let redis_url = format!("redis://{}:{}", redis_host, redis_port);

    match redis::Client::open(redis_url.as_str()) {
        Ok(client) => match redis::aio::ConnectionManager::new(client).await {
            Ok(conn) => {
                info!("Redis cache store connected: {}:{}", redis_host, redis_port);
                Some(Arc::new(RedisCacheStore::new(conn)))
            }
            Err(e) => {
                warn!("Redis connection failed: {}, using in-memory cache", e);
                None
            }
        },
        Err(e) => {
            warn!("Redis client creation failed: {}, using in-memory cache", e);
            None
        }
    }
}

/// Create Redis budget store (optional)
///
/// Returns None if Redis is not configured or connection fails.
pub async fn create_budget_store() -> Option<Arc<dyn BudgetStore>> {
    let redis_host =
        std::env::var("VERIDACTUS_STORE_REDIS_HOST").unwrap_or_else(|_| "localhost".to_string());
    let redis_port = std::env::var("VERIDACTUS_STORE_REDIS_PORT")
        .unwrap_or_else(|_| "6379".to_string())
        .parse()
        .unwrap_or(6379);

    let redis_url = format!("redis://{}:{}", redis_host, redis_port);

    match redis::Client::open(redis_url.as_str()) {
        Ok(client) => match redis::aio::ConnectionManager::new(client).await {
            Ok(conn) => {
                info!(
                    "Redis budget store connected: {}:{}",
                    redis_host, redis_port
                );
                Some(Arc::new(RedisBudgetStore::new(conn)))
            }
            Err(e) => {
                warn!("Redis connection failed: {}, budget tracking disabled", e);
                None
            }
        },
        Err(e) => {
            warn!(
                "Redis client creation failed: {}, budget tracking disabled",
                e
            );
            None
        }
    }
}

/// Create S3/MinIO object store (optional, requires `s3` feature)
///
/// Returns None if S3 is not configured or feature is disabled.
#[cfg(feature = "s3")]
pub async fn create_object_store() -> Option<Arc<dyn ObjectStore>> {
    let endpoint = std::env::var("VERIDACTUS_STORE_S3_ENDPOINT").ok()?;
    let bucket = std::env::var("VERIDACTUS_STORE_S3_BUCKET").ok()?;
    let access_key = std::env::var("VERIDACTUS_STORE_S3_ACCESS_KEY").ok()?;
    let secret_key = std::env::var("VERIDACTUS_STORE_S3_SECRET_KEY").ok()?;
    let region =
        std::env::var("VERIDACTUS_STORE_S3_REGION").unwrap_or_else(|_| "us-east-1".to_string());

    info!(
        "Initializing S3/MinIO object store: endpoint={}, bucket={}",
        endpoint, bucket
    );

    // Configure AWS SDK for MinIO/S3
    let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;

    let s3_config = aws_sdk_s3::config::Builder::from(&config)
        .endpoint_url(&endpoint)
        .region(aws_sdk_s3::config::Region::new(region))
        .force_path_style(true) // Required for MinIO
        .build();

    let client = aws_sdk_s3::Client::from_conf(s3_config);

    // Create bucket if it doesn't exist
    if let Err(e) = client.create_bucket().bucket(&bucket).send().await {
        if !e.to_string().contains("BucketAlreadyOwnedByYou")
            && !e.to_string().contains("BucketAlreadyExists")
        {
            warn!("Failed to create S3 bucket {}: {}", bucket, e);
        }
    }

    info!("S3/MinIO object store initialized: bucket={}", bucket);
    Some(Arc::new(S3ObjectStore::new(client, bucket)))
}

#[cfg(not(feature = "s3"))]
pub async fn create_object_store() -> Option<Arc<dyn crate::store::traits::ObjectStore>> {
    warn!("S3 feature is not enabled, object storage disabled");
    None
}
