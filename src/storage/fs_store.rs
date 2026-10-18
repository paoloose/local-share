use super::{BucketStore, StoreError};
use crate::shared::naming::BucketName;
use crate::shared::types::{guess_content_type, FileMeta};
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

/// Filesystem-backed store rooted at the `buckets/` directory.
#[derive(Debug, Clone)]
pub struct FsStore {
    root: PathBuf,
}

impl FsStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Registers a blob that appeared in `files/` out of band (someone copied
    /// it in with the filesystem), so it shows up like any dropped file. The
    /// file name doubles as its id; existing entries are returned as-is.
    pub fn register_external(&self, bucket: &BucketName, id: &str) -> Result<FileMeta, StoreError> {
        let mut files = self.read_manifest(bucket)?;
        if let Some(existing) = files.iter().find(|meta| meta.id == id) {
            return Ok(existing.clone());
        }
        let path = self.files_dir(bucket).join(id);
        let size = fs::metadata(&path)?.len();
        let meta = FileMeta {
            id: id.to_string(),
            name: id.to_string(),
            owner: "disk".to_string(),
            content_type: guess_content_type(id),
            size,
            uploaded_at: Self::now_rfc3339(),
        };
        files.push(meta.clone());
        files.sort_by(|a, b| b.uploaded_at.cmp(&a.uploaded_at));
        self.write_manifest(bucket, &files)?;
        Ok(meta)
    }

    fn bucket_dir(&self, bucket: &BucketName) -> PathBuf {
        self.root.join(bucket.as_str())
    }

    fn files_dir(&self, bucket: &BucketName) -> PathBuf {
        self.bucket_dir(bucket).join("files")
    }

    fn manifest_path(&self, bucket: &BucketName) -> PathBuf {
        self.bucket_dir(bucket).join("meta.json")
    }

    fn read_manifest(&self, bucket: &BucketName) -> Result<Vec<FileMeta>, StoreError> {
        let path = self.manifest_path(bucket);
        if !path.exists() {
            return Ok(Vec::new());
        }
        let raw = fs::read(&path)?;
        serde_json::from_slice(&raw).map_err(|source| StoreError::CorruptManifest {
            bucket: bucket.to_string(),
            source,
        })
    }

    /// Writes the manifest via a temp file + rename so readers never see a
    /// half-written document.
    fn write_manifest(&self, bucket: &BucketName, files: &[FileMeta]) -> Result<(), StoreError> {
        let dir = self.bucket_dir(bucket);
        fs::create_dir_all(&dir)?;
        let tmp = dir.join("meta.json.tmp");
        let mut handle = fs::File::create(&tmp)?;
        serde_json::to_writer_pretty(&mut handle, files)
            .map_err(|source| StoreError::CorruptManifest {
                bucket: bucket.to_string(),
                source,
            })?;
        handle.write_all(b"\n")?;
        handle.sync_all()?;
        fs::rename(&tmp, self.manifest_path(bucket))?;
        Ok(())
    }

    fn write_blob(&self, bucket: &BucketName, id: &str, data: &[u8]) -> Result<(), StoreError> {
        let dir = self.files_dir(bucket);
        fs::create_dir_all(&dir)?;
        let tmp = dir.join(format!("{id}.tmp"));
        let mut handle = fs::File::create(&tmp)?;
        handle.write_all(data)?;
        handle.sync_all()?;
        fs::rename(&tmp, dir.join(id))?;
        Ok(())
    }

    fn now_rfc3339() -> String {
        OffsetDateTime::now_utc()
            .format(&Rfc3339)
            .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
    }
}

impl BucketStore for FsStore {
    fn list_buckets(&self) -> Result<Vec<String>, StoreError> {
        let mut names = Vec::new();
        if let Ok(entries) = fs::read_dir(&self.root) {
            for entry in entries.flatten() {
                if !entry.path().is_dir() {
                    continue;
                }
                if let Some(name) = entry.file_name().to_str() {
                    if BucketName::parse(name).is_ok() {
                        names.push(name.to_string());
                    }
                }
            }
        }
        names.sort();
        Ok(names)
    }

    fn list(&self, bucket: &BucketName) -> Result<Vec<FileMeta>, StoreError> {
        let mut files = self.read_manifest(bucket)?;
        files.sort_by(|a, b| b.uploaded_at.cmp(&a.uploaded_at));
        Ok(files)
    }

    fn file_path(&self, bucket: &BucketName, id: &str) -> Result<PathBuf, StoreError> {
        let files = self.read_manifest(bucket)?;
        if !files.iter().any(|meta| meta.id == id) {
            return Err(StoreError::NotFound(id.to_string()));
        }
        let path = self.files_dir(bucket).join(id);
        if !path.is_file() {
            return Err(StoreError::NotFound(id.to_string()));
        }
        Ok(path)
    }

    fn save(
        &self,
        bucket: &BucketName,
        name: &str,
        owner: &str,
        content_type: &str,
        data: Vec<u8>,
    ) -> Result<FileMeta, StoreError> {
        let meta = FileMeta {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.trim().chars().take(255).collect(),
            owner: owner.trim().chars().take(64).collect(),
            content_type: content_type.to_string(),
            size: data.len() as u64,
            uploaded_at: Self::now_rfc3339(),
        };
        self.write_blob(bucket, &meta.id, &data)?;
        let mut files = self.read_manifest(bucket)?;
        files.push(meta.clone());
        files.sort_by(|a, b| b.uploaded_at.cmp(&a.uploaded_at));
        self.write_manifest(bucket, &files)?;
        Ok(meta)
    }

    fn rename(&self, bucket: &BucketName, id: &str, name: &str) -> Result<FileMeta, StoreError> {
        let mut files = self.read_manifest(bucket)?;
        let meta = files
            .iter_mut()
            .find(|meta| meta.id == id)
            .ok_or_else(|| StoreError::NotFound(id.to_string()))?;
        meta.name = name.trim().chars().take(255).collect();
        let updated = meta.clone();
        files.sort_by(|a, b| b.uploaded_at.cmp(&a.uploaded_at));
        self.write_manifest(bucket, &files)?;
        Ok(updated)
    }

    fn delete(&self, bucket: &BucketName, id: &str) -> Result<(), StoreError> {
        let mut files = self.read_manifest(bucket)?;
        let before = files.len();
        files.retain(|meta| meta.id != id);
        if files.len() == before {
            return Err(StoreError::NotFound(id.to_string()));
        }
        self.write_manifest(bucket, &files)?;
        let _ = fs::remove_file(self.files_dir(bucket).join(id));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn meta_by_name<'a>(files: &'a [FileMeta], name: &str) -> Option<&'a FileMeta> {
        files.iter().find(|meta| meta.name == name)
    }

    #[test]
    fn missing_bucket_lists_empty_and_stays_missing() {
        let dir = tempfile::tempdir().unwrap();
        let store = FsStore::new(dir.path());
        let bucket = BucketName::parse("andes").unwrap();

        assert!(store.list(&bucket).unwrap().is_empty());
        assert!(store.list_buckets().unwrap().is_empty());
        assert!(
            !dir.path().join("andes").exists(),
            "listing must not create buckets"
        );
    }

    #[test]
    fn save_creates_the_bucket_on_write() {
        let dir = tempfile::tempdir().unwrap();
        let store = FsStore::new(dir.path());
        let bucket = BucketName::parse("andes").unwrap();

        let meta = store
            .save(&bucket, "hello.txt", "paoloose", "text/plain", b"hi".to_vec())
            .unwrap();

        assert!(dir.path().join("andes/meta.json").is_file());
        assert!(dir.path().join("andes/files").join(&meta.id).is_file());
        assert_eq!(store.list_buckets().unwrap(), vec!["andes".to_string()]);
        assert_eq!(store.list(&bucket).unwrap()[0].name, "hello.txt");
    }

    #[test]
    fn list_sorts_newest_first() {
        let dir = tempfile::tempdir().unwrap();
        let store = FsStore::new(dir.path());
        let bucket = BucketName::parse("andes").unwrap();

        store.save(&bucket, "first.txt", "a", "text/plain", vec![]).unwrap();
        store.save(&bucket, "second.txt", "b", "text/plain", vec![]).unwrap();
        store.save(&bucket, "third.txt", "c", "text/plain", vec![]).unwrap();

        let names: Vec<_> = store
            .list(&bucket)
            .unwrap()
            .iter()
            .map(|meta| meta.name.clone())
            .collect();
        assert_eq!(names, vec!["third.txt", "second.txt", "first.txt"]);
    }

    #[test]
    fn rename_and_delete_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let store = FsStore::new(dir.path());
        let bucket = BucketName::parse("team-dump").unwrap();
        let meta = store
            .save(&bucket, "old name.txt", "paoloose", "text/plain", b"data".to_vec())
            .unwrap();

        let renamed = store.rename(&bucket, &meta.id, "new name.txt").unwrap();
        assert_eq!(renamed.name, "new name.txt");
        assert_eq!(meta_by_name(&store.list(&bucket).unwrap(), "new name.txt").is_some(), true);

        store.delete(&bucket, &meta.id).unwrap();
        assert!(store.list(&bucket).unwrap().is_empty());
        assert!(matches!(
            store.delete(&bucket, &meta.id),
            Err(StoreError::NotFound(_))
        ));
    }

    #[test]
    fn handles_unicode_names() {
        let dir = tempfile::tempdir().unwrap();
        let store = FsStore::new(dir.path());
        let bucket = BucketName::parse("andes").unwrap();
        let meta = store
            .save(&bucket, "résumé ü.txt", "paoloose", "text/plain", b"x".to_vec())
            .unwrap();
        assert_eq!(meta.name, "résumé ü.txt");
        assert_eq!(store.list(&bucket).unwrap()[0].name, "résumé ü.txt");
    }

    #[test]
    fn external_files_can_register_and_leave() {
        let dir = tempfile::tempdir().unwrap();
        let store = FsStore::new(dir.path());
        let bucket = BucketName::parse("andes").unwrap();

        fs::create_dir_all(dir.path().join("andes/files")).unwrap();
        fs::write(dir.path().join("andes/files/report.pdf"), b"pdf").unwrap();

        let meta = store.register_external(&bucket, "report.pdf").unwrap();
        assert_eq!(meta.name, "report.pdf");
        assert_eq!(meta.owner, "disk");
        assert_eq!(meta.content_type, "application/pdf");
        assert_eq!(meta.size, 3);
        assert_eq!(store.list(&bucket).unwrap().len(), 1);

        // registering twice must not duplicate
        store.register_external(&bucket, "report.pdf").unwrap();
        assert_eq!(store.list(&bucket).unwrap().len(), 1);

        store.delete(&bucket, "report.pdf").unwrap();
        assert!(store.list(&bucket).unwrap().is_empty());
    }

    #[test]
    fn file_path_is_only_available_for_known_ids() {
        let dir = tempfile::tempdir().unwrap();
        let store = FsStore::new(dir.path());
        let bucket = BucketName::parse("andes").unwrap();
        let meta = store
            .save(&bucket, "a.txt", "paoloose", "text/plain", b"x".to_vec())
            .unwrap();

        assert!(store.file_path(&bucket, &meta.id).is_ok());
        assert!(matches!(
            store.file_path(&bucket, "nope"),
            Err(StoreError::NotFound(_))
        ));
    }
}
