# PhotonCast Quick Links - Raycast Parity Spec

> **Version:** 1.0.0  
> **Date:** 2026-01-21  
> **Goal:** Match Raycast quicklinks behavior exactly

---

## Overview

Quick Links provide instant access to frequently visited URLs, files, and folders with support for dynamic placeholders. This spec updates the existing quicklinks implementation to achieve full parity with Raycast's feature set.

---

## 1. Core Features

### 1.1 What Quick Links Can Open

| Type | Example | App Selection |
|------|---------|---------------|
| URLs | `https://github.com` | Default browser or specific browser |
| Files | `~/Documents/notes.md` | Default app or specific app |
| Folders | `~/Projects/myapp` | Finder, Terminal, or IDE |
| Deep Links | `raycast://extensions/...` | N/A (system handles) |

### 1.2 App Selection

Users can specify which application opens the quicklink:
- **Default** - System default for the URL/file type
- **Specific Browser** - Safari, Chrome, Firefox, Arc, etc.
- **Specific App** - VS Code, Terminal, iTerm2, etc.

---

## 2. Dynamic Placeholders

### 2.1 Argument Placeholders

| Placeholder | Description | Example |
|------------|-------------|---------|
| `{argument}` | Single input prompt | `https://google.com/search?q={argument}` |
| `{argument name="..."}` | Named argument (reusable) | `{argument name="query"}` |
| `{argument default="..."}` | Optional with default | `{argument default="rust"}` |
| `{argument options="a,b,c"}` | Dropdown options | `{argument name="lang" options="en,es,fr"}` |

**Rules:**
- Maximum 3 different arguments per quicklink
- Named arguments with same name share the same value
- Arguments without default are required

### 2.2 System Placeholders

| Placeholder | Description | Auto Percent-Encode |
|------------|-------------|---------------------|
| `{clipboard}` | Current clipboard text | Yes |
| `{selection}` | Selected text from frontmost app | Yes |
| `{date}` | Current date (system format) | No |
| `{time}` | Current time (system format) | No |
| `{datetime}` | Date and time | No |
| `{day}` | Day of week (e.g., "Monday") | No |
| `{uuid}` | Random UUID | No |

### 2.3 Modifiers

Modifiers transform placeholder values using pipe syntax: `{placeholder | modifier}`

| Modifier | Description | Example |
|----------|-------------|---------|
| `uppercase` | `Foo` → `FOO` | `{clipboard \| uppercase}` |
| `lowercase` | `Foo` → `foo` | `{argument \| lowercase}` |
| `trim` | Remove leading/trailing whitespace | `{selection \| trim}` |
| `percent-encode` | URL encode special chars | `{argument \| percent-encode}` |
| `raw` | Disable auto percent-encoding | `{clipboard \| raw}` |

Multiple modifiers can be chained: `{clipboard | trim | uppercase}`

**Default Behavior:**
- All placeholders in quicklink URLs are automatically percent-encoded
- Use `raw` modifier to disable this

### 2.4 Date/Time Offsets

| Syntax | Description |
|--------|-------------|
| `{date offset="+2d"}` | 2 days from now |
| `{date offset="-1M"}` | 1 month ago |
| `{time offset="+3h"}` | 3 hours from now |
| `{datetime offset="+1y -2d"}` | 1 year forward, 2 days back |

**Units:** `m` (minutes), `h` (hours), `d` (days), `M` (months), `y` (years)

### 2.5 Custom Date Formats

```
{date format="yyyy-MM-dd"}           → 2026-01-21
{date format="EEEE, MMM d, yyyy"}    → Tuesday, Jan 21, 2026
{date format="MM/dd/yyyy"}           → 01/21/2026
{date format="HH:mm:ss"}             → 14:30:45
```

Can combine with offsets: `{date format="yyyy-MM-dd" offset="+3d"}`

---

## 3. User Interface

### 3.1 Creating Quick Links

**Command:** "Create Quicklink" accessible from root search

**Fields:**
1. **Name** - Display name (required)
2. **Link** - URL, file path, or folder path (required)
3. **Open With** - Application selector (optional, default: system default)
4. **Icon** - Custom icon or emoji (optional, auto-fetches favicon for URLs)
5. **Alias** - Short keyword for quick access (optional)

**Auto Fill Feature:**
- When creating, can auto-fill from:
  - Active browser tab (URL + title)
  - Clipboard content
- Toggle in preferences

### 3.2 Using Quick Links

**From Root Search:**
1. Type quicklink name/alias
2. If has arguments → shows input fields
3. Press Enter → opens link

**Quick Search (Accessibility Required):**
1. Select text in any app
2. Press quicklink hotkey (configurable per-link)
3. Selected text passed as first argument

### 3.3 Managing Quick Links

**Actions (Cmd+K):**
- Edit quicklink
- Delete quicklink
- Duplicate quicklink
- Copy link
- Copy name
- Assign hotkey

### 3.4 Bundled Quick Links Library

Pre-configured quicklinks available to add:
- Google Search
- GitHub Search
- Stack Overflow Search
- YouTube Search
- Wikipedia Search
- Google Translate
- Google Maps
- Amazon Search
- Twitter/X Search

---

## 4. Data Model

### 4.1 QuickLink Structure

```rust
pub struct QuickLink {
    pub id: QuickLinkId,
    pub name: String,
    pub link: String,                    // URL/path with placeholders
    pub open_with: Option<String>,       // Bundle ID or app name
    pub icon: QuickLinkIcon,
    pub alias: Option<String>,           // Short keyword
    pub hotkey: Option<Hotkey>,          // Per-link hotkey
    pub created_at: DateTime<Utc>,
    pub accessed_at: Option<DateTime<Utc>>,
    pub access_count: u64,
}

pub enum QuickLinkIcon {
    Favicon(PathBuf),      // Cached favicon
    Emoji(char),           // Single emoji
    SystemIcon(String),    // SF Symbol name
    CustomImage(PathBuf),  // User-provided image
    Default,               // Globe icon
}
```

### 4.2 Parsed Placeholder

```rust
pub struct ParsedPlaceholder {
    pub kind: PlaceholderKind,
    pub name: Option<String>,
    pub default: Option<String>,
    pub options: Vec<String>,
    pub modifiers: Vec<Modifier>,
    pub format: Option<String>,      // For date
    pub offset: Option<DateOffset>,  // For date/time
}

pub enum PlaceholderKind {
    Argument,
    Clipboard,
    Selection,
    Date,
    Time,
    DateTime,
    Day,
    Uuid,
}

pub enum Modifier {
    Uppercase,
    Lowercase,
    Trim,
    PercentEncode,
    Raw,
}
```

---

## 5. Storage

### 5.1 SQLite Schema

```sql
CREATE TABLE quick_links (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    link TEXT NOT NULL,
    open_with TEXT,
    icon_type TEXT NOT NULL,  -- 'favicon', 'emoji', 'system', 'custom', 'default'
    icon_value TEXT,          -- path, emoji char, or symbol name
    alias TEXT,
    hotkey TEXT,              -- JSON: {"modifiers": ["cmd"], "key": "g"}
    created_at TEXT NOT NULL,
    accessed_at TEXT,
    access_count INTEGER DEFAULT 0
);

CREATE INDEX idx_quicklinks_alias ON quick_links(alias);
CREATE INDEX idx_quicklinks_name ON quick_links(name);
CREATE VIRTUAL TABLE quicklinks_fts USING fts5(name, link, alias);
```

### 5.2 TOML Export Format

```toml
# ~/.config/photoncast/quicklinks.toml

[[links]]
name = "Google Search"
link = "https://google.com/search?q={argument}"
alias = "g"
icon = "🔍"

[[links]]
name = "GitHub Search"
link = "https://github.com/search?q={argument name=\"query\"}&type={argument name=\"type\" options=\"repositories,code,issues\" default=\"repositories\"}"
open_with = "com.apple.Safari"
alias = "gh"
```

---

## 6. Implementation Tasks

### Phase 1: Placeholder System (Core)
- [ ] Implement placeholder parser with full syntax support
- [ ] Support all placeholder types (argument, clipboard, selection, date, etc.)
- [ ] Implement modifier system (uppercase, lowercase, trim, percent-encode, raw)
- [ ] Implement date/time offsets and custom formats
- [ ] Add percent-encoding by default for URL placeholders

### Phase 2: UI Enhancements
- [ ] Add "Create Quicklink" command to root search
- [ ] Implement argument input UI (text fields, dropdowns for options)
- [ ] Add app selector (Open With) to create/edit UI
- [ ] Add icon picker (emoji, system icons, custom)
- [ ] Add alias field
- [ ] Implement Auto Fill from browser/clipboard

### Phase 3: Quick Search
- [ ] Add per-quicklink hotkey assignment
- [ ] Implement Quick Search (select text → trigger quicklink)
- [ ] Request Accessibility permission when needed

### Phase 4: Library & Polish
- [ ] Create bundled quicklinks library
- [ ] Add "Find in Library" browser in preferences
- [ ] Implement import from JSON
- [ ] Add duplicate action
- [ ] Update preferences UI for quicklinks management

---

## 7. Migration

Existing quicklinks with `{query}` placeholder will be auto-migrated to `{argument}`.

---

## 8. Dependencies

- `chrono` - Date/time handling
- `percent-encoding` - URL encoding
- `regex` - Placeholder parsing
- Existing: `rusqlite`, `serde`, `toml`

---

## 9. Test Cases

```rust
#[test]
fn test_argument_placeholder() {
    let link = "https://google.com/search?q={argument}";
    let result = substitute(link, &[("argument", "rust lang")]);
    assert_eq!(result, "https://google.com/search?q=rust%20lang");
}

#[test]
fn test_named_arguments() {
    let link = "https://translate.google.com/?sl={argument name=\"from\"}&tl={argument name=\"to\"}&text={argument name=\"from\"}";
    let result = substitute(link, &[("from", "en"), ("to", "es")]);
    assert_eq!(result, "https://translate.google.com/?sl=en&tl=es&text=en");
}

#[test]
fn test_modifiers() {
    let link = "https://example.com/{argument | uppercase | trim}";
    let result = substitute(link, &[("argument", "  hello world  ")]);
    assert_eq!(result, "https://example.com/HELLO%20WORLD");
}

#[test]
fn test_raw_modifier() {
    let link = "https://example.com/{argument | raw}";
    let result = substitute(link, &[("argument", "hello world")]);
    assert_eq!(result, "https://example.com/hello world"); // Not encoded
}

#[test]
fn test_date_offset() {
    let link = "https://cal.com/date={date offset=\"+7d\" format=\"yyyy-MM-dd\"}";
    // Result depends on current date
}

#[test]
fn test_options_dropdown() {
    let placeholders = parse_placeholders("https://github.com/search?type={argument options=\"repos,code,issues\"}");
    assert_eq!(placeholders[0].options, vec!["repos", "code", "issues"]);
}
```
