//! Cloud Recording Infrastructure
//!
//! Local storage backend for HORUS recordings with support for:
//! - Streaming uploads during recording
//! - Chunked/resumable downloads
//! - Gzip compression for efficient storage
//! - Metadata indexing and search
//!
//! ## Usage
//!
//! ```rust,ignore
//! use horus_core::scheduling::cloud_recording::*;
//! use std::path::Path;
//!
//! // Configure local storage backend
//! let config = CloudConfig::local(Path::new("/recordings"), "robot-001/");
//!
//! // Create uploader
//! let uploader = CloudUploader::new(config)?;
//!
//! // Upload a recording
//! uploader.upload_recording(&recording_path, Some("session-123"))?;
//!
//! // Stream upload during recording
//! let mut streaming = uploader.start_streaming_upload("session-123")?;
//! streaming.upload_chunk(&data)?;
//! streaming.finalize()?;
//! ```

use std::collections::HashMap;
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64};
use std::sync::Arc;
use std::time::Instant;

use chrono::{DateTime, Utc};
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Simple hash function for checksums (DJB2 algorithm)
fn simple_hash(data: &[u8]) -> u32 {
    let mut hash: u32 = 5381;
    for &byte in data {
        hash = hash.wrapping_mul(33).wrapping_add(byte as u32);
    }
    hash
}

/// Errors that can occur during cloud operations
#[derive(Error, Debug)]
pub enum CloudError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("Cloud storage error: {0}")]
    Storage(String),

    #[error("Authentication error: {0}")]
    Auth(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Upload failed: {0}")]
    UploadFailed(String),

    #[error("Download failed: {0}")]
    DownloadFailed(String),

    #[error("Recording not found: {0}")]
    NotFound(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Compression error: {0}")]
    Compression(String),
}

pub type Result<T> = std::result::Result<T, CloudError>;

/// Cloud storage provider
///
/// Currently only local filesystem is supported. Cloud providers (S3, GCS, Azure)
/// would require additional dependencies and are not implemented.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CloudProvider {
    /// Local filesystem storage
    Local { base_path: PathBuf },
}

/// Cloud storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudConfig {
    /// Storage provider
    pub provider: CloudProvider,
    /// Base prefix/path for recordings
    pub prefix: String,
    /// Compression level (0 = none, 1-9 = gzip levels)
    pub compression_level: u32,
    /// Chunk size for multipart uploads (default: 5MB)
    pub chunk_size: usize,
    /// Maximum concurrent uploads
    pub max_concurrent_uploads: usize,
    /// Enable automatic retry on failure
    pub auto_retry: bool,
    /// Maximum retry attempts
    pub max_retries: u32,
    /// Retry delay in milliseconds
    pub retry_delay_ms: u64,
    /// Upload timeout per chunk (seconds)
    pub chunk_timeout_secs: u64,
    /// Enable server-side encryption
    pub enable_encryption: bool,
    /// Custom metadata to add to all uploads
    pub custom_metadata: HashMap<String, String>,
}

impl CloudConfig {
    /// Create local filesystem configuration
    pub fn local(base_path: &Path, prefix: &str) -> Self {
        Self {
            provider: CloudProvider::Local {
                base_path: base_path.to_path_buf(),
            },
            prefix: prefix.to_string(),
            compression_level: 6,
            chunk_size: 5 * 1024 * 1024,
            max_concurrent_uploads: 4,
            auto_retry: true,
            max_retries: 3,
            retry_delay_ms: 1000,
            chunk_timeout_secs: 60,
            enable_encryption: false,
            custom_metadata: HashMap::new(),
        }
    }

    /// Set compression level
    pub fn with_compression(mut self, level: u32) -> Self {
        self.compression_level = level.min(9);
        self
    }

    /// Set chunk size for multipart uploads
    pub fn with_chunk_size(mut self, size: usize) -> Self {
        self.chunk_size = size.max(1024 * 1024); // Minimum 1MB
        self
    }

    /// Enable server-side encryption
    pub fn with_encryption(mut self) -> Self {
        self.enable_encryption = true;
        self
    }

    /// Add custom metadata
    pub fn with_metadata(mut self, key: &str, value: &str) -> Self {
        self.custom_metadata
            .insert(key.to_string(), value.to_string());
        self
    }
}

/// Recording metadata stored in cloud
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudRecordingMetadata {
    /// Unique recording ID
    pub recording_id: String,
    /// Session ID
    pub session_id: String,
    /// Robot/node name
    pub robot_name: Option<String>,
    /// Recording start time
    pub started_at: DateTime<Utc>,
    /// Recording end time
    pub ended_at: Option<DateTime<Utc>>,
    /// Total duration in seconds
    pub duration_secs: f64,
    /// Total size in bytes (compressed)
    pub compressed_size: u64,
    /// Total size in bytes (uncompressed)
    pub uncompressed_size: u64,
    /// Number of ticks
    pub total_ticks: u64,
    /// Topics recorded
    pub topics: Vec<String>,
    /// Recording type (standard, zero_copy, distributed)
    pub recording_type: String,
    /// Custom tags
    pub tags: HashMap<String, String>,
    /// Cloud storage path
    pub cloud_path: String,
    /// Parts/chunks in multipart upload
    pub parts: Vec<CloudPart>,
}

/// Part of a multipart upload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudPart {
    pub part_number: u32,
    pub size: u64,
    pub etag: Option<String>,
    pub checksum: Option<String>,
}

/// Upload progress information
#[derive(Debug, Clone)]
pub struct UploadProgress {
    /// Total bytes to upload
    pub total_bytes: u64,
    /// Bytes uploaded so far
    pub uploaded_bytes: u64,
    /// Number of parts completed
    pub parts_completed: u32,
    /// Total parts
    pub total_parts: u32,
    /// Upload speed in bytes/second
    pub speed_bps: f64,
    /// Estimated time remaining in seconds
    pub eta_secs: f64,
    /// Current status
    pub status: UploadStatus,
}

/// Upload status
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UploadStatus {
    Initializing,
    Uploading,
    Finalizing,
    Completed,
    Failed(String),
    Cancelled,
}

/// Cloud storage backend trait
pub trait CloudBackend: Send + Sync {
    /// Initialize connection
    fn init(&mut self) -> Result<()>;

    /// Upload a single file
    fn upload_file(&self, local_path: &Path, cloud_path: &str) -> Result<String>;

    /// Start multipart upload
    fn start_multipart(&self, cloud_path: &str) -> Result<String>;

    /// Upload a part
    fn upload_part(&self, upload_id: &str, part_number: u32, data: &[u8]) -> Result<CloudPart>;

    /// Complete multipart upload
    fn complete_multipart(
        &self,
        upload_id: &str,
        cloud_path: &str,
        parts: &[CloudPart],
    ) -> Result<()>;

    /// Abort multipart upload
    fn abort_multipart(&self, upload_id: &str, cloud_path: &str) -> Result<()>;

    /// Download a file
    fn download_file(&self, cloud_path: &str, local_path: &Path) -> Result<()>;

    /// Download a byte range
    fn download_range(&self, cloud_path: &str, start: u64, end: u64) -> Result<Vec<u8>>;

    /// Check if file exists
    fn exists(&self, cloud_path: &str) -> Result<bool>;

    /// Delete a file
    fn delete(&self, cloud_path: &str) -> Result<()>;

    /// List files with prefix
    fn list(&self, prefix: &str) -> Result<Vec<String>>;

    /// Get file metadata
    fn head(&self, cloud_path: &str) -> Result<HashMap<String, String>>;
}

/// Local filesystem backend (for testing)
pub struct LocalBackend {
    base_path: PathBuf,
}

impl LocalBackend {
    pub fn new(base_path: &Path) -> Self {
        Self {
            base_path: base_path.to_path_buf(),
        }
    }

    fn full_path(&self, cloud_path: &str) -> PathBuf {
        self.base_path.join(cloud_path)
    }
}

impl CloudBackend for LocalBackend {
    fn init(&mut self) -> Result<()> {
        std::fs::create_dir_all(&self.base_path)?;
        Ok(())
    }

    fn upload_file(&self, local_path: &Path, cloud_path: &str) -> Result<String> {
        let dest = self.full_path(cloud_path);
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::copy(local_path, &dest)?;
        Ok(cloud_path.to_string())
    }

    fn start_multipart(&self, cloud_path: &str) -> Result<String> {
        // For local backend, create a temp directory for parts
        let upload_id = format!("local_upload_{}", uuid::Uuid::new_v4());
        let parts_dir = self.base_path.join(".uploads").join(&upload_id);
        std::fs::create_dir_all(&parts_dir)?;

        // Store the target path
        std::fs::write(parts_dir.join("target"), cloud_path)?;

        Ok(upload_id)
    }

    fn upload_part(&self, upload_id: &str, part_number: u32, data: &[u8]) -> Result<CloudPart> {
        let parts_dir = self.base_path.join(".uploads").join(upload_id);
        let part_path = parts_dir.join(format!("part_{:05}", part_number));
        std::fs::write(&part_path, data)?;

        Ok(CloudPart {
            part_number,
            size: data.len() as u64,
            etag: None,
            checksum: Some(format!("{:x}", simple_hash(data))),
        })
    }

    fn complete_multipart(
        &self,
        upload_id: &str,
        cloud_path: &str,
        parts: &[CloudPart],
    ) -> Result<()> {
        let parts_dir = self.base_path.join(".uploads").join(upload_id);
        let dest = self.full_path(cloud_path);

        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Concatenate all parts
        let mut output = File::create(&dest)?;
        for part in parts {
            let part_path = parts_dir.join(format!("part_{:05}", part.part_number));
            let mut input = File::open(&part_path)?;
            std::io::copy(&mut input, &mut output)?;
        }

        // Clean up temp directory
        std::fs::remove_dir_all(&parts_dir)?;

        Ok(())
    }

    fn abort_multipart(&self, upload_id: &str, _cloud_path: &str) -> Result<()> {
        let parts_dir = self.base_path.join(".uploads").join(upload_id);
        if parts_dir.exists() {
            std::fs::remove_dir_all(&parts_dir)?;
        }
        Ok(())
    }

    fn download_file(&self, cloud_path: &str, local_path: &Path) -> Result<()> {
        let src = self.full_path(cloud_path);
        if let Some(parent) = local_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::copy(&src, local_path)?;
        Ok(())
    }

    fn download_range(&self, cloud_path: &str, start: u64, end: u64) -> Result<Vec<u8>> {
        let path = self.full_path(cloud_path);
        let mut file = File::open(&path)?;
        file.seek(std::io::SeekFrom::Start(start))?;

        let len = (end - start) as usize;
        let mut buffer = vec![0u8; len];
        file.read_exact(&mut buffer)?;

        Ok(buffer)
    }

    fn exists(&self, cloud_path: &str) -> Result<bool> {
        Ok(self.full_path(cloud_path).exists())
    }

    fn delete(&self, cloud_path: &str) -> Result<()> {
        let path = self.full_path(cloud_path);
        if path.exists() {
            std::fs::remove_file(&path)?;
        }
        Ok(())
    }

    fn list(&self, prefix: &str) -> Result<Vec<String>> {
        let search_path = self.full_path(prefix);

        let mut results = Vec::new();
        if search_path.exists() && search_path.is_dir() {
            // List entries directly in this directory (files and subdirectories)
            for entry in std::fs::read_dir(&search_path)? {
                let entry = entry?;
                let path = entry.path();
                if let Ok(rel) = path.strip_prefix(&self.base_path) {
                    results.push(rel.to_string_lossy().to_string());
                }
            }
        } else if search_path.exists() && search_path.is_file() {
            // If prefix is a file, return it directly
            if let Ok(rel) = search_path.strip_prefix(&self.base_path) {
                results.push(rel.to_string_lossy().to_string());
            }
        }

        Ok(results)
    }

    fn head(&self, cloud_path: &str) -> Result<HashMap<String, String>> {
        let path = self.full_path(cloud_path);
        let metadata = std::fs::metadata(&path)?;

        let mut result = HashMap::new();
        result.insert("size".to_string(), metadata.len().to_string());
        if let Ok(modified) = metadata.modified() {
            let datetime: DateTime<Utc> = modified.into();
            result.insert("last-modified".to_string(), datetime.to_rfc3339());
        }

        Ok(result)
    }
}

use std::io::Seek;

/// Cloud uploader for recordings
pub struct CloudUploader {
    config: CloudConfig,
    backend: Box<dyn CloudBackend>,
    #[allow(dead_code)] // Reserved for future multipart upload tracking
    active_uploads: Arc<RwLock<HashMap<String, ActiveUpload>>>,
}

#[allow(dead_code)] // Reserved for future multipart upload progress tracking
struct ActiveUpload {
    upload_id: String,
    cloud_path: String,
    parts: Vec<CloudPart>,
    uploaded_bytes: AtomicU64,
    total_bytes: u64,
    started_at: Instant,
    cancelled: AtomicBool,
}

impl CloudUploader {
    /// Create a new cloud uploader
    pub fn new(config: CloudConfig) -> Result<Self> {
        let CloudProvider::Local { ref base_path } = config.provider;
        let mut backend = LocalBackend::new(base_path);
        backend.init()?;
        let backend: Box<dyn CloudBackend> = Box::new(backend);

        Ok(Self {
            config,
            backend,
            active_uploads: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Upload a recording directory
    pub fn upload_recording(
        &self,
        local_path: &Path,
        cloud_key: Option<&str>,
    ) -> Result<CloudRecordingMetadata> {
        // Determine cloud path
        let key = cloud_key.unwrap_or_else(|| {
            local_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("recording")
        });
        let cloud_path = format!("{}/{}", self.config.prefix, key);

        // Read local metadata if available
        let metadata_path = local_path.join("metadata.json");
        let local_metadata: Option<serde_json::Value> = if metadata_path.exists() {
            std::fs::read_to_string(&metadata_path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
        } else {
            None
        };

        // Collect files to upload
        let mut files_to_upload = Vec::new();
        let mut total_size: u64 = 0;

        for entry in std::fs::read_dir(local_path)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                let size = std::fs::metadata(&path)?.len();
                total_size += size;
                files_to_upload.push((path, size));
            }
        }

        // Upload each file (with optional compression)
        let mut compressed_size: u64 = 0;
        let mut uploaded_files = Vec::new();

        for (file_path, _size) in &files_to_upload {
            let file_name = file_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown");

            let file_cloud_path = format!("{}/{}", cloud_path, file_name);

            // Compress if enabled and file is data
            if self.config.compression_level > 0
                && (file_name.ends_with(".bin") || file_name == "data.bin")
            {
                let compressed = self.compress_file(&file_path)?;
                let compressed_path = format!("{}.gz", file_cloud_path);

                // Upload compressed data using multipart for large files
                if compressed.len() > self.config.chunk_size {
                    self.upload_multipart(&compressed, &compressed_path)?;
                } else {
                    // For small files, use temp file
                    let temp_path = std::env::temp_dir()
                        .join(format!("horus_upload_{}.gz", uuid::Uuid::new_v4()));
                    std::fs::write(&temp_path, &compressed)?;
                    self.backend.upload_file(&temp_path, &compressed_path)?;
                    std::fs::remove_file(&temp_path)?;
                }

                compressed_size += compressed.len() as u64;
                uploaded_files.push(compressed_path);
            } else {
                self.backend.upload_file(&file_path, &file_cloud_path)?;
                compressed_size += std::fs::metadata(&file_path)?.len();
                uploaded_files.push(file_cloud_path);
            }
        }

        // Create cloud metadata
        let metadata = CloudRecordingMetadata {
            recording_id: uuid::Uuid::new_v4().to_string(),
            session_id: local_metadata
                .as_ref()
                .and_then(|m| m.get("session_id"))
                .and_then(|v| v.as_str())
                .unwrap_or(key)
                .to_string(),
            robot_name: local_metadata
                .as_ref()
                .and_then(|m| m.get("node_name"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            started_at: Utc::now(),
            ended_at: Some(Utc::now()),
            duration_secs: 0.0,
            compressed_size,
            uncompressed_size: total_size,
            total_ticks: local_metadata
                .as_ref()
                .and_then(|m| m.get("total_ticks"))
                .and_then(|v| v.as_u64())
                .unwrap_or(0),
            topics: local_metadata
                .as_ref()
                .and_then(|m| m.get("topics"))
                .and_then(|v| v.as_object())
                .map(|o| o.keys().cloned().collect())
                .unwrap_or_default(),
            recording_type: "standard".to_string(),
            tags: self.config.custom_metadata.clone(),
            cloud_path: cloud_path.clone(),
            parts: Vec::new(),
        };

        // Upload metadata
        let metadata_json = serde_json::to_string_pretty(&metadata)
            .map_err(|e| CloudError::Serialization(e.to_string()))?;
        let metadata_cloud_path = format!("{}/cloud_metadata.json", cloud_path);
        let temp_path =
            std::env::temp_dir().join(format!("horus_metadata_{}.json", uuid::Uuid::new_v4()));
        std::fs::write(&temp_path, &metadata_json)?;
        self.backend.upload_file(&temp_path, &metadata_cloud_path)?;
        std::fs::remove_file(&temp_path)?;

        Ok(metadata)
    }

    /// Compress a file
    fn compress_file(&self, path: &Path) -> Result<Vec<u8>> {
        let input = std::fs::read(path)?;
        let mut encoder =
            GzEncoder::new(Vec::new(), Compression::new(self.config.compression_level));
        encoder
            .write_all(&input)
            .map_err(|e| CloudError::Compression(e.to_string()))?;
        encoder
            .finish()
            .map_err(|e| CloudError::Compression(e.to_string()))
    }

    /// Upload data using multipart upload
    fn upload_multipart(&self, data: &[u8], cloud_path: &str) -> Result<()> {
        let upload_id = self.backend.start_multipart(cloud_path)?;

        let mut parts = Vec::new();
        let mut offset = 0;
        let mut part_number = 1u32;

        while offset < data.len() {
            let end = (offset + self.config.chunk_size).min(data.len());
            let chunk = &data[offset..end];

            let part = self.backend.upload_part(&upload_id, part_number, chunk)?;
            parts.push(part);

            offset = end;
            part_number += 1;
        }

        self.backend
            .complete_multipart(&upload_id, cloud_path, &parts)?;

        Ok(())
    }

    /// Start streaming upload for a recording session
    pub fn start_streaming_upload(&self, session_id: &str) -> Result<StreamingUploader> {
        let cloud_path = format!("{}/{}/data.bin", self.config.prefix, session_id);
        let upload_id = self.backend.start_multipart(&cloud_path)?;

        Ok(StreamingUploader {
            upload_id,
            cloud_path,
            config: self.config.clone(),
            backend: self.backend.as_ref(),
            parts: Vec::new(),
            buffer: Vec::with_capacity(self.config.chunk_size),
            total_uploaded: 0,
            part_number: 1,
        })
    }

    /// Download a recording
    pub fn download_recording(&self, cloud_key: &str, local_path: &Path) -> Result<()> {
        let cloud_path = format!("{}/{}", self.config.prefix, cloud_key);

        std::fs::create_dir_all(local_path)?;

        // List entries in the cloud path
        let entries = self.backend.list(&cloud_path)?;

        for entry in entries {
            let entry_name = entry
                .strip_prefix(&cloud_path)
                .map(|s| s.trim_start_matches('/'))
                .unwrap_or(&entry);

            // Skip empty names (this happens when the entry IS the cloud_path)
            if entry_name.is_empty() {
                continue;
            }

            let local_file = local_path.join(entry_name);

            // Try to download - if it fails because it's a directory, skip it
            let download_result = if entry.ends_with(".gz") {
                // Download and decompress
                let temp_path = std::env::temp_dir()
                    .join(format!("horus_download_{}.gz", uuid::Uuid::new_v4()));
                match self.backend.download_file(&entry, &temp_path) {
                    Ok(_) => {
                        let compressed = std::fs::read(&temp_path)?;
                        let mut decoder = GzDecoder::new(&compressed[..]);
                        let mut decompressed = Vec::new();
                        decoder
                            .read_to_end(&mut decompressed)
                            .map_err(|e| CloudError::Compression(e.to_string()))?;

                        let decompressed_name = entry_name.trim_end_matches(".gz");
                        std::fs::write(local_path.join(decompressed_name), &decompressed)?;
                        std::fs::remove_file(&temp_path)?;
                        Ok(())
                    }
                    Err(e) => Err(e),
                }
            } else {
                self.backend.download_file(&entry, &local_file)
            };

            // Skip directories (they fail to copy)
            if let Err(CloudError::Io(ref e)) = download_result {
                if e.kind() == std::io::ErrorKind::InvalidInput {
                    continue; // Skip directories
                }
            }
            download_result?;
        }

        Ok(())
    }

    /// List recordings
    pub fn list_recordings(&self) -> Result<Vec<String>> {
        self.backend.list(&self.config.prefix)
    }

    /// Delete a recording
    pub fn delete_recording(&self, cloud_key: &str) -> Result<()> {
        let cloud_path = format!("{}/{}", self.config.prefix, cloud_key);
        let files = self.backend.list(&cloud_path)?;

        for file in files {
            self.backend.delete(&file)?;
        }

        Ok(())
    }

    /// Get recording metadata
    pub fn get_metadata(&self, cloud_key: &str) -> Result<CloudRecordingMetadata> {
        let metadata_path = format!("{}/{}/cloud_metadata.json", self.config.prefix, cloud_key);

        let temp_path =
            std::env::temp_dir().join(format!("horus_meta_{}.json", uuid::Uuid::new_v4()));
        self.backend.download_file(&metadata_path, &temp_path)?;

        let content = std::fs::read_to_string(&temp_path)?;
        std::fs::remove_file(&temp_path)?;

        serde_json::from_str(&content).map_err(|e| CloudError::Serialization(e.to_string()))
    }
}

/// Streaming uploader for real-time recording upload
pub struct StreamingUploader<'a> {
    upload_id: String,
    cloud_path: String,
    config: CloudConfig,
    backend: &'a dyn CloudBackend,
    parts: Vec<CloudPart>,
    buffer: Vec<u8>,
    total_uploaded: u64,
    part_number: u32,
}

impl<'a> StreamingUploader<'a> {
    /// Upload a chunk of data
    pub fn upload_chunk(&mut self, data: &[u8]) -> Result<()> {
        self.buffer.extend_from_slice(data);

        // Upload when buffer exceeds chunk size
        while self.buffer.len() >= self.config.chunk_size {
            let chunk: Vec<u8> = self.buffer.drain(..self.config.chunk_size).collect();
            self.upload_buffer_chunk(&chunk)?;
        }

        Ok(())
    }

    fn upload_buffer_chunk(&mut self, chunk: &[u8]) -> Result<()> {
        // Optionally compress
        let data = if self.config.compression_level > 0 {
            let mut encoder =
                GzEncoder::new(Vec::new(), Compression::new(self.config.compression_level));
            encoder
                .write_all(chunk)
                .map_err(|e| CloudError::Compression(e.to_string()))?;
            encoder
                .finish()
                .map_err(|e| CloudError::Compression(e.to_string()))?
        } else {
            chunk.to_vec()
        };

        let part = self
            .backend
            .upload_part(&self.upload_id, self.part_number, &data)?;
        self.parts.push(part);
        self.total_uploaded += data.len() as u64;
        self.part_number += 1;

        Ok(())
    }

    /// Get current progress
    pub fn progress(&self) -> UploadProgress {
        UploadProgress {
            total_bytes: 0, // Unknown for streaming
            uploaded_bytes: self.total_uploaded,
            parts_completed: self.parts.len() as u32,
            total_parts: 0, // Unknown for streaming
            speed_bps: 0.0,
            eta_secs: 0.0,
            status: UploadStatus::Uploading,
        }
    }

    /// Finalize the upload
    pub fn finalize(mut self) -> Result<Vec<CloudPart>> {
        // Upload remaining buffer
        if !self.buffer.is_empty() {
            let remaining = std::mem::take(&mut self.buffer);
            self.upload_buffer_chunk(&remaining)?;
        }

        self.backend
            .complete_multipart(&self.upload_id, &self.cloud_path, &self.parts)?;

        Ok(self.parts)
    }

    /// Abort the upload
    pub fn abort(self) -> Result<()> {
        self.backend
            .abort_multipart(&self.upload_id, &self.cloud_path)
    }
}

/// Cloud recording index for searching
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudRecordingIndex {
    /// All indexed recordings
    pub recordings: Vec<CloudRecordingMetadata>,
    /// Index by session ID
    pub by_session: HashMap<String, Vec<usize>>,
    /// Index by robot name
    pub by_robot: HashMap<String, Vec<usize>>,
    /// Index by date
    pub by_date: HashMap<String, Vec<usize>>,
    /// Index by tag
    pub by_tag: HashMap<String, Vec<usize>>,
}

impl CloudRecordingIndex {
    /// Create a new empty index
    pub fn new() -> Self {
        Self {
            recordings: Vec::new(),
            by_session: HashMap::new(),
            by_robot: HashMap::new(),
            by_date: HashMap::new(),
            by_tag: HashMap::new(),
        }
    }

    /// Add a recording to the index
    pub fn add(&mut self, metadata: CloudRecordingMetadata) {
        let idx = self.recordings.len();

        // Index by session
        self.by_session
            .entry(metadata.session_id.clone())
            .or_default()
            .push(idx);

        // Index by robot
        if let Some(ref robot) = metadata.robot_name {
            self.by_robot.entry(robot.clone()).or_default().push(idx);
        }

        // Index by date
        let date = metadata.started_at.format("%Y-%m-%d").to_string();
        self.by_date.entry(date).or_default().push(idx);

        // Index by tags
        for (key, value) in &metadata.tags {
            let tag = format!("{}:{}", key, value);
            self.by_tag.entry(tag).or_default().push(idx);
        }

        self.recordings.push(metadata);
    }

    /// Search recordings
    pub fn search(&self, query: &RecordingQuery) -> Vec<&CloudRecordingMetadata> {
        let mut results: Vec<usize> = (0..self.recordings.len()).collect();

        // Filter by session
        if let Some(ref session) = query.session_id {
            if let Some(indices) = self.by_session.get(session) {
                results.retain(|i| indices.contains(i));
            } else {
                results.clear();
            }
        }

        // Filter by robot
        if let Some(ref robot) = query.robot_name {
            if let Some(indices) = self.by_robot.get(robot) {
                results.retain(|i| indices.contains(i));
            } else {
                results.clear();
            }
        }

        // Filter by date range
        if let Some(ref start) = query.start_date {
            results.retain(|&i| self.recordings[i].started_at >= *start);
        }
        if let Some(ref end) = query.end_date {
            results.retain(|&i| self.recordings[i].started_at <= *end);
        }

        // Filter by tags
        for (key, value) in &query.tags {
            let tag = format!("{}:{}", key, value);
            if let Some(indices) = self.by_tag.get(&tag) {
                results.retain(|i| indices.contains(i));
            } else {
                results.clear();
                break;
            }
        }

        results.into_iter().map(|i| &self.recordings[i]).collect()
    }
}

impl Default for CloudRecordingIndex {
    fn default() -> Self {
        Self::new()
    }
}

/// Query for searching recordings
#[derive(Debug, Clone, Default)]
pub struct RecordingQuery {
    pub session_id: Option<String>,
    pub robot_name: Option<String>,
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub tags: HashMap<String, String>,
    pub min_duration_secs: Option<f64>,
    pub max_duration_secs: Option<f64>,
}

impl RecordingQuery {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn session(mut self, session_id: &str) -> Self {
        self.session_id = Some(session_id.to_string());
        self
    }

    pub fn robot(mut self, robot_name: &str) -> Self {
        self.robot_name = Some(robot_name.to_string());
        self
    }

    pub fn after(mut self, date: DateTime<Utc>) -> Self {
        self.start_date = Some(date);
        self
    }

    pub fn before(mut self, date: DateTime<Utc>) -> Self {
        self.end_date = Some(date);
        self
    }

    pub fn tag(mut self, key: &str, value: &str) -> Self {
        self.tags.insert(key.to_string(), value.to_string());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_local_backend() -> Result<()> {
        let temp_dir = env::temp_dir().join(format!("horus_cloud_test_{}", uuid::Uuid::new_v4()));
        let config = CloudConfig::local(&temp_dir, "recordings");

        let uploader = CloudUploader::new(config)?;

        // Create a test recording
        let recording_dir = temp_dir.join("test_recording");
        std::fs::create_dir_all(&recording_dir)?;
        std::fs::write(recording_dir.join("data.bin"), b"test data")?;
        std::fs::write(recording_dir.join("index.bin"), b"test index")?;
        std::fs::write(
            recording_dir.join("metadata.json"),
            r#"{"session_id":"test","total_ticks":100}"#,
        )?;

        // Upload
        let metadata = uploader.upload_recording(&recording_dir, Some("test-session"))?;
        assert_eq!(metadata.session_id, "test");
        assert_eq!(metadata.total_ticks, 100);

        // List
        let recordings = uploader.list_recordings()?;
        assert!(!recordings.is_empty());

        // Download
        let download_dir = temp_dir.join("downloaded");
        uploader.download_recording("test-session", &download_dir)?;
        assert!(download_dir.join("metadata.json").exists());

        // Cleanup
        std::fs::remove_dir_all(&temp_dir)?;

        Ok(())
    }

    #[test]
    fn test_multipart_upload() -> Result<()> {
        let temp_dir =
            env::temp_dir().join(format!("horus_multipart_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir)?;

        let mut backend = LocalBackend::new(&temp_dir);
        backend.init()?;

        // Start multipart
        let upload_id = backend.start_multipart("test/multipart.bin")?;

        // Upload parts
        let part1 = backend.upload_part(&upload_id, 1, b"part one data")?;
        let part2 = backend.upload_part(&upload_id, 2, b"part two data")?;

        // Complete
        backend.complete_multipart(&upload_id, "test/multipart.bin", &[part1, part2])?;

        // Verify
        let content = std::fs::read(temp_dir.join("test/multipart.bin"))?;
        assert_eq!(content, b"part one datapart two data");

        // Cleanup
        std::fs::remove_dir_all(&temp_dir)?;

        Ok(())
    }

    #[test]
    fn test_streaming_upload() -> Result<()> {
        let temp_dir =
            env::temp_dir().join(format!("horus_streaming_test_{}", uuid::Uuid::new_v4()));
        let config = CloudConfig::local(&temp_dir, "recordings").with_chunk_size(100); // Small chunks for testing

        let uploader = CloudUploader::new(config)?;

        let mut streaming = uploader.start_streaming_upload("streaming-session")?;

        // Upload in chunks
        for i in 0..10 {
            let data = format!("chunk {} data ", i);
            streaming.upload_chunk(data.as_bytes())?;
        }

        let parts = streaming.finalize()?;
        assert!(!parts.is_empty());

        // Verify file was created
        let file_path = temp_dir.join("recordings/streaming-session/data.bin");
        assert!(file_path.exists());

        // Cleanup
        std::fs::remove_dir_all(&temp_dir)?;

        Ok(())
    }

    #[test]
    fn test_recording_index() {
        let mut index = CloudRecordingIndex::new();

        // Add some recordings
        index.add(CloudRecordingMetadata {
            recording_id: "rec1".to_string(),
            session_id: "session-001".to_string(),
            robot_name: Some("robot-A".to_string()),
            started_at: Utc::now(),
            ended_at: None,
            duration_secs: 60.0,
            compressed_size: 1000,
            uncompressed_size: 2000,
            total_ticks: 100,
            topics: vec!["sensor/imu".to_string()],
            recording_type: "standard".to_string(),
            tags: [("env".to_string(), "test".to_string())]
                .into_iter()
                .collect(),
            cloud_path: "recordings/session-001".to_string(),
            parts: Vec::new(),
        });

        index.add(CloudRecordingMetadata {
            recording_id: "rec2".to_string(),
            session_id: "session-002".to_string(),
            robot_name: Some("robot-A".to_string()),
            started_at: Utc::now(),
            ended_at: None,
            duration_secs: 120.0,
            compressed_size: 2000,
            uncompressed_size: 4000,
            total_ticks: 200,
            topics: vec!["sensor/imu".to_string(), "motor/cmd".to_string()],
            recording_type: "standard".to_string(),
            tags: [("env".to_string(), "prod".to_string())]
                .into_iter()
                .collect(),
            cloud_path: "recordings/session-002".to_string(),
            parts: Vec::new(),
        });

        // Search by robot
        let results = index.search(&RecordingQuery::new().robot("robot-A"));
        assert_eq!(results.len(), 2);

        // Search by session
        let results = index.search(&RecordingQuery::new().session("session-001"));
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].recording_id, "rec1");

        // Search by tag
        let results = index.search(&RecordingQuery::new().tag("env", "prod"));
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].recording_id, "rec2");
    }
}
