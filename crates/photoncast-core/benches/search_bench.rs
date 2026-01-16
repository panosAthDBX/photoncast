//! Search performance benchmarks.
//!
//! This module benchmarks the search engine performance including:
//! - Fuzzy matching on 200 apps
//! - Ranking on 100 results
//! - End-to-end search latency
//!
//! Target: <30ms end-to-end search latency

use std::path::PathBuf;

use chrono::Utc;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};

use photoncast_core::indexer::{AppBundleId, IndexedApp};
use photoncast_core::search::fuzzy::FuzzyMatcher;
use photoncast_core::search::index::{
    EarlyTerminationConfig, SearchIndex, UsageDataProvider, UsageRecord,
};
use photoncast_core::search::providers::{AppProvider, OptimizedAppProvider, SearchProvider};
use photoncast_core::search::ranking::{FrecencyScore, ResultRanker};
use photoncast_core::search::{SearchConfig, SearchEngine, SearchResult};

/// Creates a realistic test app with the given name and bundle ID.
fn create_test_app(name: &str, bundle_id: &str) -> IndexedApp {
    IndexedApp {
        name: name.to_string(),
        bundle_id: AppBundleId::new(bundle_id),
        path: PathBuf::from(format!("/Applications/{}.app", name)),
        icon_path: None,
        category: None,
        keywords: Vec::new(),
        last_modified: Utc::now(),
    }
}

/// Creates a set of realistic app names for benchmarking.
fn create_realistic_apps(count: usize) -> Vec<IndexedApp> {
    // Real macOS app names for realistic benchmarks
    let app_names = [
        "Safari",
        "Chrome",
        "Firefox",
        "Xcode",
        "Visual Studio Code",
        "Slack",
        "Discord",
        "Spotify",
        "Finder",
        "Mail",
        "Messages",
        "FaceTime",
        "Calendar",
        "Notes",
        "Reminders",
        "Photos",
        "Preview",
        "TextEdit",
        "Terminal",
        "Activity Monitor",
        "System Preferences",
        "App Store",
        "Books",
        "News",
        "Stocks",
        "Weather",
        "Music",
        "Podcasts",
        "TV",
        "Maps",
        "Contacts",
        "Home",
        "Shortcuts",
        "Voice Memos",
        "QuickTime Player",
        "Archive Utility",
        "Bluetooth File Exchange",
        "ColorSync Utility",
        "Console",
        "Digital Color Meter",
        "Disk Utility",
        "Grapher",
        "Keychain Access",
        "Migration Assistant",
        "Screenshot",
        "Script Editor",
        "MIDI Setup",
        "VoiceOver Utility",
        "Accessibility Inspector",
        "FileMerge",
        "Instruments",
        "Simulator",
        "Network Radar",
        "Wireless Diagnostics",
        "Directory Utility",
        "Ticket Viewer",
        "System Information",
        "Storage Management",
        "Boot Camp Assistant",
        "DVD Player",
        "Font Book",
        "Image Capture",
        "Chess",
        "Clock",
        "Photo Booth",
        "Stickies",
        "Keynote",
        "Numbers",
        "Pages",
        "GarageBand",
        "iMovie",
        "Final Cut Pro",
        "Logic Pro",
        "Motion",
        "Compressor",
        "MainStage",
        "Adobe Photoshop",
        "Adobe Illustrator",
        "Adobe InDesign",
        "Adobe Premiere Pro",
        "Adobe After Effects",
        "Adobe XD",
        "Figma",
        "Sketch",
        "Affinity Designer",
        "Affinity Photo",
        "Pixelmator Pro",
        "DaVinci Resolve",
        "OBS",
        "Zoom",
        "Microsoft Teams",
        "Microsoft Word",
        "Microsoft Excel",
        "Microsoft PowerPoint",
        "Microsoft Outlook",
        "Microsoft OneNote",
        "Notion",
        "Obsidian",
        "Bear",
        "Craft",
        "Things",
        "Todoist",
        "Fantastical",
        "Cardhop",
        "Alfred",
        "Raycast",
        "1Password",
        "Bitwarden",
        "Dropbox",
        "Google Drive",
        "iCloud",
        "OneDrive",
        "CleanMyMac",
        "DaisyDisk",
        "AppCleaner",
        "Bartender",
        "Magnet",
        "Rectangle",
        "Divvy",
        "BetterSnapTool",
        "Amphetamine",
        "Caffeine",
        "Lungo",
        "Hidden Bar",
        "Dozer",
        "iStatMenus",
        "Little Snitch",
        "Micro Snitch",
        "Macs Fan Control",
        "Turbo Boost Switcher",
        "Keka",
        "The Unarchiver",
        "Transmit",
        "Cyberduck",
        "Forklift",
        "Path Finder",
        "Commander One",
        "TablePlus",
        "Sequel Pro",
        "Postico",
        "MongoDB Compass",
        "Postman",
        "Insomnia",
        "Charles Proxy",
        "Proxyman",
        "Wireshark",
        "Docker Desktop",
        "Parallels Desktop",
        "VMware Fusion",
        "VirtualBox",
        "UTM",
        "Homebrew Cask",
        "iTerm",
        "Hyper",
        "Warp",
        "Alacritty",
        "Kitty",
        "Tower",
        "Sourcetree",
        "GitHub Desktop",
        "GitKraken",
        "Fork",
        "Sublime Text",
        "Atom",
        "Nova",
        "BBEdit",
        "CotEditor",
        "TextMate",
        "MacVim",
        "Neovide",
        "JetBrains Toolbox",
        "IntelliJ IDEA",
        "WebStorm",
        "PyCharm",
        "GoLand",
        "RustRover",
        "CLion",
        "DataGrip",
        "Rider",
        "AppCode",
        "Android Studio",
        "Flutter",
        "React Native Debugger",
        "Reactotron",
        "Flipper",
        "Proxyman",
        "HTTPie",
        "Paw",
        "RapidAPI",
        "Altair GraphQL",
        "Apollo Studio",
        "Prisma Studio",
        "Redis Desktop Manager",
        "Another Redis Desktop Manager",
        "Medis",
        "Elasticvue",
        "Robo 3T",
        "Studio 3T",
    ];

    let mut apps = Vec::with_capacity(count);
    for i in 0..count {
        let name = app_names[i % app_names.len()];
        // Add a suffix if we need more apps than unique names
        let suffix = if i >= app_names.len() {
            format!(" {}", i / app_names.len() + 1)
        } else {
            String::new()
        };
        let full_name = format!("{}{}", name, suffix);
        let bundle_id = format!("com.test.app{}", i);
        apps.push(create_test_app(&full_name, &bundle_id));
    }
    apps
}

/// Usage data provider for benchmarks.
struct BenchUsageData {
    /// Usage records keyed by bundle ID.
    records: Vec<(String, UsageRecord)>,
}

impl BenchUsageData {
    /// Creates usage data with random frecency values.
    fn new(apps: &[IndexedApp]) -> Self {
        let now = Utc::now();
        let records = apps
            .iter()
            .enumerate()
            .map(|(i, app)| {
                let launch_count = (i % 100) as u32 + 1;
                let hours_ago = (i % 72) as i64;
                let last_launched = now
                    - chrono::Duration::try_hours(hours_ago).unwrap_or(chrono::Duration::zero());
                (
                    app.bundle_id.as_str().to_string(),
                    UsageRecord {
                        launch_count,
                        last_launched,
                    },
                )
            })
            .collect();
        Self { records }
    }
}

impl UsageDataProvider for BenchUsageData {
    fn get_usage(&self, bundle_id: &str) -> Option<UsageRecord> {
        self.records
            .iter()
            .find(|(id, _)| id == bundle_id)
            .map(|(_, record)| record.clone())
    }
}

/// Benchmarks fuzzy matching performance on varying app counts.
fn bench_fuzzy_matching(c: &mut Criterion) {
    let mut group = c.benchmark_group("fuzzy_matching");

    for app_count in [50, 100, 200, 500].iter() {
        let apps = create_realistic_apps(*app_count);

        group.throughput(Throughput::Elements(*app_count as u64));
        group.bench_with_input(
            BenchmarkId::new("score_all_apps", app_count),
            &apps,
            |b, apps| {
                let mut matcher = FuzzyMatcher::default();
                b.iter(|| {
                    let query = "saf"; // Common prefix query
                    for app in apps {
                        black_box(matcher.score(query, &app.name));
                    }
                })
            },
        );
    }

    group.finish();
}

/// Benchmarks the search index building and query performance.
fn bench_search_index(c: &mut Criterion) {
    let mut group = c.benchmark_group("search_index");

    let apps = create_realistic_apps(200);
    let usage = BenchUsageData::new(&apps);

    // Benchmark index building
    group.bench_function("build_index_200_apps", |b| {
        b.iter(|| {
            black_box(SearchIndex::build(&apps, &usage));
        })
    });

    // Benchmark pre-lowercased access
    let index = SearchIndex::build(&apps, &usage);
    group.bench_function("iterate_prelowercased_200_apps", |b| {
        b.iter(|| {
            for entry in index.iter() {
                black_box(&entry.name_lower);
            }
        })
    });

    group.finish();
}

/// Benchmarks the standard AppProvider vs OptimizedAppProvider.
fn bench_app_provider_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("app_provider");

    let apps = create_realistic_apps(200);
    let usage = BenchUsageData::new(&apps);

    // Standard AppProvider
    let standard_provider = AppProvider::new();
    standard_provider.set_apps(apps.clone());

    // Optimized AppProvider
    let optimized_provider = OptimizedAppProvider::new();
    optimized_provider.build_index_with_usage(&apps, &usage);

    // Common search queries
    let queries = [
        "saf",
        "terminal",
        "code",
        "x",
        "photo",
        "system preferences",
    ];

    for query in queries.iter() {
        group.bench_with_input(BenchmarkId::new("standard", query), query, |b, query| {
            b.iter(|| {
                black_box(standard_provider.search(query, 10));
            })
        });

        group.bench_with_input(BenchmarkId::new("optimized", query), query, |b, query| {
            b.iter(|| {
                black_box(optimized_provider.search(query, 10));
            })
        });
    }

    group.finish();
}

/// Benchmarks early termination effectiveness.
fn bench_early_termination(c: &mut Criterion) {
    let mut group = c.benchmark_group("early_termination");

    let apps = create_realistic_apps(200);
    let usage = BenchUsageData::new(&apps);

    // Provider without early termination (high threshold)
    let no_termination = OptimizedAppProvider::with_config(EarlyTerminationConfig {
        threshold_multiplier: 100.0, // Effectively disabled
        min_quality_score: 0,
    });
    no_termination.build_index_with_usage(&apps, &usage);

    // Provider with early termination (default config)
    let with_termination = OptimizedAppProvider::with_config(EarlyTerminationConfig {
        threshold_multiplier: 2.0,
        min_quality_score: 50,
    });
    with_termination.build_index_with_usage(&apps, &usage);

    // Query that matches many apps
    let query = "app";

    group.bench_function("without_early_termination", |b| {
        b.iter(|| {
            black_box(no_termination.search(query, 10));
        })
    });

    group.bench_function("with_early_termination", |b| {
        b.iter(|| {
            black_box(with_termination.search(query, 10));
        })
    });

    group.finish();
}

/// Benchmarks result ranking performance.
fn bench_ranking(c: &mut Criterion) {
    let mut group = c.benchmark_group("ranking");

    // Create 100 mock search results
    let apps = create_realistic_apps(100);
    let results: Vec<SearchResult> = apps
        .iter()
        .enumerate()
        .map(|(i, app)| SearchResult {
            id: photoncast_core::search::SearchResultId::new(format!("app:{}", app.bundle_id)),
            title: app.name.clone(),
            subtitle: app.path.display().to_string(),
            icon: photoncast_core::search::IconSource::AppIcon {
                bundle_id: app.bundle_id.as_str().to_string(),
                icon_path: app.icon_path.clone(),
            },
            result_type: photoncast_core::search::ResultType::Application,
            score: (100 - i) as f64,
            match_indices: vec![0, 1, 2],
            action: photoncast_core::search::SearchAction::LaunchApp {
                bundle_id: app.bundle_id.as_str().to_string(),
                path: app.path.clone(),
            },
        })
        .collect();

    let ranker = ResultRanker::new();

    group.bench_function("rank_by_match_quality_100_results", |b| {
        b.iter(|| {
            let mut results_copy = results.clone();
            ranker.rank_by_match_quality(&mut results_copy);
            black_box(results_copy);
        })
    });

    group.bench_function("rank_with_frecency_100_results", |b| {
        let usage = BenchUsageData::new(&apps);
        b.iter(|| {
            let mut results_copy = results.clone();
            ranker.rank_with_frecency(&mut results_copy, |id| {
                let bundle_id = id.strip_prefix("app:").unwrap_or(id);
                usage
                    .get_usage(bundle_id)
                    .map(|r| FrecencyScore::calculate(r.launch_count, r.last_launched))
                    .unwrap_or_else(FrecencyScore::zero)
            });
            black_box(results_copy);
        })
    });

    group.finish();
}

/// End-to-end search benchmark - the main performance target.
fn bench_end_to_end_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("end_to_end");

    let apps = create_realistic_apps(200);
    let usage = BenchUsageData::new(&apps);

    // Create optimized provider with full setup
    let provider = OptimizedAppProvider::new();
    provider.build_index_with_usage(&apps, &usage);

    // Create search engine with the provider
    let mut engine = SearchEngine::with_config(SearchConfig {
        max_results_per_provider: 10,
        max_total_results: 20,
        ..Default::default()
    });
    engine.add_provider(provider);

    // Benchmark various query lengths
    let queries = [
        ("single_char", "s"),
        ("short_prefix", "saf"),
        ("medium_query", "safari"),
        ("long_query", "system preferences"),
        ("fuzzy_query", "sfr"),
        ("no_match", "zzzzz"),
    ];

    for (name, query) in queries.iter() {
        group.bench_with_input(BenchmarkId::new("search", name), query, |b, query| {
            b.iter(|| {
                black_box(engine.search_sync(query));
            })
        });
    }

    group.finish();
}

/// Benchmark summary - tests the main <30ms target.
fn bench_target_30ms(c: &mut Criterion) {
    let mut group = c.benchmark_group("target_30ms");
    group.sample_size(100);

    let apps = create_realistic_apps(200);
    let usage = BenchUsageData::new(&apps);

    let provider = OptimizedAppProvider::new();
    provider.build_index_with_usage(&apps, &usage);

    let mut engine = SearchEngine::with_config(SearchConfig {
        max_results_per_provider: 10,
        max_total_results: 20,
        ..Default::default()
    });
    engine.add_provider(provider);

    // This is the main benchmark - should complete in <30ms
    group.bench_function("full_search_workflow_200_apps", |b| {
        b.iter(|| {
            // Simulate a realistic search workflow
            let results = engine.search_sync("saf");
            black_box(results);
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_fuzzy_matching,
    bench_search_index,
    bench_app_provider_comparison,
    bench_early_termination,
    bench_ranking,
    bench_end_to_end_search,
    bench_target_30ms,
);
criterion_main!(benches);
