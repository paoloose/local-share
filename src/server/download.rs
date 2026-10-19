//! Binary transfer endpoints. Kept as plain axum routes (rather than server
//! functions) so the wire protocol is simple and scriptable: uploads arrive as
//! gzip bodies with metadata headers, downloads stream back with
//! `Content-Disposition`. This is also exactly the protocol the original app
//! spoke, so existing curl commands keep working.

use crate::server::services::services;
use crate::shared::naming::BucketName;
use crate::shared::types::{FileEvent, FileEventKind};
use crate::storage::BucketStore;
use axum::body::Body;
use axum::extract::{Path, Query};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use std::collections::HashMap;

const MAX_UPLOAD_BYTES: usize = 256 * 1024 * 1024;

fn error(status: StatusCode, message: &str) -> Response {
    (status, Json(serde_json::json!({ "error": message }))).into_response()
}

fn percent_decode(value: &str) -> String {
    percent_encoding::percent_decode_str(value)
        .decode_utf8_lossy()
        .to_string()
}

fn bucket_from_query(params: &HashMap<String, String>) -> Result<BucketName, Response> {
    let raw = params.get("bucket").map(String::as_str).unwrap_or_default();
    BucketName::parse(raw).map_err(|err| error(StatusCode::BAD_REQUEST, &err.to_string()))
}

/// `POST /api/upload?bucket=<name>`
///
/// Body: raw file bytes, gzip-compressed when `x-encoding: gzip` is set.
/// Headers: `x-file-name` (percent-encoded), `x-owner`, `x-content-type`.
pub async fn upload(
    Query(params): Query<HashMap<String, String>>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Response {
    let bucket = match bucket_from_query(&params) {
        Ok(bucket) => bucket,
        Err(response) => return response,
    };

    let name = header_str(&headers, "x-file-name");
    let name = percent_decode(&name).trim().to_string();
    if name.is_empty() {
        return error(StatusCode::BAD_REQUEST, "missing file name");
    }
    let owner = percent_decode(&header_str(&headers, "x-owner"));
    let content_type = header_str(&headers, "x-content-type");
    let content_type = if content_type.is_empty() {
        "application/octet-stream".to_string()
    } else {
        content_type
    };

    if body.len() > MAX_UPLOAD_BYTES {
        return error(StatusCode::PAYLOAD_TOO_LARGE, "file too large");
    }

    let raw = body.to_vec();
    let data = if header_str(&headers, "x-encoding") == "gzip" {
        use std::io::Read;
        let mut decoded = Vec::new();
        let mut decoder = flate2::read::GzDecoder::new(&raw[..]);
        if decoder.read_to_end(&mut decoded).is_err() {
            return error(StatusCode::BAD_REQUEST, "invalid gzip payload");
        }
        decoded
    } else {
        raw
    };

    match services().store.save(&bucket, &name, &owner, &content_type, data) {
        Ok(meta) => {
            services().events.publish(FileEvent {
                bucket: bucket.to_string(),
                id: meta.id.clone(),
                name: meta.name.clone(),
                kind: FileEventKind::Added,
            });
            (StatusCode::CREATED, Json(meta)).into_response()
        }
        Err(err) => error(StatusCode::INTERNAL_SERVER_ERROR, &err.to_string()),
    }
}

/// `GET /api/files/{id}/download?bucket=<name>`
///
/// Streams the stored blob gzipped with `Content-Encoding: gzip` (when the
/// client advertises support), so the browser transparently decompresses it.
pub async fn download(
    Path(id): Path<String>,
    Query(params): Query<HashMap<String, String>>,
    headers: HeaderMap,
) -> Response {
    let bucket = match bucket_from_query(&params) {
        Ok(bucket) => bucket,
        Err(response) => return response,
    };

    let path = match services().store.file_path(&bucket, &id) {
        Ok(path) => path,
        Err(err) => return error(StatusCode::NOT_FOUND, &err.to_string()),
    };
    let name = match services()
        .store
        .list(&bucket)
        .map(|files| files.into_iter().find(|meta| meta.id == id))
    {
        Ok(Some(meta)) => meta.name,
        Ok(None) => return error(StatusCode::NOT_FOUND, "file not found"),
        Err(err) => return error(StatusCode::INTERNAL_SERVER_ERROR, &err.to_string()),
    };

    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(err) => return error(StatusCode::INTERNAL_SERVER_ERROR, &err.to_string()),
    };

    let mut response_headers = HeaderMap::new();
    response_headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/octet-stream"),
    );
    response_headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response_headers.insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_str(&content_disposition(&name))
            .unwrap_or_else(|_| HeaderValue::from_static("attachment")),
    );

    let accepts_gzip = header_str(&headers, header::ACCEPT_ENCODING.as_str()).contains("gzip");
    if accepts_gzip {
        use std::io::Write;
        let mut encoder =
            flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        let compressed = encoder.write_all(&bytes).and_then(|_| encoder.finish());
        match compressed {
            Ok(compressed) => {
                response_headers
                    .insert(header::CONTENT_ENCODING, HeaderValue::from_static("gzip"));
                (StatusCode::OK, response_headers, Body::from(compressed)).into_response()
            }
            Err(err) => error(StatusCode::INTERNAL_SERVER_ERROR, &err.to_string()),
        }
    } else {
        (StatusCode::OK, response_headers, Body::from(bytes)).into_response()
    }
}

fn header_str(headers: &HeaderMap, name: &str) -> String {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_string()
}

/// RFC 5987: plain ASCII fallback plus a UTF-8 encoded `filename*`.
fn content_disposition(name: &str) -> String {
    let ascii: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_graphic() && c != '"' && c != '\\' {
                c
            } else {
                '_'
            }
        })
        .collect();
    format!(
        "attachment; filename=\"{}\"; filename*=UTF-8''{}",
        ascii,
        percent_encoding::utf8_percent_encode(name, percent_encoding::NON_ALPHANUMERIC)
    )
}
