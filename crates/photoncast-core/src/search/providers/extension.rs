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
