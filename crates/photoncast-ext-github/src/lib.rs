#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::must_use_candidate)]

//! `GitHub` Search Extension for PhotonCast
//!
//! Provides search functionality for `GitHub` repositories.

use abi_stable::prefix_type::PrefixTypeTrait;
use abi_stable::sabi_trait::prelude::TD_Opaque;
use abi_stable::std_types::{RBox, ROption, RResult, RString, RVec};
use photoncast_extension_api::prelude::*;
use photoncast_extension_api::{
    CommandHandlerTrait, ExtensionApiResult, ExtensionManifest, ExtensionSearchProvider_TO,
    Extension_TO,
};

/// `GitHub` repository data
#[derive(Debug, Clone)]
struct Repository {
    name: String,
    full_name: String,
    description: Option<String>,
    html_url: String,
    clone_url: String,
    ssh_url: String,
    stars: u32,
    language: Option<String>,
    owner: String,
}

impl Repository {
    /// Creates actions for this repository
    fn actions(&self) -> RVec<Action> {
        let mut actions = RVec::new();

        // Open in browser (primary action)
        actions.push(Action {
            id: RString::from("open-browser"),
            title: RString::from("Open in Browser"),
            icon: ROption::RSome(IconSource::SystemIcon {
                name: RString::from("safari"),
            }),
            shortcut: ROption::RSome(Shortcut::cmd("o")),
            style: ActionStyle::Primary,
            handler: ActionHandler::OpenUrl(RString::from(self.html_url.as_str())),
        });

        // Copy HTTPS URL
        actions.push(Action {
            id: RString::from("copy-https"),
            title: RString::from("Copy HTTPS URL"),
            icon: ROption::RSome(IconSource::SystemIcon {
                name: RString::from("doc.on.doc"),
            }),
            shortcut: ROption::RSome(Shortcut::cmd_shift("c")),
            style: ActionStyle::Default,
            handler: ActionHandler::CopyToClipboard(RString::from(self.clone_url.as_str())),
        });

        // Copy SSH URL
        actions.push(Action {
            id: RString::from("copy-ssh"),
            title: RString::from("Copy SSH URL"),
            icon: ROption::RSome(IconSource::SystemIcon {
                name: RString::from("terminal"),
            }),
            shortcut: ROption::RNone,
            style: ActionStyle::Default,
            handler: ActionHandler::CopyToClipboard(RString::from(self.ssh_url.as_str())),
        });

        // Open Issues
        let issues_url = format!("{}/issues", self.html_url);
        actions.push(Action {
            id: RString::from("open-issues"),
            title: RString::from("Open Issues"),
            icon: ROption::RSome(IconSource::SystemIcon {
                name: RString::from("exclamationmark.circle"),
            }),
            shortcut: ROption::RSome(Shortcut::cmd("i")),
            style: ActionStyle::Default,
            handler: ActionHandler::OpenUrl(RString::from(issues_url)),
        });

        // Open Pull Requests
        let prs_url = format!("{}/pulls", self.html_url);
        actions.push(Action {
            id: RString::from("open-prs"),
            title: RString::from("Open Pull Requests"),
            icon: ROption::RSome(IconSource::SystemIcon {
                name: RString::from("arrow.triangle.pull"),
            }),
            shortcut: ROption::RSome(Shortcut::cmd("p")),
            style: ActionStyle::Default,
            handler: ActionHandler::OpenUrl(RString::from(prs_url)),
        });

        actions
    }

    /// Converts to a list item
    fn to_list_item(&self) -> ListItem {
        let mut accessories = RVec::new();

        // Stars accessory
        accessories.push(Accessory::Text(RString::from(format!("★ {}", self.stars))));

        // Language tag
        if let Some(ref lang) = self.language {
            let color = language_color(lang);
            accessories.push(Accessory::Tag {
                text: RString::from(lang.as_str()),
                color,
            });
        }

        ListItem {
            id: RString::from(self.full_name.as_str()),
            title: RString::from(self.name.as_str()),
            subtitle: ROption::from(self.description.as_ref().map(|d| RString::from(d.as_str()))),
            icon: IconSource::SystemIcon {
                name: RString::from("folder"),
            },
            accessories,
            actions: self.actions(),
            preview: ROption::RSome(self.preview()),
            shortcut: ROption::RNone,
        }
    }

    /// Creates a preview for this repository
    fn preview(&self) -> Preview {
        let markdown = format!(
            "# {}\n\n{}\n\n**Owner:** {}\n**Stars:** {}\n**Language:** {}",
            self.name,
            self.description.as_deref().unwrap_or("No description"),
            self.owner,
            self.stars,
            self.language.as_deref().unwrap_or("Unknown")
        );
        Preview::Markdown(RString::from(markdown))
    }
}

/// Maps language names to tag colors
fn language_color(language: &str) -> TagColor {
    match language.to_lowercase().as_str() {
        "rust" | "swift" => TagColor::Orange,
        "python" | "go" | "c" | "c++" => TagColor::Blue,
        "javascript" | "typescript" => TagColor::Yellow,
        "ruby" => TagColor::Red,
        "java" | "kotlin" => TagColor::Purple,
        _ => TagColor::Default,
    }
}

/// `GitHub` search provider
struct GitHubSearchProvider {
    #[allow(dead_code)]
    api_token: Option<String>,
    default_org: Option<String>,
}

impl GitHubSearchProvider {
    fn new(api_token: Option<String>, default_org: Option<String>) -> Self {
        Self {
            api_token,
            default_org,
        }
    }

    /// Simulates searching `GitHub` repositories
    /// In production, this would make actual API calls
    fn search_repos(&self, query: &str, max_results: usize) -> Vec<Repository> {
        // Demo repositories for testing
        let all_repos = vec![
            Repository {
                name: "photoncast".to_string(),
                full_name: "photoncast/photoncast".to_string(),
                description: Some("Lightning-fast macOS launcher".to_string()),
                html_url: "https://github.com/photoncast/photoncast".to_string(),
                clone_url: "https://github.com/photoncast/photoncast.git".to_string(),
                ssh_url: "git@github.com:photoncast/photoncast.git".to_string(),
                stars: 1250,
                language: Some("Rust".to_string()),
                owner: "photoncast".to_string(),
            },
            Repository {
                name: "rust".to_string(),
                full_name: "rust-lang/rust".to_string(),
                description: Some(
                    "Empowering everyone to build reliable and efficient software.".to_string(),
                ),
                html_url: "https://github.com/rust-lang/rust".to_string(),
                clone_url: "https://github.com/rust-lang/rust.git".to_string(),
                ssh_url: "git@github.com:rust-lang/rust.git".to_string(),
                stars: 92000,
                language: Some("Rust".to_string()),
                owner: "rust-lang".to_string(),
            },
            Repository {
                name: "zed".to_string(),
                full_name: "zed-industries/zed".to_string(),
                description: Some(
                    "Code at the speed of thought – Zed is a high-performance, multiplayer code editor."
                        .to_string(),
                ),
                html_url: "https://github.com/zed-industries/zed".to_string(),
                clone_url: "https://github.com/zed-industries/zed.git".to_string(),
                ssh_url: "git@github.com:zed-industries/zed.git".to_string(),
                stars: 35000,
                language: Some("Rust".to_string()),
                owner: "zed-industries".to_string(),
            },
            Repository {
                name: "react".to_string(),
                full_name: "facebook/react".to_string(),
                description: Some(
                    "The library for web and native user interfaces.".to_string(),
                ),
                html_url: "https://github.com/facebook/react".to_string(),
                clone_url: "https://github.com/facebook/react.git".to_string(),
                ssh_url: "git@github.com:facebook/react.git".to_string(),
                stars: 220_000,
                language: Some("JavaScript".to_string()),
                owner: "facebook".to_string(),
            },
            Repository {
                name: "raycast-extensions".to_string(),
                full_name: "raycast/extensions".to_string(),
                description: Some("Everything you need to extend Raycast.".to_string()),
                html_url: "https://github.com/raycast/extensions".to_string(),
                clone_url: "https://github.com/raycast/extensions.git".to_string(),
                ssh_url: "git@github.com:raycast/extensions.git".to_string(),
                stars: 5200,
                language: Some("TypeScript".to_string()),
                owner: "raycast".to_string(),
            },
        ];

        let query_lower = query.to_lowercase();

        // Filter by query, considering default_org
        let mut results: Vec<_> = all_repos
            .into_iter()
            .filter(|repo| {
                let matches_query = repo.name.to_lowercase().contains(&query_lower)
                    || repo.full_name.to_lowercase().contains(&query_lower)
                    || repo
                        .description
                        .as_ref()
                        .is_some_and(|d| d.to_lowercase().contains(&query_lower));

                // If default_org is set, prefer repos from that org
                if let Some(ref org) = self.default_org {
                    if repo.owner.to_lowercase() == org.to_lowercase() {
                        return true;
                    }
                }

                matches_query
            })
            .collect();

        // Sort by stars (most popular first)
        results.sort_by(|a, b| b.stars.cmp(&a.stars));
        results.truncate(max_results);
        results
    }
}

impl ExtensionSearchProvider for GitHubSearchProvider {
    fn id(&self) -> RString {
        RString::from("github-repos")
    }

    fn name(&self) -> RString {
        RString::from("GitHub Repositories")
    }

    fn search(&self, query: RString, max_results: usize) -> RVec<ExtensionSearchItem> {
        let repos = self.search_repos(query.as_str(), max_results);

        repos
            .into_iter()
            .enumerate()
            .map(|(i, repo)| {
                #[allow(clippy::cast_precision_loss)]
                let score = 1.0 - (i as f64 * 0.1);
                ExtensionSearchItem {
                    id: RString::from(repo.full_name.as_str()),
                    title: RString::from(repo.name.as_str()),
                    subtitle: ROption::from(
                        repo.description.as_ref().map(|d| RString::from(d.as_str())),
                    ),
                    icon: IconSource::SystemIcon {
                        name: RString::from("folder"),
                    },
                    score,
                    actions: repo.actions(),
                }
            })
            .collect()
    }
}

/// Command handler for searching repositories
struct SearchReposHandler;

impl CommandHandlerTrait for SearchReposHandler {
    fn handle(&self, ctx: ExtensionContext, _args: CommandArguments) -> ExtensionApiResult<()> {
        // Get preferences
        let prefs = ctx.host.get_preferences().unwrap_or(PreferenceValues {
            values: RVec::new(),
        });

        let api_token = prefs.values.iter().find_map(|t| {
            if t.0.as_str() == "api_token" {
                if let PreferenceValue::Secret(ref s) = t.1 {
                    Some(s.to_string())
                } else {
                    None
                }
            } else {
                None
            }
        });

        let default_org = prefs.values.iter().find_map(|t| {
            if t.0.as_str() == "default_org" {
                if let PreferenceValue::String(ref s) = t.1 {
                    Some(s.to_string())
                } else {
                    None
                }
            } else {
                None
            }
        });

        let provider = GitHubSearchProvider::new(api_token, default_org);

        // Initial search with empty query shows popular repos
        let repos = provider.search_repos("", 10);

        let items: RVec<ListItem> = repos.iter().map(Repository::to_list_item).collect();

        let sections = RVec::from(vec![ListSection {
            title: ROption::RSome(RString::from("Repositories")),
            items,
        }]);

        let view = ExtensionView::List(ListView {
            title: RString::from("Search GitHub"),
            search_bar: ROption::RSome(SearchBarConfig {
                placeholder: RString::from("Search repositories..."),
                throttle_ms: 300,
            }),
            sections,
            empty_state: ROption::RSome(EmptyState {
                icon: ROption::RSome(IconSource::SystemIcon {
                    name: RString::from("magnifyingglass"),
                }),
                title: RString::from("No repositories found"),
                description: ROption::RSome(RString::from("Try a different search query")),
                actions: RVec::new(),
            }),
            show_preview: true,
        });

        match ctx.host.render_view(view) {
            RResult::ROk(_) => ExtensionApiResult::ROk(()),
            RResult::RErr(e) => ExtensionApiResult::RErr(e),
        }
    }
}

/// `GitHub` Extension
pub struct GitHubExtension {
    ctx: Option<ExtensionContext>,
}

impl GitHubExtension {
    fn new() -> Self {
        Self { ctx: None }
    }
}

impl Extension for GitHubExtension {
    fn manifest(&self) -> ExtensionManifest {
        ExtensionManifest {
            id: RString::from("com.photoncast.github"),
            name: RString::from("GitHub"),
            version: RString::from("1.0.0"),
            description: ROption::RSome(RString::from("Search GitHub repositories")),
            author: ROption::RSome(RString::from("PhotonCast")),
            license: ROption::RSome(RString::from("MIT")),
            homepage: ROption::RSome(RString::from("https://github.com/photoncast/photoncast")),
            min_photoncast_version: ROption::RNone,
            api_version: 1,
        }
    }

    fn activate(&mut self, ctx: ExtensionContext) -> ExtensionApiResult<()> {
        self.ctx = Some(ctx);
        ExtensionApiResult::ROk(())
    }

    fn deactivate(&mut self) -> ExtensionApiResult<()> {
        self.ctx = None;
        ExtensionApiResult::ROk(())
    }

    fn search_provider(&self) -> ROption<ExtensionSearchProvider_TO<'static, RBox<()>>> {
        let Some(ctx) = self.ctx.as_ref() else {
            return ROption::RNone;
        };

        // Get preferences
        let prefs = match ctx.host.get_preferences() {
            RResult::ROk(p) => p,
            RResult::RErr(_) => return ROption::RNone,
        };

        let api_token = prefs.values.iter().find_map(|t| {
            if t.0.as_str() == "api_token" {
                if let PreferenceValue::Secret(ref s) = t.1 {
                    Some(s.to_string())
                } else {
                    None
                }
            } else {
                None
            }
        });

        let default_org = prefs.values.iter().find_map(|t| {
            if t.0.as_str() == "default_org" {
                if let PreferenceValue::String(ref s) = t.1 {
                    Some(s.to_string())
                } else {
                    None
                }
            } else {
                None
            }
        });

        let provider = GitHubSearchProvider::new(api_token, default_org);
        ROption::RSome(ExtensionSearchProvider_TO::from_value(provider, TD_Opaque))
    }

    fn commands(&self) -> RVec<ExtensionCommand> {
        RVec::from(vec![ExtensionCommand {
            id: RString::from("search-repos"),
            name: RString::from("Search Repositories"),
            mode: CommandMode::Search,
            keywords: RVec::from(vec![
                RString::from("github"),
                RString::from("repo"),
                RString::from("repository"),
            ]),
            handler: CommandHandler::new(SearchReposHandler),
            icon: ROption::RSome(IconSource::SystemIcon {
                name: RString::from("folder"),
            }),
            subtitle: ROption::RSome(RString::from("Search GitHub repositories")),
            permissions: RVec::from(vec![
                RString::from("network"),
                RString::from("clipboard"),
            ]),
        }])
    }
}

/// Creates the extension instance (called by PhotonCast)
#[no_mangle]
pub extern "C" fn create_extension() -> ExtensionBox {
    Extension_TO::from_value(GitHubExtension::new(), TD_Opaque)
}

#[abi_stable::export_root_module]
fn instantiate_root_module() -> ExtensionApiRootModule_Ref {
    ExtensionApiRootModule { create_extension }.leak_into_prefix()
}
