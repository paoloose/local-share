//! Leptos server functions: the metadata API. The UI calls these as plain
//! async functions; they execute on the server. Binary transfers (upload and
//! download) live in `crate::server::download` as regular axum routes.

use crate::shared::types::FileMeta;
#[cfg(feature = "ssr")]
use crate::storage::BucketStore;
use leptos::prelude::*;

/// Pins the custom error type so error mapping needs no turbofish.
#[cfg(feature = "ssr")]
fn server_error(err: impl std::fmt::Display) -> ServerFnError {
    ServerFnError::ServerError(err.to_string())
}

#[server(prefix = "/api", endpoint = "list_files")]
pub async fn list_files(bucket: String) -> Result<Vec<FileMeta>, ServerFnError> {
    use crate::server::services::services;
    use crate::shared::naming::BucketName;

    let bucket = BucketName::parse(&bucket)
        .map_err(server_error)?;
    services()
        .store
        .list(&bucket)
        .map_err(server_error)
}

#[server(prefix = "/api", endpoint = "list_buckets")]
pub async fn list_buckets() -> Result<Vec<String>, ServerFnError> {
    use crate::server::services::services;

    services()
        .store
        .list_buckets()
        .map_err(server_error)
}

#[server(prefix = "/api", endpoint = "rename_file")]
pub async fn rename_file(bucket: String, id: String, name: String) -> Result<FileMeta, ServerFnError> {
    use crate::server::services::services;
    use crate::shared::naming::BucketName;
    use crate::shared::types::{FileEvent, FileEventKind};

    let bucket = BucketName::parse(&bucket)
        .map_err(server_error)?;
    let meta = services()
        .store
        .rename(&bucket, &id, &name)
        .map_err(server_error)?;
    services().events.publish(FileEvent {
        bucket: bucket.to_string(),
        id: meta.id.clone(),
        name: meta.name.clone(),
        kind: FileEventKind::Renamed,
    });
    Ok(meta)
}

#[server(prefix = "/api", endpoint = "delete_file")]
pub async fn delete_file(bucket: String, id: String) -> Result<(), ServerFnError> {
    use crate::server::services::services;
    use crate::shared::naming::BucketName;
    use crate::shared::types::{FileEvent, FileEventKind};

    let bucket = BucketName::parse(&bucket)
        .map_err(server_error)?;
    services()
        .store
        .delete(&bucket, &id)
        .map_err(server_error)?;
    services().events.publish(FileEvent {
        bucket: bucket.to_string(),
        id: id.clone(),
        name: String::new(),
        kind: FileEventKind::Removed,
    });
    Ok(())
}
