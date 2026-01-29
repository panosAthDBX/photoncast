//! Favicon fetching and caching functionality.

use anyhow::Context;
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::time::Duration;
use url::Url;

use crate::error::Result;

/// Fetches a favicon for a URL.
///
/// Tries multiple strategies:
/// 1. /favicon.ico at the root domain
/// 2. Google's favicon service (fallback)
pub async fn fetch_favicon(url: &str, cache_dir: &Path) -> Result<Option<PathBuf>> {
    // Parse URL to get domain
    let parsed = Url::parse(url).context("invalid URL")?;

    // SSRF protection: validate the input URL before fetching
    if !is_allowed_url(&parsed) {
        tracing::warn!("Blocked favicon fetch for disallowed URL: {}", url);
        return Ok(None);
    }

    let domain = parsed.host_str().context("no host in URL")?;

    // Create cache directory if it doesn't exist
    std::fs::create_dir_all(cache_dir).context("failed to create cache directory")?;

    // Check if favicon is already cached
    let cache_path = cache_dir.join(format!("{}.png", sanitize_filename(domain)));
    if cache_path.exists() {
        return Ok(Some(cache_path));
    }

    // Try to fetch favicon
    let favicon_data = fetch_favicon_data(url).await?;

    if let Some(data) = favicon_data {
        // Save to cache
        std::fs::write(&cache_path, data).context("failed to write favicon to cache")?;
        Ok(Some(cache_path))
    } else {
        Ok(None)
    }
}

/// Fetches favicon data from URL.
async fn fetch_favicon_data(url: &str) -> Result<Option<Vec<u8>>> {
    let parsed = Url::parse(url).context("invalid URL")?;
    let scheme = parsed.scheme();
    let host = parsed.host_str().context("no host in URL")?;

    // Build favicon URLs to try
    let favicon_urls = vec![
        format!("{}://{}/favicon.ico", scheme, host),
        format!("https://www.google.com/s2/favicons?domain={}&sz=64", host),
    ];

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()
        .context("failed to build HTTP client")?;

    // Try each URL
    for favicon_url in favicon_urls {
        // Validate each favicon URL before fetching (SSRF protection)
        if let Ok(parsed_favicon_url) = Url::parse(&favicon_url) {
            if !is_allowed_url(&parsed_favicon_url) {
                tracing::warn!("Blocked favicon fetch for disallowed URL: {}", favicon_url);
                continue;
            }
        }

        if let Ok(response) = client.get(&favicon_url).send().await {
            if response.status().is_success() {
                if let Ok(data) = response.bytes().await {
                    return Ok(Some(data.to_vec()));
                }
            }
        }
    }

    Ok(None)
}

/// Checks whether a URL is safe to fetch (SSRF protection).
///
/// Rejects private/loopback IPs, non-HTTP(S) schemes, and URLs without a host.
fn is_allowed_url(url: &Url) -> bool {
    if !matches!(url.scheme(), "http" | "https") {
        return false;
    }

    let Some(host) = url.host_str() else {
        return false;
    };

    if host.eq_ignore_ascii_case("localhost") {
        return false;
    }

    if let Ok(ip) = host.parse::<IpAddr>() {
        return !is_private_ip(&ip);
    }

    true
}

const fn is_private_ip(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(addr) => addr.is_private() || addr.is_loopback() || addr.is_link_local(),
        IpAddr::V6(addr) => addr.is_loopback() || is_unique_local_v6(addr),
    }
}

const fn is_unique_local_v6(addr: &std::net::Ipv6Addr) -> bool {
    (addr.segments()[0] & 0xfe00) == 0xfc00
}

/// Sanitizes a filename by removing invalid characters.
fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("github.com"), "github.com");
        assert_eq!(sanitize_filename("example.com:8080"), "example.com_8080");
        assert_eq!(sanitize_filename("sub.domain.com"), "sub.domain.com");
    }

    #[tokio::test]
    async fn test_fetch_favicon() {
        let temp_dir = tempfile::tempdir().unwrap();
        let cache_dir = temp_dir.path();

        // Try to fetch a real favicon (this may fail in CI without network)
        let result = fetch_favicon("https://github.com", cache_dir).await;

        // We don't assert success because network may not be available
        // Just verify it doesn't panic
        let _ = result;
    }
}
