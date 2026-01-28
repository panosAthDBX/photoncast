//! Conference URL detection.

use regex::Regex;

/// Conference provider patterns.
static ZOOM_PATTERN: std::sync::LazyLock<Regex> =
    std::sync::LazyLock::new(|| Regex::new(r"https?://[a-z0-9.-]*zoom\.us/(j/|my/)[a-zA-Z0-9?&=/._-]+").unwrap());

static GOOGLE_MEET_PATTERN: std::sync::LazyLock<Regex> =
    std::sync::LazyLock::new(|| Regex::new(r"https?://meet\.google\.com/[a-z-]+").unwrap());

static TEAMS_PATTERN: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r"https?://teams\.microsoft\.com/l/meetup-join/[a-zA-Z0-9?&=/._%-]+").unwrap()
});

/// Detects and extracts conference URLs from event fields.
///
/// Searches in location, notes, and structured conference data.
#[must_use]
pub fn detect_conference_url(location: Option<&str>, notes: Option<&str>) -> Option<String> {
    // Check location first
    if let Some(loc) = location {
        if let Some(url) = extract_url(loc) {
            return Some(url);
        }
    }

    // Check notes
    if let Some(n) = notes {
        if let Some(url) = extract_url(n) {
            return Some(url);
        }
    }

    None
}

/// Extracts a conference URL from text.
fn extract_url(text: &str) -> Option<String> {
    // Try Zoom
    if let Some(mat) = ZOOM_PATTERN.find(text) {
        return Some(mat.as_str().to_string());
    }

    // Try Google Meet
    if let Some(mat) = GOOGLE_MEET_PATTERN.find(text) {
        return Some(mat.as_str().to_string());
    }

    // Try Microsoft Teams
    if let Some(mat) = TEAMS_PATTERN.find(text) {
        return Some(mat.as_str().to_string());
    }

    None
}

/// Conference provider type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConferenceProvider {
    /// Zoom meeting.
    Zoom,
    /// Google Meet.
    GoogleMeet,
    /// Microsoft Teams.
    MicrosoftTeams,
    /// Unknown/other provider.
    Other,
}

/// Detects the conference provider from a URL.
#[must_use]
pub fn detect_provider(url: &str) -> ConferenceProvider {
    if url.contains("zoom.us") {
        ConferenceProvider::Zoom
    } else if url.contains("meet.google.com") {
        ConferenceProvider::GoogleMeet
    } else if url.contains("teams.microsoft.com") {
        ConferenceProvider::MicrosoftTeams
    } else {
        ConferenceProvider::Other
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zoom_detection() {
        let location = "Zoom meeting: https://zoom.us/j/123456789";
        let url = detect_conference_url(Some(location), None);
        assert_eq!(url, Some("https://zoom.us/j/123456789".to_string()));

        let provider = detect_provider(&url.unwrap());
        assert_eq!(provider, ConferenceProvider::Zoom);
    }

    #[test]
    fn test_google_meet_detection() {
        let notes = "Join the meeting: https://meet.google.com/abc-defg-hij";
        let url = detect_conference_url(None, Some(notes));
        assert_eq!(
            url,
            Some("https://meet.google.com/abc-defg-hij".to_string())
        );

        let provider = detect_provider(&url.unwrap());
        assert_eq!(provider, ConferenceProvider::GoogleMeet);
    }

    #[test]
    fn test_teams_detection() {
        let location =
            "Microsoft Teams: https://teams.microsoft.com/l/meetup-join/19%3ameeting_abc123";
        let url = detect_conference_url(Some(location), None);
        assert!(url.is_some());
        assert!(url.as_ref().unwrap().contains("teams.microsoft.com"));

        let provider = detect_provider(&url.unwrap());
        assert_eq!(provider, ConferenceProvider::MicrosoftTeams);
    }

    #[test]
    fn test_no_conference_url() {
        let location = "Conference Room A";
        let notes = "Please bring your laptop";
        let url = detect_conference_url(Some(location), Some(notes));
        assert_eq!(url, None);
    }

    #[test]
    fn test_url_in_notes() {
        let notes = "Meeting agenda:\n1. Review\n2. Discuss\n\nJoin: https://zoom.us/j/987654321";
        let url = detect_conference_url(None, Some(notes));
        assert_eq!(url, Some("https://zoom.us/j/987654321".to_string()));
    }
}
