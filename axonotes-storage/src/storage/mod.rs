pub mod streaming;

use crate::config::StorageConfig;
use aws_config::BehaviorVersion;
use aws_sdk_s3::{Client, config::Credentials, config::Region};

/// S3 storage client wrapper
#[derive(Clone)]
pub struct StorageClient {
    client: Client,
    pub bucket: String,
}

impl StorageClient {
    /// Create new S3 client from config
    pub async fn new(config: &StorageConfig) -> Self {
        let credentials = Credentials::new(
            &config.access_key,
            &config.secret_key,
            None, // session token
            None, // expiry
            "static",
        );

        let mut s3_config_builder = aws_sdk_s3::config::Builder::new()
            .credentials_provider(credentials)
            .endpoint_url(&config.endpoint)
            .behavior_version(BehaviorVersion::latest())
            .force_path_style(true); // Required for MinIO

        // Add region if specified
        if let Some(region) = &config.region {
            s3_config_builder = s3_config_builder.region(Region::new(region.clone()));
        }

        let s3_config = s3_config_builder.build();
        let client = Client::from_conf(s3_config);

        Self {
            client,
            bucket: config.bucket.clone(),
        }
    }

    /// Get S3 client
    pub fn client(&self) -> &Client {
        &self.client
    }

    /// Get bucket name
    pub fn bucket(&self) -> &str {
        &self.bucket
    }
}
