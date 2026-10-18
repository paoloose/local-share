use serde::{Deserialize, Serialize};

/// Metadata for one dropped file, as stored in the bucket manifest and sent
/// over the wire. `uploaded_at` is an RFC 3339 UTC timestamp so lexicographic
/// sorting matches chronological sorting.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileMeta {
    pub id: String,
    pub name: String,
    pub owner: String,
    pub content_type: String,
    pub size: u64,
    pub uploaded_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum SortMode {
    #[default]
    DateDesc,
    DateAsc,
    Owner,
    Name,
    Type,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FileEventKind {
    Added,
    Renamed,
    Removed,
    Changed,
}

/// Published on every mutation (and by the filesystem watcher) so open
/// clients hot reload their file list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileEvent {
    pub bucket: String,
    pub id: String,
    pub name: String,
    pub kind: FileEventKind,
}

impl FileMeta {
    /// File extension in lowercase, or a label derived from the content type.
    /// Mirrors the old UI's type badge ("pdf", "zip", "file", ...).
    pub fn type_label(&self) -> String {
        match self.name.rsplit_once('.') {
            Some((stem, ext)) if !stem.is_empty() && !ext.is_empty() && ext.len() <= 5 => {
                ext.to_lowercase()
            }
            _ => self
                .content_type
                .split('/')
                .nth(1)
                .unwrap_or("file")
                .to_lowercase(),
        }
    }
}

/// Best-effort content type from a file name, for files copied straight into
/// a bucket folder on disk.
pub fn guess_content_type(name: &str) -> String {
    let ext = name
        .rsplit_once('.')
        .map(|(_, ext)| ext.to_lowercase())
        .unwrap_or_default();
    match ext.as_str() {
        "txt" | "md" | "log" => "text/plain",
        "html" | "htm" => "text/html",
        "csv" => "text/csv",
        "json" => "application/json",
        "pdf" => "application/pdf",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "zip" => "application/zip",
        "gz" => "application/gzip",
        _ => "application/octet-stream",
    }
    .to_string()
}

/// Human size like the old UI: "812 B", "1.4 KB", "3.2 MB".
pub fn format_size(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;
    let b = bytes as f64;
    if b < KB {
        format!("{bytes} B")
    } else if b < MB {
        format!("{:.1} KB", b / KB)
    } else if b < GB {
        format!("{:.1} MB", b / MB)
    } else {
        format!("{:.2} GB", b / GB)
    }
}

/// Formats an RFC 3339 UTC timestamp as "Oct 17, 2026, 03:53 PM", matching the
/// old UI's date column without pulling a date crate into the browser bundle.
pub fn format_when(iso: &str) -> String {
    let bytes = iso.as_bytes();
    if bytes.len() < 19 {
        return iso.to_string();
    }
    let year = &iso[0..4];
    let month: u32 = iso[5..7].parse().unwrap_or(1);
    let day = &iso[8..10];
    let mut hour: u32 = iso[11..13].parse().unwrap_or(0);
    let minute = &iso[14..16];
    let meridiem = if hour >= 12 { "PM" } else { "AM" };
    hour = hour % 12;
    if hour == 0 {
        hour = 12;
    }
    let month_name = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ][(month.clamp(1, 12) - 1) as usize];
    format!("{month_name} {day}, {year}, {hour:02}:{minute} {meridiem}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size_formatting() {
        assert_eq!(format_size(512), "512 B");
        assert_eq!(format_size(2048), "2.0 KB");
        assert_eq!(format_size(3 * 1024 * 1024), "3.0 MB");
        assert_eq!(format_size(2 * 1024 * 1024 * 1024), "2.00 GB");
    }

    #[test]
    fn when_formatting() {
        assert_eq!(format_when("2026-10-17T15:53:31Z"), "Oct 17, 2026, 03:53 PM");
        assert_eq!(format_when("2026-10-17T03:04:00Z"), "Oct 17, 2026, 03:04 AM");
        assert_eq!(format_when("2026-10-17T12:00:00Z"), "Oct 17, 2026, 12:00 PM");
    }

    #[test]
    fn guessed_content_types() {
        assert_eq!(guess_content_type("notes.txt"), "text/plain");
        assert_eq!(guess_content_type("PHOTO.JPG"), "image/jpeg");
        assert_eq!(guess_content_type("archive.unknown"), "application/octet-stream");
        assert_eq!(guess_content_type("README"), "application/octet-stream");
    }

    #[test]
    fn type_labels() {
        let meta = |name: &str, ct: &str| FileMeta {
            id: "x".into(),
            name: name.into(),
            owner: "paoloose".into(),
            content_type: ct.into(),
            size: 1,
            uploaded_at: "2026-10-17T15:53:31Z".into(),
        };
        assert_eq!(meta("notes.PDF", "application/pdf").type_label(), "pdf");
        assert_eq!(meta("archive.tar", "application/x-tar").type_label(), "tar");
        assert_eq!(meta("README", "text/plain").type_label(), "plain");
        assert_eq!(meta(".bashrc", "text/plain").type_label(), "plain");
    }
}
