//! Extension search provider.

use std::sync::Arc;

use parking_lot::RwLock;

use crate::extensions::ExtensionManager;
use crate::search::providers::SearchProvider;
use crate::search::{ResultType, SearchResult};

#[derive(Clone)]
pub struct ExtensionProvider {
    manager: Arc<RwLock<ExtensionManager>>,
}

impl ExtensionProvider {
    #[must_use]
    pub fn new(manager: Arc<RwLock<ExtensionManager>>) -> Self {
        Self { manager }
    }
}

impl SearchProvider for ExtensionProvider {
    fn name(&self) -> &'static str {
        "Extensions"
    }

    fn result_type(&self) -> ResultType {
        ResultType::SystemCommand
    }

    fn search(&self, query: &str, max_results: usize) -> Vec<SearchResult> {
        self.manager.read().search(query, max_results)
    }

    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use parking_lot::RwLock;

    use super::*;
    use crate::extensions::ExtensionManager;
    use crate::search::providers::SearchProvider;

    #[test]
    fn test_extension_provider_construction_and_metadata() {
        let manager = Arc::new(RwLock::new(ExtensionManager::new()));
        let provider = ExtensionProvider::new(manager);

        assert_eq!(provider.name(), "Extensions");
        assert_eq!(provider.result_type(), ResultType::SystemCommand);
    }

    #[test]
    fn test_extension_provider_empty_search_returns_no_results() {
        let manager = Arc::new(RwLock::new(ExtensionManager::new()));
        let provider = ExtensionProvider::new(manager);

        let empty_query_results = provider.search("", 10);
        assert!(empty_query_results.is_empty());

        let no_loaded_extensions_results = provider.search("anything", 10);
        assert!(no_loaded_extensions_results.is_empty());
    }
}
