/// Blob Storage System for Cloudflare R2
///
/// This module provides efficient binary blob storage using FlatBuffers for serialization.
/// All file content and metadata are stored as binary blobs in R2, making it faster and
/// more cost-effective than traditional Git storage.
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use tokio::fs;

/// Blob metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlobMetadata {
    /// SHA-256 hash of the blob content
    pub hash: String,

    /// Original file path
    pub path: String,

    /// Blob size in bytes
    pub size: u64,

    /// Original (uncompressed) size in bytes, if compressed
    pub original_size: Option<u64>,

    /// MIME type
    pub mime_type: String,

    /// Creation timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,

    /// Compression algorithm used (if any)
    pub compression: Option<String>,
}

/// Binary blob representation
#[derive(Debug)]
pub struct Blob {
    pub metadata: BlobMetadata,
    pub content: Vec<u8>,
}

impl Blob {
    /// Create a new blob from file content
    pub async fn from_file(path: &Path) -> Result<Self> {
        let content = fs::read(path).await.context("Failed to read file")?;

        let hash = compute_hash(&content);
        let size = content.len() as u64;
        let mime_type = detect_mime_type(path);

        let metadata = BlobMetadata {
            hash: hash.clone(),
            path: path.display().to_string(),
            size,
            original_size: None,
            mime_type,
            created_at: chrono::Utc::now(),
            compression: None,
        };

        Ok(Self { metadata, content })
    }

    /// Create blob from raw content
    pub fn from_content(path: &str, content: Vec<u8>) -> Self {
        let hash = compute_hash(&content);
        let size = content.len() as u64;
        let mime_type = detect_mime_type_from_path(path);

        let metadata = BlobMetadata {
            hash: hash.clone(),
            path: path.to_string(),
            size,
            original_size: None,
            mime_type,
            created_at: chrono::Utc::now(),
            compression: None,
        };

        Self { metadata, content }
    }

    /// Serialize blob to binary format
    pub fn to_binary(&self) -> Result<Vec<u8>> {
        // Simple binary format:
        // [metadata_len: u32][metadata_json][content]

        let metadata_json = serde_json::to_vec(&self.metadata)?;
        let metadata_len = metadata_json.len() as u32;

        let mut binary = Vec::with_capacity(4 + metadata_json.len() + self.content.len());
        binary.extend_from_slice(&metadata_len.to_le_bytes());
        binary.extend_from_slice(&metadata_json);
        binary.extend_from_slice(&self.content);

        Ok(binary)
    }

    /// Deserialize blob from binary format
    pub fn from_binary(binary: &[u8]) -> Result<Self> {
        if binary.len() < 4 {
            anyhow::bail!("Invalid blob: too short");
        }

        let metadata_len =
            u32::from_le_bytes([binary[0], binary[1], binary[2], binary[3]]) as usize;

        if binary.len() < 4 + metadata_len {
            anyhow::bail!("Invalid blob: metadata truncated");
        }

        let metadata_json = &binary[4..4 + metadata_len];
        let metadata: BlobMetadata = serde_json::from_slice(metadata_json)?;

        let content = binary[4 + metadata_len..].to_vec();

        Ok(Self { metadata, content })
    }

    /// Compress blob content using LZ4
    pub fn compress(&mut self) -> Result<()> {
        if self.metadata.compression.is_some() {
            return Ok(()); // Already compressed
        }

        let compressed = lz4::block::compress(&self.content, None, false)?;

        // Only use compression if it actually reduces size
        if compressed.len() < self.content.len() {
            // Remember original size so we can safely decompress later
            self.metadata.original_size = Some(self.metadata.size);
            self.content = compressed;
            self.metadata.compression = Some("lz4".to_string());
            self.metadata.size = self.content.len() as u64;
        }

        Ok(())
    }

    /// Decompress blob content
    pub fn decompress(&mut self) -> Result<()> {
        if self.metadata.compression.is_none() {
            return Ok(()); // Not compressed
        }

        // Use the recorded original size when available to avoid
        // decompression errors due to missing size hints.
        let original_size = self
            .metadata
            .original_size
            .unwrap_or(self.metadata.size) as i32;

        let decompressed = lz4::block::decompress(&self.content, Some(original_size))?;
        self.content = decompressed;
        self.metadata.compression = None;
        self.metadata.original_size = None;
        self.metadata.size = self.content.len() as u64;

        Ok(())
    }

    /// Get blob hash (content-addressable)
    pub fn hash(&self) -> &str {
        &self.metadata.hash
    }
}

/// Compute SHA-256 hash of content
fn compute_hash(content: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content);
    format!("{:x}", hasher.finalize())
}

/// Detect MIME type from file path
fn detect_mime_type(path: &Path) -> String {
    detect_mime_type_from_path(&path.display().to_string())
}

/// Detect MIME type from path string
fn detect_mime_type_from_path(path: &str) -> String {
    let path_lower = path.to_lowercase();

    if path_lower.ends_with(".rs") {
        "text/x-rust".to_string()
    } else if path_lower.ends_with(".js") || path_lower.ends_with(".mjs") {
        "text/javascript".to_string()
    } else if path_lower.ends_with(".ts") {
        "text/typescript".to_string()
    } else if path_lower.ends_with(".tsx") {
        "text/tsx".to_string()
    } else if path_lower.ends_with(".json") {
        "application/json".to_string()
    } else if path_lower.ends_with(".md") {
        "text/markdown".to_string()
    } else if path_lower.ends_with(".html") {
        "text/html".to_string()
    } else if path_lower.ends_with(".css") {
        "text/css".to_string()
    } else if path_lower.ends_with(".toml") {
        "application/toml".to_string()
    } else if path_lower.ends_with(".yaml") || path_lower.ends_with(".yml") {
        "application/yaml".to_string()
    } else {
        "application/octet-stream".to_string()
    }
}

/// Blob repository for local caching
pub struct BlobRepository {
    cache_dir: PathBuf,
}

impl BlobRepository {
    /// Create new blob repository
    pub fn new(forge_dir: &Path) -> Result<Self> {
        let cache_dir = forge_dir.join("blobs");
        std::fs::create_dir_all(&cache_dir)?;

        Ok(Self { cache_dir })
    }

    /// Store blob locally
    pub async fn store_local(&self, blob: &Blob) -> Result<()> {
        let hash = blob.hash();
        let blob_path = self.get_blob_path(hash);

        // Create directory structure (first 2 chars of hash)
        if let Some(parent) = blob_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        let binary = blob.to_binary()?;
        fs::write(&blob_path, binary).await?;

        Ok(())
    }

    /// Load blob from local cache
    pub async fn load_local(&self, hash: &str) -> Result<Blob> {
        let blob_path = self.get_blob_path(hash);
        let binary = fs::read(&blob_path)
            .await
            .context("Blob not found in cache")?;

        Blob::from_binary(&binary)
    }

    /// Check if blob exists locally
    pub async fn exists_local(&self, hash: &str) -> bool {
        self.get_blob_path(hash).exists()
    }

    /// Get blob storage path (content-addressable)
    fn get_blob_path(&self, hash: &str) -> PathBuf {
        // Store blobs like Git: .dx/forge/blobs/ab/cdef1234...
        let prefix = &hash[..2];
        let suffix = &hash[2..];
        self.cache_dir.join(prefix).join(suffix)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_blob_serialization() {
        let content = b"Hello, world!".to_vec();
        let blob = Blob::from_content("test.txt", content.clone());

        let binary = blob.to_binary().unwrap();
        let restored = Blob::from_binary(&binary).unwrap();

        assert_eq!(blob.metadata.hash, restored.metadata.hash);
        assert_eq!(blob.content, restored.content);
        assert_eq!(blob.metadata.path, restored.metadata.path);
    }

    #[tokio::test]
    async fn test_blob_compression() {
        let content = b"Hello, world! ".repeat(1000);
        let mut blob = Blob::from_content("test.txt", content.clone());

        let original_size = blob.metadata.size;
        blob.compress().unwrap();
        let compressed_size = blob.metadata.size;

        assert!(compressed_size < original_size);
        assert_eq!(blob.metadata.compression, Some("lz4".to_string()));

        blob.decompress().unwrap();
        assert_eq!(blob.content, content);
        assert_eq!(blob.metadata.compression, None);
    }
}
