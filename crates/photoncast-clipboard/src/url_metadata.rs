//! URL metadata fetching for clipboard links.
//!
//! Fetches page titles and favicons for URL clipboard items.

use std::path::{Path, PathBuf};
use std::time::Duration;

use tracing::debug;

use crate::error::{ClipboardError, Result};

/// Timeout for HTTP requests.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);

/// Maximum HTML size to fetch for title extraction.
const MAX_HTML_SIZE: usize = 50 * 1024; // 50KB

/// URL metadata.
#[derive(Debug, Clone)]
pub struct UrlMetadata {
    /// Page title.
    pub title: Option<String>,
    /// Path to cached favicon.
    pub favicon_path: Option<PathBuf>,
}

/// Fetcher for URL metadata.
#[derive(Debug)]
pub struct UrlMetadataFetcher {
    client: reqwest::Client,
}

impl Default for UrlMetadataFetcher {
    fn default() -> Self {
        Self::new()
    }
}

impl UrlMetadataFetcher {
    /// Creates a new URL metadata fetcher.
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) PhotonCast/1.0")
            .redirect(reqwest::redirect::Policy::limited(5))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self { client }
    }

    /// Fetches metadata for a URL.
    ///
    /// # Arguments
    ///
    /// * `url` - The URL to fetch metadata for.
    /// * `cache_dir` - Directory to cache favicons.
    ///
    /// # Returns
    ///
    /// Returns the fetched metadata or an error.
    pub async fn fetch(&self, url: &str, cache_dir: &Path) -> Result<UrlMetadata> {
        let parsed_url = url::Url::parse(url)
            .map_err(|e| ClipboardError::url_metadata(format!("Invalid URL: {}", e)))?;

        if !is_allowed_url(&parsed_url) {
            return Err(ClipboardError::url_metadata(
                "URL scheme or host not allowed",
            ));
        }

        // Fetch page HTML for title
        let title = self.fetch_title(&parsed_url).await.ok();

        // Fetch favicon
        let favicon_path = self
            .fetch_favicon(&parsed_url, cache_dir)
            .await
            .ok()
            .flatten();

        Ok(UrlMetadata {
            title,
            favicon_path,
        })
    }

    /// Fetches the page title.
    async fn fetch_title(&self, url: &url::Url) -> Result<String> {
        debug!("Fetching title for {}", url);

        let response = self.client.get(url.as_str()).send().await?;

        if !response.status().is_success() {
            return Err(ClipboardError::url_metadata(format!(
                "HTTP error: {}",
                response.status()
            )));
        }

        // Read limited amount of HTML
        let content_length = response.content_length().unwrap_or(0) as usize;
        let bytes = if content_length > MAX_HTML_SIZE {
            // Stream and limit
            let bytes = response.bytes().await?;
            bytes.slice(0..MAX_HTML_SIZE.min(bytes.len()))
        } else {
            response.bytes().await?
        };

        let html = String::from_utf8_lossy(&bytes);

        // Extract title
        extract_title(&html).ok_or_else(|| ClipboardError::url_metadata("No title found"))
    }

    /// Fetches the favicon.
    async fn fetch_favicon(&self, url: &url::Url, cache_dir: &Path) -> Result<Option<PathBuf>> {
        debug!("Fetching favicon for {}", url);

        // Try common favicon locations
        let favicon_urls = vec![
            format!(
                "{}://{}/favicon.ico",
                url.scheme(),
                url.host_str().unwrap_or("")
            ),
            format!(
                "{}://{}/favicon.png",
                url.scheme(),
                url.host_str().unwrap_or("")
            ),
        ];

        for favicon_url in favicon_urls {
            if let Ok(response) = self.client.get(&favicon_url).send().await {
                if response.status().is_success() {
                    if let Ok(bytes) = response.bytes().await {
                        // Generate cache filename from URL
                        let hash = simple_hash(url.as_str());
                        let ext = if std::path::Path::new(&favicon_url)
                            .extension()
                            .is_some_and(|ext| ext.eq_ignore_ascii_case("png"))
                        {
                            "png"
                        } else {
                            "ico"
                        };
                        let filename = format!("favicon_{}.{}", hash, ext);
                        let path = cache_dir.join(filename);

                        if matches!(std::fs::write(&path, &bytes), Ok(())) {
                            return Ok(Some(path));
                        }
                    }
                }
            }
        }

        Ok(None)
    }
}

/// Extracts the title from HTML.
fn extract_title(html: &str) -> Option<String> {
    // Simple regex-free extraction
    let lower = html.to_lowercase();

    // Find <title> tag
    let start = lower.find("<title")?;
    let tag_end = lower[start..].find('>')? + start + 1;
    let end = lower[tag_end..].find("</title>")? + tag_end;

    let title = html[tag_end..end].trim();

    // Decode common HTML entities
    let decoded = decode_html_entities(title);

    if decoded.is_empty() {
        None
    } else {
        Some(decoded)
    }
}

/// Decodes common HTML entities.
fn decode_html_entities(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&nbsp;", " ")
}

/// Simple hash function for cache filenames.
fn simple_hash(s: &str) -> u64 {
    let mut hash: u64 = 5381;
    for byte in s.bytes() {
        hash = hash.wrapping_mul(33).wrapping_add(u64::from(byte));
    }
    hash
}

fn is_allowed_url(url: &url::Url) -> bool {
    if !matches!(url.scheme(), "http" | "https") {
        return false;
    }

    let Some(host) = url.host_str() else {
        return false;
    };

    if host.eq_ignore_ascii_case("localhost") {
        return false;
    }

    if let Ok(ip) = host.parse::<std::net::IpAddr>() {
        return !is_private_ip(&ip);
    }

    true
}

const fn is_private_ip(ip: &std::net::IpAddr) -> bool {
    match ip {
        std::net::IpAddr::V4(addr) => {
            addr.is_private() || addr.is_loopback() || addr.is_link_local()
        },
        std::net::IpAddr::V6(addr) => addr.is_loopback() || is_unique_local_v6(addr),
    }
}

const fn is_unique_local_v6(addr: &std::net::Ipv6Addr) -> bool {
    (addr.segments()[0] & 0xfe00) == 0xfc00
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_title() {
        let html = r"<html><head><title>Example Title</title></head></html>";
        assert_eq!(extract_title(html), Some("Example Title".to_string()));

        let html = r"<html><head><title>   Trimmed Title   </title></head></html>";
        assert_eq!(extract_title(html), Some("Trimmed Title".to_string()));

        let html = r"<html><head><title></title></head></html>";
        assert_eq!(extract_title(html), None);

        let html = r"<html><head></head></html>";
        assert_eq!(extract_title(html), None);
    }

    #[test]
    fn test_extract_title_with_entities() {
        let html = r"<title>Test &amp; Example &lt;Title&gt;</title>";
        assert_eq!(
            extract_title(html),
            Some("Test & Example <Title>".to_string())
        );
    }

    #[test]
    fn test_decode_html_entities() {
        assert_eq!(decode_html_entities("&amp;"), "&");
        assert_eq!(decode_html_entities("&lt;test&gt;"), "<test>");
        assert_eq!(decode_html_entities("hello&nbsp;world"), "hello world");
    }

    #[test]
    fn test_simple_hash() {
        let hash1 = simple_hash("https://example.com");
        let hash2 = simple_hash("https://example.com");
        let hash3 = simple_hash("https://different.com");

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_fetcher_creation() {
        let fetcher = UrlMetadataFetcher::new();
        // Just test that it creates successfully
        assert!(std::mem::size_of_val(&fetcher) > 0);
    }

    #[tokio::test]
    async fn test_fetch_invalid_url() {
        let fetcher = UrlMetadataFetcher::new();
        let result = fetcher.fetch("not a valid url", Path::new("/tmp")).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_fetch_rejects_localhost() {
        let fetcher = UrlMetadataFetcher::new();
        let result = fetcher
            .fetch("http://localhost:8080", Path::new("/tmp"))
            .await;
        assert!(result.is_err());
    }
}
