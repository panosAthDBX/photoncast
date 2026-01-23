//! TOML file I/O for quick links.

use anyhow::Context;
use std::path::Path;

use crate::error::Result;
use crate::models::QuickLinksToml;

/// Writes quick links to a TOML file.
pub fn write_toml<P: AsRef<Path>>(path: P, toml: &QuickLinksToml) -> Result<()> {
    let toml_str =
        toml::to_string_pretty(toml).context("failed to serialize quick links to TOML")?;

    std::fs::write(path.as_ref(), toml_str).context("failed to write TOML file")?;

    Ok(())
}

/// Reads quick links from a TOML file.
pub fn read_toml<P: AsRef<Path>>(path: P) -> Result<QuickLinksToml> {
    let toml_str = std::fs::read_to_string(path.as_ref()).context("failed to read TOML file")?;

    let toml: QuickLinksToml = toml::from_str(&toml_str).context("failed to parse TOML")?;

    Ok(toml)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::QuickLinkToml;
    use tempfile::tempdir;

    #[test]
    fn test_toml_roundtrip() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("quicklinks.toml");

        let toml = QuickLinksToml {
            links: vec![
                QuickLinkToml {
                    name: "GitHub".to_string(),
                    link: "https://github.com".to_string(),
                    alias: Some("gh".to_string()),
                    icon: Some("🐙".to_string()),
                    open_with: None,
                    hotkey: None,
                    keywords: vec!["gh".to_string(), "git".to_string()],
                    tags: vec!["dev".to_string()],
                },
                QuickLinkToml {
                    name: "Google Search".to_string(),
                    link: "https://google.com/search?q={argument}".to_string(),
                    alias: Some("g".to_string()),
                    icon: None,
                    open_with: Some("com.apple.Safari".to_string()),
                    hotkey: Some("cmd+shift+g".to_string()),
                    keywords: vec![],
                    tags: vec![],
                },
            ],
        };

        // Write
        write_toml(&path, &toml).unwrap();

        // Read
        let loaded = read_toml(&path).unwrap();
        assert_eq!(loaded.links.len(), 2);
        assert_eq!(loaded.links[0].name, "GitHub");
        assert_eq!(loaded.links[0].alias, Some("gh".to_string()));
        assert_eq!(
            loaded.links[1].link,
            "https://google.com/search?q={argument}"
        );
        assert_eq!(
            loaded.links[1].open_with,
            Some("com.apple.Safari".to_string())
        );
        assert_eq!(loaded.links[1].hotkey, Some("cmd+shift+g".to_string()));
    }
}
