//! Bucket storage: where dropped files live on disk.
//!
//! Layout (unchanged from the original app, so existing data keeps working):
//!
//! ```text
//! buckets/
//!   <bucket>/
//!     meta.json          # manifest: Vec<FileMeta>
//!     files/<id>         # raw bytes of each dropped file
//! ```
//!
//! Buckets are created on first write: listing a bucket that does not exist
//! simply returns an empty list and touches nothing.

use crate::shared::naming::BucketName;
use crate::shared::types::FileMeta;
use std::path::PathBuf;

mod fs_store;
pub use fs_store::FsStore;

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("invalid bucket name: {0}")]
    InvalidBucket(String),
    #[error("file not found: {0}")]
    NotFound(String),
    #[error("corrupt manifest for bucket {bucket}: {source}")]
    CorruptManifest {
        bucket: String,
        source: serde_json::Error,
    },
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

pub trait BucketStore {
    /// Existing bucket names, sorted. Never creates anything.
    fn list_buckets(&self) -> Result<Vec<String>, StoreError>;

    /// Files in a bucket, newest first. Missing buckets read as empty.
    fn list(&self, bucket: &BucketName) -> Result<Vec<FileMeta>, StoreError>;

    /// Absolute path of a stored blob, for streaming downloads.
    fn file_path(&self, bucket: &BucketName, id: &str) -> Result<PathBuf, StoreError>;

    /// Stores bytes under a fresh id, creating the bucket on first write.
    fn save(
        &self,
        bucket: &BucketName,
        name: &str,
        owner: &str,
        content_type: &str,
        data: Vec<u8>,
    ) -> Result<FileMeta, StoreError>;

    fn rename(
        &self,
        bucket: &BucketName,
        id: &str,
        name: &str,
    ) -> Result<FileMeta, StoreError>;

    fn delete(&self, bucket: &BucketName, id: &str) -> Result<(), StoreError>;
}
