//! Integration tests for the File Search feature.
//!
//! These tests verify the complete file search workflow including:
//! - End-to-end search through the file index
//! - Browsing mode navigation
//! - File actions (rename, duplicate, get info, etc.)

use std::path::{Path, PathBuf};
use std::time::Duration;

use photoncast_core::platform::file_browser::{DirectoryEntry, FileBrowser};
use photoncast_core::search::file_index::{FileIndex, FileTokenizer, IndexedFile, IndexingService};
use photoncast_core::search::file_query::{FileCategory, FileQuery, FileTypeFilter};
use tempfile::{tempdir, TempDir};

// =============================================================================
// Test Helpers
// =============================================================================

/// Creates a test directory structure for file search tests.
///
/// Structure:
/// ```text
/// temp_dir/
/// ├── documents/
/// │   ├── report.pdf
/// │   ├── notes.txt
/// │   └── spreadsheet.xlsx
/// ├── images/
/// │   ├── photo.jpg
/// │   └── screenshot.png
/// ├── code/
/// │   ├── main.rs
/// │   ├── lib.rs
/// │   └── utils/
/// │       └── helper.rs
/// ├── MyAwesomeProject/
/// │   └── README.md
/// ├── special_chars/
/// │   └── file with spaces.txt
/// └── empty_folder/
/// ```
fn create_test_file_structure(dir: &Path) {
    // Documents folder
    let docs = dir.join("documents");
    std::fs::create_dir_all(&docs).unwrap();
    std::fs::write(docs.join("report.pdf"), b"PDF content").unwrap();
    std::fs::write(docs.join("notes.txt"), b"Some notes here").unwrap();
    std::fs::write(docs.join("spreadsheet.xlsx"), b"Excel data").unwrap();

    // Images folder
    let images = dir.join("images");
    std::fs::create_dir_all(&images).unwrap();
    std::fs::write(images.join("photo.jpg"), b"JPEG content").unwrap();
    std::fs::write(images.join("screenshot.png"), b"PNG content").unwrap();

    // Code folder with nested structure
    let code = dir.join("code");
    let utils = code.join("utils");
    std::fs::create_dir_all(&utils).unwrap();
    std::fs::write(code.join("main.rs"), b"fn main() {}").unwrap();
    std::fs::write(code.join("lib.rs"), b"pub mod utils;").unwrap();
    std::fs::write(utils.join("helper.rs"), b"pub fn help() {}").unwrap();

    // CamelCase folder
    let project = dir.join("MyAwesomeProject");
    std::fs::create_dir_all(&project).unwrap();
    std::fs::write(project.join("README.md"), b"# My Awesome Project").unwrap();

    // Special characters folder
    let special = dir.join("special_chars");
    std::fs::create_dir_all(&special).unwrap();
    std::fs::write(special.join("file with spaces.txt"), b"content").unwrap();

    // Empty folder
    std::fs::create_dir_all(dir.join("empty_folder")).unwrap();
}

/// Creates a temp directory with the test file structure.
fn setup_test_dir() -> TempDir {
    let temp = tempdir().expect("should create temp dir");
    create_test_file_structure(temp.path());
    temp
}

// =============================================================================
// Search Flow Integration Tests
// =============================================================================

#[test]
fn test_index_directory_and_search_by_term() {
    let temp = setup_test_dir();

    // Create and populate index
    let mut index = FileIndex::open_in_memory().unwrap();
    index_directory_recursive(&mut index, temp.path());

    // Search for "report"
    let tokens = FileTokenizer::tokenize("report");
    let results = index.search(&tokens, 10).unwrap();

    assert!(!results.is_empty(), "Should find at least one result");
    assert!(
        results.iter().any(|r| r.name == "report.pdf"),
        "Should find report.pdf"
    );
}

#[test]
fn test_index_and_search_camel_case() {
    let temp = setup_test_dir();

    let mut index = FileIndex::open_in_memory().unwrap();
    index_directory_recursive(&mut index, temp.path());

    // Search for "awesome" (from MyAwesomeProject)
    let tokens = FileTokenizer::tokenize("awesome");
    let results = index.search(&tokens, 10).unwrap();

    assert!(!results.is_empty(), "Should find camelCase match");
    assert!(
        results.iter().any(|r| r.name == "MyAwesomeProject"),
        "Should find MyAwesomeProject folder"
    );
}

#[test]
fn test_search_with_multiple_tokens() {
    let temp = setup_test_dir();

    let mut index = FileIndex::open_in_memory().unwrap();
    index_directory_recursive(&mut index, temp.path());

    // Search for "my awesome" (should match MyAwesomeProject)
    let tokens = FileTokenizer::tokenize("my awesome");
    let results = index.search(&tokens, 10).unwrap();

    assert!(
        !results.is_empty(),
        "Should find result with multiple tokens"
    );
    assert!(
        results.iter().any(|r| r.name == "MyAwesomeProject"),
        "Should find MyAwesomeProject folder"
    );
}

#[test]
fn test_search_prefix_matching() {
    let temp = setup_test_dir();

    let mut index = FileIndex::open_in_memory().unwrap();
    index_directory_recursive(&mut index, temp.path());

    // Prefix search for "rep" should find "report.pdf"
    let results = index.search_prefix("rep", 10).unwrap();

    assert!(!results.is_empty(), "Prefix search should find results");
    assert!(
        results.iter().any(|r| r.name == "report.pdf"),
        "Should find report.pdf with prefix 'rep'"
    );
}

#[test]
fn test_search_with_file_type_filter() {
    let temp = setup_test_dir();

    let mut index = FileIndex::open_in_memory().unwrap();
    index_directory_recursive(&mut index, temp.path());

    // Search for files, then filter by extension
    let results = index.search(&["txt".to_string()], 20).unwrap();

    // Filter results to only .txt files
    let txt_results: Vec<_> = results
        .iter()
        .filter(|r| r.extension.as_deref() == Some("txt"))
        .collect();

    assert!(
        !txt_results.is_empty(),
        "Should find txt files when searching for 'txt'"
    );
}

#[test]
fn test_query_parse_and_filter_by_extension() {
    let temp = setup_test_dir();

    // Parse query with file type filter
    let query = FileQuery::parse(".txt notes");

    assert_eq!(
        query.file_type,
        Some(FileTypeFilter::Extension("txt".to_string()))
    );
    assert_eq!(query.terms, vec!["notes"]);

    // Test matching against files
    let notes_txt = temp.path().join("documents/notes.txt");
    assert!(query.matches_file(&notes_txt, "notes.txt"));

    let report_pdf = temp.path().join("documents/report.pdf");
    assert!(!query.matches_file(&report_pdf, "report.pdf"));
}

#[test]
fn test_query_parse_location_filter() {
    let temp = setup_test_dir();

    // Create a query with location
    let docs_path = temp.path().join("documents");
    let query_str = format!("report in {}", docs_path.display());
    let mut query = FileQuery::parse(&query_str);

    // Manually set location since parse won't recognize temp paths
    query.location = Some(docs_path.clone());
    query.terms = vec!["report".to_string()];

    // Test matching
    let report_in_docs = docs_path.join("report.pdf");
    assert!(query.matches_file(&report_in_docs, "report.pdf"));

    let report_elsewhere = temp.path().join("images/report.pdf");
    std::fs::write(&report_elsewhere, b"").unwrap();
    assert!(!query.matches_file(&report_elsewhere, "report.pdf"));
}

#[test]
fn test_query_parse_parent_folder_filter() {
    let temp = setup_test_dir();

    // Parse query with parent folder pattern
    let query = FileQuery::parse("code/helper");

    assert_eq!(query.parent_folder, Some("code".to_string()));
    assert_eq!(query.terms, vec!["helper"]);

    // Test matching
    let helper_in_code = temp.path().join("code/utils/helper.rs");
    assert!(query.matches_file(&helper_in_code, "helper.rs"));

    let helper_elsewhere = temp.path().join("documents/helper.txt");
    std::fs::write(&helper_elsewhere, b"").unwrap();
    assert!(!query.matches_file(&helper_elsewhere, "helper.txt"));
}

#[test]
fn test_query_folder_prioritization() {
    // Parse query with trailing slash
    let query = FileQuery::parse("documents/");

    assert!(query.prioritize_folders);
    assert!(query.terms.contains(&"documents".to_string()));

    // Should only match directories
    let folder_path = Path::new("/test/documents");
    // Note: We can't test is_dir without real filesystem
}

#[test]
fn test_query_exact_phrase_match() {
    let temp = setup_test_dir();

    // Parse query with quoted phrase
    let query = FileQuery::parse("\"file with spaces\"");

    assert_eq!(query.exact_phrase, Some("file with spaces".to_string()));

    // Test matching
    let file_with_spaces = temp.path().join("special_chars/file with spaces.txt");
    assert!(query.matches_file(&file_with_spaces, "file with spaces.txt"));

    let no_spaces = temp.path().join("documents/notes.txt");
    assert!(!query.matches_file(&no_spaces, "notes.txt"));
}

#[test]
fn test_file_category_extension_mapping() {
    // Test Documents category
    assert!(FileCategory::Documents.extensions().contains(&"pdf"));
    assert!(FileCategory::Documents.extensions().contains(&"txt"));
    assert!(FileCategory::Documents.extensions().contains(&"docx"));

    // Test Images category
    assert!(FileCategory::Images.extensions().contains(&"jpg"));
    assert!(FileCategory::Images.extensions().contains(&"png"));

    // Test Code category
    assert!(FileCategory::Code.extensions().contains(&"rs"));
    assert!(FileCategory::Code.extensions().contains(&"js"));
}

#[test]
fn test_file_type_filter_matches() {
    let pdf_filter = FileTypeFilter::Extension("pdf".to_string());
    assert!(pdf_filter.matches_extension("pdf"));
    assert!(pdf_filter.matches_extension("PDF"));
    assert!(!pdf_filter.matches_extension("txt"));

    let images_filter = FileTypeFilter::Category(FileCategory::Images);
    assert!(images_filter.matches_extension("jpg"));
    assert!(images_filter.matches_extension("png"));
    assert!(!images_filter.matches_extension("pdf"));
}

#[test]
fn test_indexing_service_basic() {
    let temp = setup_test_dir();

    let index = FileIndex::open_in_memory().unwrap();
    let service = IndexingService::new(index, vec![temp.path().to_path_buf()]);

    // Run indexing synchronously
    service.start_indexing_sync().unwrap();

    assert!(service.is_complete());
    assert!(service.files_indexed() > 0);

    // Verify we can search the indexed files
    let index = service.index().read();
    let results = index.search(&["report".to_string()], 10).unwrap();
    assert!(!results.is_empty());
}

#[tokio::test]
async fn test_indexing_service_async() {
    let temp = setup_test_dir();

    let index = FileIndex::open_in_memory().unwrap();
    let service = IndexingService::new(index, vec![temp.path().to_path_buf()]);

    // Run indexing asynchronously
    service.start_indexing().await.unwrap();

    assert!(service.is_complete());

    let index = service.index().read();
    let file_count = index.file_count().unwrap();
    assert!(file_count > 0, "Should have indexed files");
}

#[test]
fn test_indexing_service_skips_hidden_files() {
    let temp = tempdir().unwrap();

    // Create a hidden file
    std::fs::write(temp.path().join(".hidden_file"), b"hidden").unwrap();
    std::fs::write(temp.path().join("visible.txt"), b"visible").unwrap();

    let index = FileIndex::open_in_memory().unwrap();
    let service = IndexingService::new(index, vec![temp.path().to_path_buf()]);
    service.start_indexing_sync().unwrap();

    let index = service.index().read();
    let hidden_results = index.search(&["hidden".to_string()], 10).unwrap();
    let visible_results = index.search(&["visible".to_string()], 10).unwrap();

    assert!(hidden_results.is_empty(), "Should not index hidden files");
    assert!(!visible_results.is_empty(), "Should index visible files");
}

#[test]
fn test_search_empty_directory() {
    let temp = tempdir().unwrap();
    std::fs::create_dir_all(temp.path().join("empty")).unwrap();

    let index = FileIndex::open_in_memory().unwrap();
    let service = IndexingService::new(index, vec![temp.path().to_path_buf()]);
    service.start_indexing_sync().unwrap();

    // Search should handle empty directories gracefully
    let index = service.index().read();
    let results = index.search(&["nonexistent".to_string()], 10).unwrap();
    assert!(results.is_empty());
}

// =============================================================================
// Browsing Mode Integration Tests
// =============================================================================

#[test]
fn test_browsing_mode_detection_root() {
    assert!(FileBrowser::is_browsing_mode("/"));
    assert!(FileBrowser::is_browsing_mode("/Users"));
    assert!(FileBrowser::is_browsing_mode("/Applications/Safari.app"));
}

#[test]
fn test_browsing_mode_detection_home() {
    assert!(FileBrowser::is_browsing_mode("~"));
    assert!(FileBrowser::is_browsing_mode("~/"));
    assert!(FileBrowser::is_browsing_mode("~/Documents"));
}

#[test]
fn test_browsing_mode_detection_env_vars() {
    assert!(FileBrowser::is_browsing_mode("$HOME"));
    assert!(FileBrowser::is_browsing_mode("$HOME/Documents"));
    assert!(FileBrowser::is_browsing_mode("${HOME}/Documents"));
}

#[test]
fn test_browsing_mode_detection_regular_query() {
    assert!(!FileBrowser::is_browsing_mode("firefox"));
    assert!(!FileBrowser::is_browsing_mode("my document"));
    assert!(!FileBrowser::is_browsing_mode(""));
    assert!(!FileBrowser::is_browsing_mode("   "));
}

#[test]
fn test_path_parsing_root() {
    let path = FileBrowser::parse_path("/");
    assert_eq!(path, Some(PathBuf::from("/")));

    let path = FileBrowser::parse_path("/Users/test");
    assert_eq!(path, Some(PathBuf::from("/Users/test")));
}

#[test]
fn test_path_parsing_home() {
    let path = FileBrowser::parse_path("~");
    assert!(path.is_some());

    if let Some(home) = dirs::home_dir() {
        assert_eq!(path.unwrap(), home);
    }
}

#[test]
fn test_path_parsing_home_subdir() {
    let path = FileBrowser::parse_path("~/Documents");
    assert!(path.is_some());

    if let Some(home) = dirs::home_dir() {
        assert_eq!(path.unwrap(), home.join("Documents"));
    }
}

#[test]
fn test_env_var_expansion() {
    std::env::set_var("TEST_PATH_VAR", "/test/path");

    let expanded = FileBrowser::expand_env_vars("$TEST_PATH_VAR/subdir");
    assert_eq!(expanded, "/test/path/subdir");

    let expanded = FileBrowser::expand_env_vars("${TEST_PATH_VAR}/subdir");
    assert_eq!(expanded, "/test/path/subdir");

    std::env::remove_var("TEST_PATH_VAR");
}

#[test]
fn test_env_var_expansion_undefined() {
    let result = FileBrowser::expand_env_vars("$UNDEFINED_TEST_VAR_XYZ/path");
    assert_eq!(result, "$UNDEFINED_TEST_VAR_XYZ/path");
}

#[test]
fn test_list_directory_contents() {
    let temp = setup_test_dir();

    let entries = FileBrowser::list_directory(temp.path()).unwrap();

    assert!(!entries.is_empty());

    // Check that folders come before files (sorting)
    let folder_names: Vec<_> = entries
        .iter()
        .filter(|e| e.is_directory())
        .map(|e| e.name.as_str())
        .collect();

    assert!(folder_names.contains(&"documents"));
    assert!(folder_names.contains(&"images"));
    assert!(folder_names.contains(&"code"));
}

#[test]
fn test_list_directory_sorting() {
    let temp = tempdir().unwrap();

    // Create files and folders
    std::fs::create_dir(temp.path().join("alpha_folder")).unwrap();
    std::fs::create_dir(temp.path().join("beta_folder")).unwrap();
    std::fs::write(temp.path().join("alpha_file.txt"), b"").unwrap();
    std::fs::write(temp.path().join("beta_file.txt"), b"").unwrap();

    let entries = FileBrowser::list_directory(temp.path()).unwrap();
    let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();

    // Folders should come first
    let first_file_idx = names
        .iter()
        .position(|&n| n.contains("file"))
        .unwrap_or(names.len());

    for (i, entry) in entries.iter().enumerate() {
        if i < first_file_idx {
            assert!(entry.is_directory(), "Expected folder at position {}", i);
        } else {
            assert!(!entry.is_directory(), "Expected file at position {}", i);
        }
    }
}

#[test]
fn test_list_directory_nonexistent() {
    let result = FileBrowser::list_directory(Path::new("/nonexistent/path/12345"));
    assert!(result.is_err());
}

#[test]
fn test_file_browser_navigation() {
    let temp = setup_test_dir();

    let mut browser = FileBrowser::with_path(temp.path().to_path_buf());
    assert_eq!(browser.current_path(), temp.path());

    // Navigate to documents
    let docs_path = temp.path().join("documents");
    browser.navigate_to(docs_path.clone());
    assert_eq!(browser.current_path(), docs_path.as_path());

    // Go up
    assert!(browser.go_up());
    assert_eq!(browser.current_path(), temp.path());
}

#[test]
fn test_file_browser_go_up_at_root() {
    let mut browser = FileBrowser::with_path(PathBuf::from("/"));
    assert!(!browser.go_up(), "Should return false at root");
}

#[test]
fn test_file_browser_enter_folder() {
    let temp = setup_test_dir();

    let mut browser = FileBrowser::with_path(temp.path().to_path_buf());
    let entries = FileBrowser::list_directory(temp.path()).unwrap();

    // Find the documents folder
    let docs_entry = entries.iter().find(|e| e.name == "documents").unwrap();

    assert!(browser.enter_folder(docs_entry));
    assert!(browser.current_path().ends_with("documents"));
}

#[test]
fn test_file_browser_enter_file_fails() {
    let temp = setup_test_dir();

    let mut browser = FileBrowser::with_path(temp.path().to_path_buf());

    // Create a fake file entry
    let file_entry = DirectoryEntry {
        path: temp.path().join("documents/notes.txt"),
        name: "notes.txt".to_string(),
        kind: photoncast_core::platform::spotlight::FileKind::File,
        size: Some(100),
        modified: None,
        item_count: None,
    };

    assert!(
        !browser.enter_folder(&file_entry),
        "Should not enter a file"
    );
}

#[test]
fn test_filter_entries_by_name() {
    let temp = setup_test_dir();

    let entries = FileBrowser::list_directory(&temp.path().join("documents")).unwrap();
    let filtered = FileBrowser::filter_entries(&entries, "rep");

    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].name, "report.pdf");
}

#[test]
fn test_filter_entries_case_insensitive() {
    let temp = setup_test_dir();

    let entries = FileBrowser::list_directory(&temp.path().join("documents")).unwrap();

    let filtered_lower = FileBrowser::filter_entries(&entries, "report");
    let filtered_upper = FileBrowser::filter_entries(&entries, "REPORT");

    assert_eq!(filtered_lower.len(), 1);
    assert_eq!(filtered_upper.len(), 1);
}

#[test]
fn test_filter_entries_empty_filter() {
    let temp = setup_test_dir();

    let entries = FileBrowser::list_directory(&temp.path().join("documents")).unwrap();
    let filtered = FileBrowser::filter_entries(&entries, "");

    assert_eq!(filtered.len(), entries.len());
}

#[test]
fn test_extract_filter_existing_directory() {
    let temp = setup_test_dir();
    let docs_path = temp.path().join("documents");

    let result = FileBrowser::extract_filter(&docs_path.to_string_lossy());
    assert!(result.is_some());

    let (path, filter) = result.unwrap();
    assert_eq!(path, docs_path);
    assert!(filter.is_empty());
}

#[test]
fn test_extract_filter_with_partial_name() {
    let temp = setup_test_dir();
    let partial_path = temp.path().join("documents/nonexistent");

    let result = FileBrowser::extract_filter(&partial_path.to_string_lossy());

    if let Some((path, filter)) = result {
        assert_eq!(path, temp.path().join("documents"));
        assert_eq!(filter, "nonexistent");
    }
}

#[cfg(unix)]
#[test]
fn test_symlink_handling() {
    let temp = tempdir().unwrap();

    // Create target file
    let target = temp.path().join("target.txt");
    std::fs::write(&target, b"content").unwrap();

    // Create symlink
    let link = temp.path().join("link.txt");
    std::os::unix::fs::symlink(&target, &link).unwrap();

    let entries = FileBrowser::list_directory(temp.path()).unwrap();
    let link_entry = entries.iter().find(|e| e.name == "link.txt").unwrap();

    assert!(link_entry.is_symlink());

    // Resolve symlink
    let resolved = FileBrowser::resolve_symlink(link_entry);
    assert!(resolved.to_string_lossy().contains("target.txt"));
}

// =============================================================================
// File Action Integration Tests (macOS only)
// =============================================================================

#[cfg(target_os = "macos")]
mod file_action_tests {
    use super::*;
    use photoncast_core::platform::file_actions::{
        compress, delete_permanently, duplicate_file, get_file_info, move_file, rename_file,
        validate_filename, FileActionError,
    };

    #[test]
    fn test_get_file_info_basic() {
        let temp = tempdir().unwrap();
        let file_path = temp.path().join("test.txt");
        std::fs::write(&file_path, "test content").unwrap();

        let info = get_file_info(&file_path).unwrap();

        assert_eq!(info.name, "test.txt");
        assert_eq!(info.size, 12); // "test content" is 12 bytes
        assert!(!info.is_directory);
        assert!(info.is_readable);
    }

    #[test]
    fn test_get_file_info_directory() {
        let temp = tempdir().unwrap();
        let dir_path = temp.path().join("subdir");
        std::fs::create_dir(&dir_path).unwrap();

        // Create files in directory
        std::fs::write(dir_path.join("file1.txt"), b"").unwrap();
        std::fs::write(dir_path.join("file2.txt"), b"").unwrap();

        let info = get_file_info(&dir_path).unwrap();

        assert!(info.is_directory);
        assert_eq!(info.item_count, Some(2));
    }

    #[test]
    fn test_get_file_info_not_found() {
        let result = get_file_info(Path::new("/nonexistent/file.txt"));
        assert!(matches!(
            result.unwrap_err(),
            FileActionError::NotFound { .. }
        ));
    }

    #[test]
    fn test_validate_filename_valid() {
        assert!(validate_filename("document.pdf").is_ok());
        assert!(validate_filename("my file.txt").is_ok());
        assert!(validate_filename("file-with-dashes").is_ok());
        assert!(validate_filename(".hidden").is_ok());
    }

    #[test]
    fn test_validate_filename_invalid() {
        // Slash
        assert!(validate_filename("path/to/file").is_err());

        // Colon
        assert!(validate_filename("file:name").is_err());

        // Empty
        assert!(validate_filename("").is_err());

        // Reserved
        assert!(validate_filename(".").is_err());
        assert!(validate_filename("..").is_err());
    }

    #[test]
    fn test_rename_file_success() {
        let temp = tempdir().unwrap();
        let original = temp.path().join("original.txt");
        std::fs::write(&original, b"content").unwrap();

        let new_path = rename_file(&original, "renamed.txt").unwrap();

        assert!(new_path.exists());
        assert!(!original.exists());
        assert!(new_path.ends_with("renamed.txt"));
    }

    #[test]
    fn test_rename_file_invalid_name() {
        let temp = tempdir().unwrap();
        let original = temp.path().join("original.txt");
        std::fs::write(&original, b"content").unwrap();

        let result = rename_file(&original, "bad/name");
        assert!(matches!(
            result.unwrap_err(),
            FileActionError::InvalidFilename { .. }
        ));
    }

    #[test]
    fn test_rename_file_already_exists() {
        let temp = tempdir().unwrap();
        let file1 = temp.path().join("file1.txt");
        let file2 = temp.path().join("file2.txt");
        std::fs::write(&file1, b"content1").unwrap();
        std::fs::write(&file2, b"content2").unwrap();

        let result = rename_file(&file1, "file2.txt");
        assert!(matches!(
            result.unwrap_err(),
            FileActionError::AlreadyExists { .. }
        ));
    }

    #[test]
    fn test_duplicate_file_success() {
        let temp = tempdir().unwrap();
        let original = temp.path().join("original.txt");
        std::fs::write(&original, b"content").unwrap();

        let copy_path = duplicate_file(&original).unwrap();

        assert!(copy_path.exists());
        assert!(original.exists()); // Original should still exist
        assert!(copy_path.to_string_lossy().contains("copy"));
        assert_eq!(std::fs::read_to_string(&copy_path).unwrap(), "content");
    }

    #[test]
    fn test_duplicate_file_multiple_copies() {
        let temp = tempdir().unwrap();
        let original = temp.path().join("original.txt");
        std::fs::write(&original, b"content").unwrap();

        let copy1 = duplicate_file(&original).unwrap();
        let copy2 = duplicate_file(&original).unwrap();

        assert!(copy1.exists());
        assert!(copy2.exists());
        assert_ne!(copy1, copy2);
    }

    #[test]
    fn test_move_file_success() {
        let temp = tempdir().unwrap();
        let subdir = temp.path().join("subdir");
        std::fs::create_dir(&subdir).unwrap();

        let file_path = temp.path().join("file.txt");
        std::fs::write(&file_path, b"content").unwrap();

        let new_path = move_file(&file_path, &subdir).unwrap();

        assert!(new_path.exists());
        assert!(!file_path.exists());
        assert_eq!(new_path.parent().unwrap(), subdir);
    }

    #[test]
    fn test_move_file_not_found() {
        let temp = tempdir().unwrap();
        let result = move_file(Path::new("/nonexistent/file.txt"), temp.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_delete_permanently_file() {
        let temp = tempdir().unwrap();
        let file_path = temp.path().join("to_delete.txt");
        std::fs::write(&file_path, b"content").unwrap();

        assert!(file_path.exists());
        delete_permanently(&file_path).unwrap();
        assert!(!file_path.exists());
    }

    #[test]
    fn test_delete_permanently_directory() {
        let temp = tempdir().unwrap();
        let dir_path = temp.path().join("to_delete");
        std::fs::create_dir(&dir_path).unwrap();
        std::fs::write(dir_path.join("file.txt"), b"content").unwrap();

        assert!(dir_path.exists());
        delete_permanently(&dir_path).unwrap();
        assert!(!dir_path.exists());
    }

    #[test]
    fn test_compress_file() {
        let temp = tempdir().unwrap();
        let file_path = temp.path().join("to_compress.txt");
        std::fs::write(&file_path, b"content to compress").unwrap();

        let archive_path = compress(&file_path).unwrap();

        assert!(archive_path.exists());
        assert!(archive_path.to_string_lossy().ends_with(".zip"));
    }

    #[test]
    fn test_compress_directory() {
        let temp = tempdir().unwrap();
        let dir_path = temp.path().join("folder");
        std::fs::create_dir(&dir_path).unwrap();
        std::fs::write(dir_path.join("file1.txt"), b"content1").unwrap();
        std::fs::write(dir_path.join("file2.txt"), b"content2").unwrap();

        let archive_path = compress(&dir_path).unwrap();

        assert!(archive_path.exists());
        assert!(archive_path.to_string_lossy().ends_with(".zip"));
    }

    #[test]
    fn test_error_user_messages() {
        let err = FileActionError::not_found("/path/to/file.txt");
        assert!(err.user_message().contains("doesn't exist"));

        let err = FileActionError::permission_denied("/path/to/file.txt");
        assert!(err.user_message().contains("permission"));

        let err = FileActionError::already_exists("/path/to/file.txt");
        assert!(err.user_message().contains("already exists"));
    }

    #[test]
    fn test_error_is_recoverable() {
        assert!(!FileActionError::not_found("x").is_recoverable());
        assert!(FileActionError::permission_denied("x").is_recoverable());
        assert!(FileActionError::already_exists("x").is_recoverable());
    }
}

// =============================================================================
// Edge Case Tests
// =============================================================================

#[test]
fn test_tokenizer_unicode_normalization() {
    // Test ASCII folding
    let tokens = FileTokenizer::tokenize("résumé");
    assert!(tokens.contains(&"resume".to_string()));

    let tokens = FileTokenizer::tokenize("naïve_café");
    assert!(tokens.contains(&"naive".to_string()));
    assert!(tokens.contains(&"cafe".to_string()));
}

#[test]
fn test_tokenizer_empty_input() {
    let tokens = FileTokenizer::tokenize("");
    assert!(tokens.is_empty());
}

#[test]
fn test_tokenizer_path_segments() {
    let tokens = FileTokenizer::tokenize("/Users/john/Documents/file.txt");

    assert!(tokens.contains(&"users".to_string()));
    assert!(tokens.contains(&"john".to_string()));
    assert!(tokens.contains(&"documents".to_string()));
    assert!(tokens.contains(&"file".to_string()));
    assert!(tokens.contains(&"txt".to_string()));
}

#[test]
fn test_index_handles_special_characters() {
    let temp = tempdir().unwrap();

    // Create file with special characters
    let special_file = temp.path().join("file with spaces & symbols.txt");
    std::fs::write(&special_file, b"content").unwrap();

    let mut index = FileIndex::open_in_memory().unwrap();
    index.add_file(&special_file).unwrap();

    // Should be able to search for it
    let results = index.search(&["spaces".to_string()], 10).unwrap();
    assert!(!results.is_empty());
}

#[test]
fn test_query_combined_filters() {
    // Test combining multiple query features
    let query = FileQuery::parse(".pdf \"annual report\" budget");

    assert_eq!(
        query.file_type,
        Some(FileTypeFilter::Extension("pdf".to_string()))
    );
    assert_eq!(query.exact_phrase, Some("annual report".to_string()));
    assert!(query.terms.contains(&"budget".to_string()));
}

#[test]
fn test_spotlight_predicate_generation() {
    let query = FileQuery::parse(".pdf report");

    let predicate = query.to_spotlight_predicate();
    assert!(predicate.contains("kMDItemFSName"));
    assert!(predicate.contains("*.pdf"));
    assert!(predicate.contains("report"));
}

#[test]
fn test_mdfind_name_query_generation() {
    let query = FileQuery::parse("report annual");

    let name_query = query.to_mdfind_name_query();
    assert!(name_query.contains("report"));
    assert!(name_query.contains("annual"));
}

// =============================================================================
// Concurrent Access Tests
// =============================================================================

#[test]
fn test_concurrent_index_reads() {
    use std::sync::Arc;
    use std::thread;

    let dir = tempdir().unwrap();
    create_test_file_structure(dir.path());

    let mut index = FileIndex::open_in_memory().unwrap();
    index_directory_recursive(&mut index, dir.path());

    let index = Arc::new(parking_lot::RwLock::new(index));

    // Spawn multiple readers concurrently
    let handles: Vec<_> = (0..10)
        .map(|_| {
            let index = Arc::clone(&index);
            thread::spawn(move || {
                for _ in 0..100 {
                    let guard = index.read();
                    let _ = guard.search(&["report".to_string()], 10);
                    let _ = guard.search(&["code".to_string()], 10);
                }
            })
        })
        .collect();

    // All threads should complete without deadlock or panic
    for handle in handles {
        handle.join().unwrap();
    }
}

#[test]
fn test_concurrent_read_write() {
    use std::sync::Arc;
    use std::thread;

    let dir = tempdir().unwrap();
    let index = Arc::new(parking_lot::RwLock::new(FileIndex::open_in_memory().unwrap()));

    // One writer thread adding files
    let index_writer = Arc::clone(&index);
    let dir_path = dir.path().to_path_buf();
    let writer = thread::spawn(move || {
        for i in 0..50 {
            let file_path = dir_path.join(format!("file{}.txt", i));
            std::fs::write(&file_path, format!("content {}", i)).unwrap();

            let mut guard = index_writer.write();
            let _ = guard.add_file(&file_path);
        }
    });

    // Multiple reader threads searching
    let reader_handles: Vec<_> = (0..5)
        .map(|_| {
            let index = Arc::clone(&index);
            thread::spawn(move || {
                for _ in 0..100 {
                    let guard = index.read();
                    let _ = guard.search(&["file".to_string()], 10);
                    thread::sleep(Duration::from_micros(100));
                }
            })
        })
        .collect();

    writer.join().unwrap();
    for handle in reader_handles {
        handle.join().unwrap();
    }

    // Verify final state
    let index = index.read();
    let results = index.search(&["file".to_string()], 100).unwrap();
    assert!(results.len() >= 40); // Most files should be indexed
}

#[test]
fn test_indexing_service_concurrent_search() {
    use std::sync::Arc;
    use std::thread;

    let dir = tempdir().unwrap();
    create_test_file_structure(dir.path());

    let index = FileIndex::open_in_memory().unwrap();
    let service = Arc::new(IndexingService::new(index, vec![dir.path().to_path_buf()]));

    // Start indexing in background
    let service_clone = Arc::clone(&service);
    let indexer = thread::spawn(move || {
        service_clone.start_indexing_sync().unwrap();
    });

    // Concurrent searches while indexing
    let search_handles: Vec<_> = (0..5)
        .map(|_| {
            let service = Arc::clone(&service);
            thread::spawn(move || {
                for _ in 0..50 {
                    let index = service.index().read();
                    let _ = index.search(&["report".to_string()], 10);
                    thread::sleep(Duration::from_millis(1));
                }
            })
        })
        .collect();

    indexer.join().unwrap();
    for handle in search_handles {
        handle.join().unwrap();
    }

    // Verify indexing completed
    assert!(service.is_complete());
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Recursively indexes all files in a directory.
fn index_directory_recursive(index: &mut FileIndex, dir: &Path) {
    index.begin_batch().unwrap();

    for entry in walkdir::WalkDir::new(dir)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();

        // Skip hidden files
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if name.starts_with('.') {
                continue;
            }
        }

        if let Ok(indexed) = IndexedFile::from_path(path) {
            let _ = index.add_indexed_file_batch(&indexed);
        }
    }

    index.commit_batch().unwrap();
}
