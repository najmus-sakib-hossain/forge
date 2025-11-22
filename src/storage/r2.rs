/// Cloudflare R2 Storage Backend
///
/// This module provides integration with Cloudflare R2 for blob storage.
/// Zero egress fees make it perfect for code hosting platforms.
use anyhow::{Context, Result};
use reqwest::{header, Client, StatusCode};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::time::Duration;

use super::blob::Blob;

/// R2 configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct R2Config {
    /// R2 account ID
    pub account_id: String,

    /// R2 bucket name
    pub bucket_name: String,

    /// R2 access key ID
    pub access_key_id: String,

    /// R2 secret access key
    pub secret_access_key: String,

    /// Custom domain (optional)
    pub custom_domain: Option<String>,
}

impl R2Config {
    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self> {
        dotenvy::dotenv().ok();

        let account_id = std::env::var("R2_ACCOUNT_ID").context("R2_ACCOUNT_ID not set in .env")?;
        let bucket_name =
            std::env::var("R2_BUCKET_NAME").context("R2_BUCKET_NAME not set in .env")?;
        let access_key_id =
            std::env::var("R2_ACCESS_KEY_ID").context("R2_ACCESS_KEY_ID not set in .env")?;
        let secret_access_key = std::env::var("R2_SECRET_ACCESS_KEY")
            .context("R2_SECRET_ACCESS_KEY not set in .env")?;
        let custom_domain = std::env::var("R2_CUSTOM_DOMAIN").ok();

        Ok(Self {
            account_id,
            bucket_name,
            access_key_id,
            secret_access_key,
            custom_domain,
        })
    }

    /// Get R2 endpoint URL
    pub fn endpoint_url(&self) -> String {
        if let Some(domain) = &self.custom_domain {
            format!("https://{}", domain)
        } else {
            format!("https://{}.r2.cloudflarestorage.com", self.account_id)
        }
    }
}

/// R2 storage client
pub struct R2Storage {
    config: R2Config,
    client: Client,
}

impl R2Storage {
    /// Create new R2 storage client
    pub fn new(config: R2Config) -> Result<Self> {
        let client = Client::builder().timeout(Duration::from_secs(30)).build()?;

        Ok(Self { config, client })
    }

    /// Upload blob to R2
    pub async fn upload_blob(&self, blob: &Blob) -> Result<String> {
        let hash = blob.hash();
        let key = format!("blobs/{}/{}", &hash[..2], &hash[2..]);

        let binary = blob.to_binary()?;
        let content_hash = compute_sha256_hex(&binary);
        let date = chrono::Utc::now().format("%Y%m%dT%H%M%SZ").to_string();

        let url = format!(
            "{}/{}/{}",
            self.config.endpoint_url(),
            self.config.bucket_name,
            key
        );

        // Create AWS Signature V4 (simplified - in production use aws-sigv4 crate)
        let authorization = self.create_auth_header("PUT", &key, &binary)?;

        let response = self
            .client
            .put(&url)
            .header(header::AUTHORIZATION, authorization)
            .header(header::CONTENT_TYPE, "application/octet-stream")
            .header("x-amz-content-sha256", content_hash)
            .header("x-amz-date", date)
            .body(binary)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("R2 upload failed: {} - {}", status, body);
        }

        Ok(key)
    }

    /// Download blob from R2
    pub async fn download_blob(&self, hash: &str) -> Result<Blob> {
        let key = format!("blobs/{}/{}", &hash[..2], &hash[2..]);
        let date = chrono::Utc::now().format("%Y%m%dT%H%M%SZ").to_string();

        let url = format!(
            "{}/{}/{}",
            self.config.endpoint_url(),
            self.config.bucket_name,
            key
        );

        let authorization = self.create_auth_header("GET", &key, &[])?;

        let response = self
            .client
            .get(&url)
            .header(header::AUTHORIZATION, authorization)
            .header("x-amz-date", date)
            .header("x-amz-content-sha256", "UNSIGNED-PAYLOAD")
            .send()
            .await?;

        if response.status() == StatusCode::NOT_FOUND {
            anyhow::bail!("Blob not found: {}", hash);
        }

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("R2 download failed: {} - {}", status, body);
        }

        let binary = response.bytes().await?;
        Blob::from_binary(&binary)
    }

    /// Check if blob exists in R2
    pub async fn blob_exists(&self, hash: &str) -> Result<bool> {
        let key = format!("blobs/{}/{}", &hash[..2], &hash[2..]);
        let date = chrono::Utc::now().format("%Y%m%dT%H%M%SZ").to_string();

        let url = format!(
            "{}/{}/{}",
            self.config.endpoint_url(),
            self.config.bucket_name,
            key
        );

        let authorization = self.create_auth_header("HEAD", &key, &[])?;

        let response = self
            .client
            .head(&url)
            .header(header::AUTHORIZATION, authorization)
            .header("x-amz-date", date)
            .send()
            .await?;

        Ok(response.status().is_success())
    }

    /// Delete blob from R2
    pub async fn delete_blob(&self, hash: &str) -> Result<()> {
        let key = format!("blobs/{}/{}", &hash[..2], &hash[2..]);
        let date = chrono::Utc::now().format("%Y%m%dT%H%M%SZ").to_string();

        let url = format!(
            "{}/{}/{}",
            self.config.endpoint_url(),
            self.config.bucket_name,
            key
        );

        let authorization = self.create_auth_header("DELETE", &key, &[])?;

        let response = self
            .client
            .delete(&url)
            .header(header::AUTHORIZATION, authorization)
            .header("x-amz-date", date)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("R2 delete failed: {} - {}", status, body);
        }

        Ok(())
    }

    /// Download component from R2
    pub async fn download_component(
        &self,
        tool: &str,
        component: &str,
        version: Option<&str>,
    ) -> Result<String> {
        let version = version.unwrap_or("latest");
        let key = format!("components/{}/{}/{}.tsx", tool, version, component);
        let date = chrono::Utc::now().format("%Y%m%dT%H%M%SZ").to_string();

        let url = format!(
            "{}/{}/{}",
            self.config.endpoint_url(),
            self.config.bucket_name,
            key
        );

        let authorization = self.create_auth_header("GET", &key, &[])?;

        let response = self
            .client
            .get(&url)
            .header(header::AUTHORIZATION, authorization)
            .header("x-amz-date", date)
            .header("x-amz-content-sha256", "UNSIGNED-PAYLOAD")
            .send()
            .await?;

        if response.status() == StatusCode::NOT_FOUND {
            anyhow::bail!("Component not found: {}/{} v{}", tool, component, version);
        }

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("R2 component download failed: {} - {}", status, body);
        }

        let content = response.text().await?;
        Ok(content)
    }

    /// Upload component to R2
    pub async fn upload_component(
        &self,
        tool: &str,
        component: &str,
        version: &str,
        content: &str,
    ) -> Result<String> {
        let key = format!("components/{}/{}/{}.tsx", tool, version, component);
        let binary = content.as_bytes();
        let content_hash = compute_sha256_hex(binary);
        let date = chrono::Utc::now().format("%Y%m%dT%H%M%SZ").to_string();

        let url = format!(
            "{}/{}/{}",
            self.config.endpoint_url(),
            self.config.bucket_name,
            key
        );

        let authorization = self.create_auth_header("PUT", &key, binary)?;

        let response = self
            .client
            .put(&url)
            .header(header::AUTHORIZATION, authorization)
            .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
            .header("x-amz-content-sha256", content_hash)
            .header("x-amz-date", date)
            .body(content.to_string())
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("R2 component upload failed: {} - {}", status, body);
        }

        Ok(key)
    }

    /// Check if component exists in R2
    pub async fn component_exists(
        &self,
        tool: &str,
        component: &str,
        version: Option<&str>,
    ) -> Result<bool> {
        let version = version.unwrap_or("latest");
        let key = format!("components/{}/{}/{}.tsx", tool, version, component);
        let date = chrono::Utc::now().format("%Y%m%dT%H%M%SZ").to_string();

        let url = format!(
            "{}/{}/{}",
            self.config.endpoint_url(),
            self.config.bucket_name,
            key
        );

        let authorization = self.create_auth_header("HEAD", &key, &[])?;

        let response = self
            .client
            .head(&url)
            .header(header::AUTHORIZATION, authorization)
            .header("x-amz-date", date)
            .send()
            .await?;

        Ok(response.status().is_success())
    }

    /// List all components in R2
    pub async fn list_components(&self, tool: &str) -> Result<Vec<String>> {
        let prefix = format!("components/{}/", tool);
        let url = format!(
            "{}/{}/?list-type=2&prefix={}",
            self.config.endpoint_url(),
            self.config.bucket_name,
            prefix
        );

        let date = chrono::Utc::now().format("%Y%m%dT%H%M%SZ").to_string();
        let authorization = self.create_auth_header("GET", &format!("?list-type=2&prefix={}", prefix), &[])?;

        let response = self
            .client
            .get(&url)
            .header(header::AUTHORIZATION, authorization)
            .header("x-amz-date", date)
            .header("x-amz-content-sha256", "UNSIGNED-PAYLOAD")
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("R2 list failed: {} - {}", status, body);
        }

        // Parse XML response (simplified - in production use proper XML parser)
        let body = response.text().await?;
        let mut components = Vec::new();
        
        for line in body.lines() {
            if line.contains("<Key>") {
                let key = line.replace("<Key>", "").replace("</Key>", "").trim().to_string();
                if let Some(name) = key.split('/').last() {
                    if let Some(component_name) = name.strip_suffix(".tsx") {
                        components.push(component_name.to_string());
                    }
                }
            }
        }

        Ok(components)
    }

    /// Sync components (bidirectional)
    pub async fn sync_components(
        &self, 
        tool: &str, 
        local_components: &[String],
        on_download: impl Fn(&str),
        on_upload: impl Fn(&str)
    ) -> Result<()> {
        // 1. List remote components
        let remote_components = self.list_components(tool).await?;
        
        // 2. Calculate sync actions
        let (to_download, to_upload) = self.calculate_sync_actions(&remote_components, local_components);
        
        // 3. Execute actions
        for remote in to_download {
            on_download(&remote);
        }
        
        for local in to_upload {
            on_upload(&local);
        }
        
        Ok(())
    }

    /// Calculate what needs to be downloaded and uploaded
    /// Returns (to_download, to_upload)
    #[cfg_attr(test, allow(dead_code))]
    pub(crate) fn calculate_sync_actions(&self, remote_components: &[String], local_components: &[String]) -> (Vec<String>, Vec<String>) {
        let mut to_download = Vec::new();
        let mut to_upload = Vec::new();

        for remote in remote_components {
            if !local_components.contains(remote) {
                to_download.push(remote.clone());
            }
        }

        for local in local_components {
            if !remote_components.contains(local) {
                to_upload.push(local.clone());
            }
        }

        (to_download, to_upload)
    }

    /// Create AWS Signature V4 authorization header (simplified)
    fn create_auth_header(&self, method: &str, key: &str, body: &[u8]) -> Result<String> {
        // Simplified auth - in production, use aws-sigv4 crate for proper signing
        // For R2, you can also use S3-compatible libraries

        use hmac::{Hmac, Mac};
        use sha2::Sha256;

        type HmacSha256 = Hmac<Sha256>;

        let date = chrono::Utc::now().format("%Y%m%dT%H%M%SZ").to_string();
        let date_short = &date[..8];

        let body_hash = compute_sha256_hex(body);
        let host = format!("{}.r2.cloudflarestorage.com", self.config.account_id);

        // Canonical request
        let canonical_request = format!(
            "{}\n/{}/{}\n\nhost:{}\nx-amz-content-sha256:{}\nx-amz-date:{}\n\nhost;x-amz-content-sha256;x-amz-date\n{}",
            method,
            self.config.bucket_name,
            key,
            host,
            body_hash,
            date,
            body_hash
        );

        let canonical_request_hash = compute_sha256_hex(canonical_request.as_bytes());

        // String to sign
        let string_to_sign = format!(
            "AWS4-HMAC-SHA256\n{}\n{}/auto/s3/aws4_request\n{}",
            date, date_short, canonical_request_hash
        );

        // Signing key
        let mut mac = HmacSha256::new_from_slice(
            format!("AWS4{}", self.config.secret_access_key).as_bytes(),
        )?;
        mac.update(date_short.as_bytes());
        let date_key = mac.finalize().into_bytes();

        let mut mac = HmacSha256::new_from_slice(&date_key)?;
        mac.update(b"auto");
        let region_key = mac.finalize().into_bytes();

        let mut mac = HmacSha256::new_from_slice(&region_key)?;
        mac.update(b"s3");
        let service_key = mac.finalize().into_bytes();

        let mut mac = HmacSha256::new_from_slice(&service_key)?;
        mac.update(b"aws4_request");
        let signing_key = mac.finalize().into_bytes();

        // Signature
        let mut mac = HmacSha256::new_from_slice(&signing_key)?;
        mac.update(string_to_sign.as_bytes());
        let signature = hex::encode(mac.finalize().into_bytes());

        Ok(format!(
            "AWS4-HMAC-SHA256 Credential={}/{}/auto/s3/aws4_request, SignedHeaders=host;x-amz-content-sha256;x-amz-date, Signature={}",
            self.config.access_key_id,
            date_short,
            signature
        ))
    }

    /// Sync local blobs up to R2 (upload missing blobs)
    pub async fn sync_up(
        &self,
        local_blobs: Vec<Blob>,
        progress_callback: Option<impl Fn(usize, usize) + Send + Sync>,
    ) -> Result<SyncResult> {
        use futures::stream::{self, StreamExt};
        
        tracing::info!("ðŸ”„ Starting R2 sync up: {} local blobs", local_blobs.len());
        
        let mut uploaded = 0;
        let mut skipped = 0;
        let mut errors = Vec::new();
        let total = local_blobs.len();

        // Check which blobs already exist in R2
        let mut to_upload = Vec::new();
        for blob in local_blobs {
            match self.blob_exists(&blob.hash()).await {
                Ok(exists) => {
                    if exists {
                        skipped += 1;
                    } else {
                        to_upload.push(blob);
                    }
                }
                Err(e) => {
                    errors.push(format!("Failed to check blob {}: {}", blob.hash(), e));
                    to_upload.push(blob); // Try to upload anyway
                }
            }
        }

        // Upload missing blobs in parallel (max 10 concurrent)
        let mut stream = stream::iter(to_upload.into_iter().enumerate())
            .map(|(idx, blob)| async move {
                let hash = blob.hash();
                match self.upload_blob(&blob).await {
                    Ok(_) => Ok::<(usize, String), String>((idx, hash.to_string())),
                    Err(e) => Err(format!("Failed to upload {}: {}", hash, e)),
                }
            })
            .buffer_unordered(10);

        while let Some(result) = stream.next().await {
            match result {
                Ok((_idx, _hash)) => {
                    uploaded += 1;
                    if let Some(cb) = &progress_callback {
                        cb(uploaded + skipped, total);
                    }
                }
                Err(e) => {
                    errors.push(e);
                }
            }
        }

        tracing::info!(
            "âœ… Sync up complete: {} uploaded, {} skipped, {} errors",
            uploaded,
            skipped,
            errors.len()
        );

        Ok(SyncResult {
            uploaded,
            downloaded: 0,
            skipped,
            errors,
        })
    }

    /// Sync remote blobs down from R2 (download missing blobs)
    pub async fn sync_down(
        &self,
        remote_hashes: Vec<String>,
        progress_callback: Option<impl Fn(usize, usize) + Send + Sync>,
    ) -> Result<Vec<Blob>> {
        use futures::stream::{self, StreamExt};
        
        tracing::info!("ðŸ”„ Starting R2 sync down: {} remote blobs", remote_hashes.len());
        
        let total = remote_hashes.len();
        let mut downloaded_blobs = Vec::new();

        // Download blobs in parallel (max 10 concurrent)
        let mut stream = stream::iter(remote_hashes.into_iter().enumerate())
            .map(|(idx, hash)| async move {
                match self.download_blob(&hash).await {
                    Ok(blob) => Ok::<(usize, Blob), String>((idx, blob)),
                    Err(e) => Err(format!("Failed to download {}: {}", hash, e)),
                }
            })
            .buffer_unordered(10);

        let mut errors = Vec::new();
        while let Some(result) = stream.next().await {
            match result {
                Ok((idx, blob)) => {
                    downloaded_blobs.push(blob);
                    if let Some(cb) = &progress_callback {
                        cb(idx + 1, total);
                    }
                }
                Err(e) => {
                    tracing::warn!("âš ï¸ {}", e);
                    errors.push(e);
                }
            }
        }

        tracing::info!(
            "âœ… Sync down complete: {} downloaded, {} errors", downloaded_blobs.len(),
            errors.len()
        );

        Ok(downloaded_blobs)
    }

    /// List all blob hashes in R2 bucket (simplified - in production use pagination)
    pub async fn list_blobs(&self, _prefix: Option<&str>) -> Result<Vec<String>> {
        // This is a simplified version. In production, use S3 ListObjects API
        // For now, return empty list as listing requires more complex S3 API integration
        tracing::warn!("R2 list_blobs not fully implemented - requires S3 ListObjects API");
        Ok(Vec::new())
    }
}

/// Sync operation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResult {
    pub uploaded: usize,
    pub downloaded: usize,
    pub skipped: usize,
    pub errors: Vec<String>,
}

/// Compute SHA-256 hex string
fn compute_sha256_hex(data: &[u8]) -> String {
    use sha2::Digest;
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

/// Batch upload blobs with progress tracking
pub async fn batch_upload_blobs(
    storage: &R2Storage,
    blobs: Vec<Blob>,
    progress_callback: impl Fn(usize, usize),
) -> Result<Vec<String>> {
    use futures::stream::{self, StreamExt};

    let total = blobs.len();
    let mut keys = Vec::with_capacity(total);

    // Upload in parallel (max 10 concurrent)
    let mut stream = stream::iter(blobs.into_iter().enumerate())
        .map(|(idx, blob)| async move {
            let key = storage.upload_blob(&blob).await?;
            Ok::<(usize, String), anyhow::Error>((idx, key))
        })
        .buffer_unordered(10);

    while let Some(result) = stream.next().await {
        let (idx, key) = result?;
        keys.push(key);
        progress_callback(idx + 1, total);
    }

    Ok(keys)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_r2_config() {
        let config = R2Config {
            account_id: "test-account".to_string(),
            bucket_name: "forge-blobs".to_string(),
            access_key_id: "test-key".to_string(),
            secret_access_key: "test-secret".to_string(),
            custom_domain: None,
        };

        assert!(config.endpoint_url().contains("test-account"));
        assert!(config.endpoint_url().contains("r2.cloudflarestorage.com"));
    }

    #[test]
    fn test_sync_calculation() {
        let config = R2Config::default();
        let storage = R2Storage::new(config).unwrap();
        
        let remote = vec!["comp1.tsx".to_string(), "comp2.tsx".to_string()];
        let local = vec!["comp2.tsx".to_string(), "comp3.tsx".to_string()];
        
        let (download, upload) = storage.calculate_sync_actions(&remote, &local);
        
        assert_eq!(download, vec!["comp1.tsx".to_string()]);
        assert_eq!(upload, vec!["comp3.tsx".to_string()]);
    }
    
    #[test]
    fn test_sync_empty() {
        let config = R2Config::default();
        let storage = R2Storage::new(config).unwrap();
        
        let remote = vec![];
        let local = vec![];
        
        let (download, upload) = storage.calculate_sync_actions(&remote, &local);
        
        assert!(download.is_empty());
        assert!(upload.is_empty());
    }
}
