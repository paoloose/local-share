use serde::{Deserialize, Serialize};
use std::fmt;

/// Maximum bucket name length, matching the old app.
pub const MAX_BUCKET_LEN: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NamingError(String);

impl fmt::Display for NamingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for NamingError {}

/// A validated bucket name: the on-disk directory name under `buckets/`.
///
/// Rules (same as the old app): starts with a letter or digit, then letters,
/// digits, dashes or underscores, at most 64 characters.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BucketName(String);

impl BucketName {
    pub fn parse(raw: &str) -> Result<Self, NamingError> {
        let name = raw.trim();
        if name.is_empty() {
            return Err(NamingError("bucket name is empty".into()));
        }
        if name.len() > MAX_BUCKET_LEN {
            return Err(NamingError(format!(
                "bucket name is longer than {MAX_BUCKET_LEN} characters"
            )));
        }
        let mut chars = name.chars();
        let first = chars.next().expect("non-empty");
        if !first.is_ascii_alphanumeric() {
            return Err(NamingError(
                "bucket names must start with a letter or digit".into(),
            ));
        }
        if !chars.all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
            return Err(NamingError(
                "bucket names can only use letters, numbers, dashes and underscores".into(),
            ));
        }
        Ok(Self(name.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for BucketName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Resolves whatever the user typed into a bucket name.
///
/// Accepts a bare name (`andes`), a full URL (`https://drive.paoloose.site/andes`),
/// a scheme-less link (`drive.paoloose.site/andes`), a deep path (first segment
/// wins), or a self-referential link with `?bucket=`.
pub fn normalize_bucket_input(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let looks_like_link = trimmed.contains("://") || trimmed.contains('/');
    if !looks_like_link {
        return trimmed.to_string();
    }
    let with_scheme = if trimmed.contains("://") {
        trimmed.to_string()
    } else {
        format!("https://{trimmed}")
    };
    match Url::parse(&with_scheme) {
        Ok(url) => {
            if let Some(from_query) = url
                .query_pairs()
                .find(|(key, _)| key == "bucket")
                .map(|(_, value)| value.to_string())
            {
                return from_query.trim().to_string();
            }
            url.path()
                .split('/')
                .find(|segment| !segment.is_empty())
                .unwrap_or(trimmed)
                .to_string()
        }
        Err(_) => trimmed
            .split(['/', '?', '#'])
            .find(|segment| !segment.is_empty())
            .unwrap_or(trimmed)
            .to_string(),
    }
}

/// Normalizes the input and validates the result in one step.
pub fn bucket_from_input(raw: &str) -> Result<BucketName, NamingError> {
    BucketName::parse(&normalize_bucket_input(raw))
}

// `url` re-exported privately so tests and callers stay decoupled from the crate.
use url::Url;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plain_names() {
        assert_eq!(BucketName::parse("andes").unwrap().as_str(), "andes");
        assert_eq!(BucketName::parse(" team-dump_2 ").unwrap().as_str(), "team-dump_2");
    }

    #[test]
    fn rejects_invalid_names() {
        assert!(BucketName::parse("").is_err());
        assert!(BucketName::parse(".hidden").is_err());
        assert!(BucketName::parse("has space").is_err());
        assert!(BucketName::parse("has/slash").is_err());
        assert!(BucketName::parse("-leading-dash").is_err());
        assert!(BucketName::parse(&"x".repeat(65)).is_err());
    }

    #[test]
    fn normalizes_urls_to_their_bucket_segment() {
        assert_eq!(
            normalize_bucket_input("https://drive.paoloose.site/andes"),
            "andes"
        );
        assert_eq!(normalize_bucket_input("drive.paoloose.site/andes"), "andes");
        assert_eq!(
            normalize_bucket_input("https://drive.paoloose.site/andes/"),
            "andes"
        );
        assert_eq!(
            normalize_bucket_input("https://drive.paoloose.site/andes/notes.txt"),
            "andes"
        );
        assert_eq!(
            normalize_bucket_input("http://localhost:7777/?bucket=andes"),
            "andes"
        );
        assert_eq!(normalize_bucket_input("andes"), "andes");
        assert_eq!(normalize_bucket_input("  andes  "), "andes");
    }

    #[test]
    fn bucket_from_input_round_trips() {
        assert_eq!(
            bucket_from_input("https://drive.paoloose.site/andes")
                .unwrap()
                .as_str(),
            "andes"
        );
        assert!(bucket_from_input("https://drive.paoloose.site/has space").is_err());
    }
}
