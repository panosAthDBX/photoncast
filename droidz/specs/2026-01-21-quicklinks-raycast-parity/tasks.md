# Quick Links Raycast Parity - Implementation Tasks

## Phase 1: Placeholder System (Core) ✅

### 1.1 Placeholder Parser
- [x] Create `placeholder.rs` module in `photoncast-quicklinks`
- [x] Define `Placeholder` enum with all types (Argument, Clipboard, Selection, Date, Time, DateTime, Day, Uuid)
- [x] Define `Modifier` enum (Uppercase, Lowercase, Trim, PercentEncode, Raw)
- [x] Implement regex-based parser for placeholder syntax
- [x] Support named arguments: `{argument name="..."}`
- [x] Support default values: `{argument default="..."}`
- [x] Support options: `{argument options="a,b,c"}`
- [x] Support modifiers with pipe syntax: `{placeholder | modifier | modifier}`
- [x] Support date offsets: `{date offset="+2d"}`
- [x] Support custom date formats: `{date format="yyyy-MM-dd"}`

### 1.2 Placeholder Substitution
- [x] Implement `substitute_placeholders()` function
- [x] Apply modifiers in order (trim before uppercase, etc.)
- [x] Auto percent-encode all substitutions by default
- [x] Handle `raw` modifier to skip encoding
- [x] Handle clipboard access (read current clipboard)
- [x] Handle selection access (get selected text - requires Accessibility)
- [x] Handle date/time with offsets and custom formats
- [x] Generate UUIDs

### 1.3 Placeholder Extraction
- [x] Implement `extract_placeholders()` to get list of required inputs
- [x] Return metadata: name, type, default, options
- [x] Deduplicate named arguments (same name = same input)

### 1.4 Migration
- [x] Auto-migrate `{query}` to `{argument}` for backward compatibility

## Phase 2: Data Model Updates ✅

### 2.1 QuickLink Model
- [x] Add `open_with: Option<String>` field (bundle ID)
- [x] Replace `icon_path` with `QuickLinkIcon` enum
- [x] Add `alias: Option<String>` field
- [x] Add `hotkey: Option<Hotkey>` field
- [x] Update SQLite schema
- [x] Update TOML serialization

### 2.2 QuickLinkIcon
- [x] Create enum: Favicon, Emoji, SystemIcon, CustomImage, Default
- [x] Implement icon resolution for display
- [x] Implement icon picker logic

## Phase 3: UI - Create Quicklink Command ✅

### 3.1 Create Quicklink View
- [x] Create new view accessible from root search
- [x] Add name input field
- [x] Add link input field with placeholder syntax highlighting
- [x] Add "Open With" app selector dropdown
- [x] Add icon picker (emoji grid, system icons, custom upload)
- [x] Add alias input field
- [x] Add "Auto Fill" toggle

### 3.2 Auto Fill Feature
- [x] Detect active browser tab URL and title
- [x] Detect clipboard URL content
- [x] Pre-fill name and link fields

### 3.3 Placeholder Preview
- [x] Show real-time preview of final URL
- [x] Highlight placeholders in link input
- [x] Validate placeholder syntax

## Phase 4: UI - Argument Input ✅

### 4.1 Argument Input View
- [x] Create view for entering argument values
- [x] Show text field for each unique argument
- [x] Show dropdown for arguments with options
- [x] Pre-fill default values
- [x] Show placeholder name as label

### 4.2 Quick Search Integration
- [x] If quicklink triggered with selected text, pre-fill first argument
- [x] Support per-quicklink hotkey configuration

## Phase 5: UI - Management ✅

### 5.1 Quicklink Actions
- [x] Add "Edit Quicklink" action
- [x] Add "Delete Quicklink" action with confirmation
- [x] Add "Duplicate Quicklink" action
- [x] Add "Copy Link" action
- [x] Add "Copy Name" action
- [x] Add "Assign Hotkey" action

### 5.2 Edit Quicklink View
- [x] Reuse Create Quicklink view with pre-filled values
- [x] Show "Save" vs "Create" button appropriately

### 5.3 Preferences Integration
- [x] Add Quicklinks section in preferences
- [x] List all quicklinks with edit/delete
- [x] Add "Create New" button
- [x] Add "Import from JSON" button
- [x] Add "Find in Library" button

## Phase 6: Bundled Library ✅

### 6.1 Create Library
- [x] Define bundled quicklinks as static data
- [x] Include: Google Search, GitHub Search, Stack Overflow, YouTube, Wikipedia, Google Translate, Google Maps, Amazon, Twitter/X

### 6.2 Library Browser
- [x] Create view to browse bundled quicklinks
- [x] Allow adding to personal quicklinks
- [x] Show preview before adding

## Phase 7: Search Provider Updates ✅

### 7.1 Update QuickLinks Provider
- [x] Show argument indicators in search results
- [x] Show icon from QuickLinkIcon enum
- [x] Filter by alias as well as name

### 7.2 Result Actions
- [x] Primary action: Open (or show argument input if needed)
- [x] Secondary actions: Edit, Delete, Copy Link

## Phase 8: Testing & Polish ✅

### 8.1 Unit Tests
- [x] Test placeholder parser with all syntax variations
- [x] Test modifier application
- [x] Test percent-encoding behavior
- [x] Test date/time offset calculations
- [x] Test custom date format parsing

### 8.2 Integration Tests
- [x] Test full flow: create → search → open
- [x] Test argument input → substitution → open
- [x] Test TOML export/import

### 8.3 Edge Cases
- [x] Handle malformed placeholders gracefully
- [x] Handle missing argument values
- [x] Handle clipboard access failure
- [x] Handle selection access without permission
