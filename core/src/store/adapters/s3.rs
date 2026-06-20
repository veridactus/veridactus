//! # S3/MinIO 对象存储适配器
//!
//! 用于存储大型 Trace 对象和归档数据。
//!
//! 注意: S3 功能需要启用 `s3` feature
//! ```toml
//! [features]
//! default = ["s3"]
//! s3 = ["aws-sdk-s3"]
//! ```

#[cfg(feature = "s3")]
mod s3_impl {
    use crate::store::traits::ObjectStore;
    use async_trait::async_trait;

    pub struct S3ObjectStore {
        client: aws_sdk_s3::Client,
        bucket: String,
    }

    impl S3ObjectStore {
        pub fn new(client: aws_sdk_s3::Client, bucket: impl Into<String>) -> Self {
            Self {
                client,
                bucket: bucket.into(),
            }
        }
    }

    #[async_trait]
    impl ObjectStore for S3ObjectStore {
        async fn put(&self, bucket: &str, key: &str, data: &[u8]) -> Result<(), String> {
            self.client
                .put_object()
                .bucket(bucket)
                .key(key)
                .body(aws_sdk_s3::primitives::ByteStream::from(data.to_vec()))
                .send()
                .await
                .map_err(|e| e.to_string())?;
            Ok(())
        }

        async fn get(&self, bucket: &str, key: &str) -> Option<Vec<u8>> {
            let output = self
                .client
                .get_object()
                .bucket(bucket)
                .key(key)
                .send()
                .await
                .ok()?;
            // collect() is async — use .await directly, NOT block_on
            let bytes = output.body.collect().await.ok()?;
            Some(bytes.to_vec())
        }

        async fn delete(&self, bucket: &str, key: &str) -> Result<(), String> {
            self.client
                .delete_object()
                .bucket(bucket)
                .key(key)
                .send()
                .await
                .map_err(|e| e.to_string())?;
            Ok(())
        }

        async fn list(&self, bucket: &str, prefix: &str) -> Vec<String> {
            self.client
                .list_objects_v2()
                .bucket(bucket)
                .prefix(prefix)
                .send()
                .await
                .ok()
                .map(|output| {
                    output
                        .contents()
                        .iter()
                        .flatten()
                        .filter_map(|obj| obj.key().map(|k| k.to_string()))
                        .collect()
                })
                .unwrap_or_default()
        }
    }
}

#[cfg(feature = "s3")]
pub use s3_impl::S3ObjectStore;
