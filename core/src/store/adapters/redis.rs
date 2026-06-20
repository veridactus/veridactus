//! # Redis 缓存存储适配器
//!
//! 使用 Redis 作为缓存层，支持预算状态和配置缓存。

use async_trait::async_trait;
use redis::{AsyncCommands, aio::ConnectionManager};
use crate::store::traits::{CacheStore, BudgetStore};

pub struct RedisCacheStore {
    client: ConnectionManager,
}

impl RedisCacheStore {
    pub fn new(client: ConnectionManager) -> Self {
        Self { client }
    }
}

#[async_trait]
impl CacheStore for RedisCacheStore {
    async fn get(&self, key: &str) -> Option<String> {
        let mut client = self.client.clone();
        client.get(key).await.ok()
    }

    async fn set(&self, key: &str, value: &str, ttl_secs: Option<u64>) -> Result<(), String> {
        let mut client = self.client.clone();
        if let Some(ttl) = ttl_secs {
            client.set_ex(key, value, ttl).await.map_err(|e| e.to_string())
        } else {
            client.set(key, value).await.map_err(|e| e.to_string())
        }
    }

    async fn delete(&self, key: &str) -> Result<(), String> {
        let mut client = self.client.clone();
        client.del::<_, ()>(key).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    async fn exists(&self, key: &str) -> bool {
        let mut client = self.client.clone();
        client.exists(key).await.unwrap_or(false)
    }
}

pub struct RedisBudgetStore {
    client: ConnectionManager,
}

impl RedisBudgetStore {
    pub fn new(client: ConnectionManager) -> Self {
        Self { client }
    }

    fn budget_key(tenant_id: &str) -> String {
        format!("veridactus:budget:{}", tenant_id)
    }

    fn limit_key(tenant_id: &str) -> String {
        format!("veridactus:budget_limit:{}", tenant_id)
    }
}

#[async_trait]
impl BudgetStore for RedisBudgetStore {
    async fn get_remaining(&self, tenant_id: &str) -> Option<f64> {
        let mut client = self.client.clone();
        client.get(Self::budget_key(tenant_id)).await.ok()
    }

    async fn reserve(&self, tenant_id: &str, amount: f64) -> Result<f64, String> {
        let mut client = self.client.clone();
        let key = Self::budget_key(tenant_id);
        redis::cmd("DECRBYFLOAT").arg(&key).arg(amount.to_string()).query_async(&mut client).await.map_err(|e| e.to_string())
    }

    async fn release(&self, tenant_id: &str, amount: f64) -> Result<f64, String> {
        let mut client = self.client.clone();
        let key = Self::budget_key(tenant_id);
        redis::cmd("INCRBYFLOAT").arg(&key).arg(amount.to_string()).query_async(&mut client).await.map_err(|e| e.to_string())
    }

    async fn set_limit(&self, tenant_id: &str, limit: f64) -> Result<(), String> {
        let mut client = self.client.clone();
        client.set(Self::limit_key(tenant_id), limit).await.map_err(|e| e.to_string())
    }

    async fn reset(&self, tenant_id: &str) -> Result<(), String> {
        let mut client = self.client.clone();
        let limit_key = Self::limit_key(tenant_id);
        let budget_key = Self::budget_key(tenant_id);
        
        let limit: Option<f64> = client.get(&limit_key).await.map_err(|e| e.to_string())?;
        
        if let Some(limit_val) = limit {
            client.set(&budget_key, limit_val).await.map_err(|e| e.to_string())
        } else {
            Ok(())
        }
    }
}