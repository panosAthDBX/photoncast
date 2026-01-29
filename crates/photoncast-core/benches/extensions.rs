//! Benchmarks for the Native Extension System (Task Group 10).
//!
//! Performance targets:
//! - Extension load time: <50ms
//! - Provider search time: <20ms
//! - Hot reload cycle: <250ms

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::path::PathBuf;
use tempfile::TempDir;

use photoncast_core::extensions::manifest::{
    load_manifest, validate_manifest, CommandManifest, ExtensionEntry, ExtensionInfo,
    ExtensionManifest, ManifestCache, ManifestCacheEntry, Permissions, PreferenceManifest,
    SUPPORTED_API_VERSION,
};
use photoncast_core::extensions::registry::{ExtensionRegistry, ExtensionState};

// =============================================================================
// Helper Functions
// =============================================================================

fn create_test_manifest(
    id: &str,
    num_commands: usize,
    num_preferences: usize,
) -> ExtensionManifest {
    let commands: Vec<CommandManifest> = (0..num_commands)
        .map(|i| CommandManifest {
            id: format!("cmd{}", i),
            name: format!("Command {}", i),
            mode: "search".to_string(),
            keywords: vec![format!("keyword{}", i), format!("kw{}", i)],
            icon: Some("star".to_string()),
            subtitle: Some(format!("Subtitle for command {}", i)),
        })
        .collect();

    let preferences: Vec<PreferenceManifest> = (0..num_preferences)
        .map(|i| PreferenceManifest {
            name: format!("pref{}", i),
            kind: "textfield".to_string(),
            required: i % 2 == 0,
            title: format!("Preference {}", i),
            description: Some(format!("Description for preference {}", i)),
            default: None,
            options: vec![],
        })
        .collect();

    ExtensionManifest {
        schema_version: 1,
        directory: None,
        extension: ExtensionInfo {
            id: id.to_string(),
            name: format!("Test Extension {}", id),
            version: "1.0.0".to_string(),
            description: "A test extension for benchmarking".to_string(),
            author: Some("Test Author".to_string()),
            license: Some("MIT".to_string()),
            homepage: Some("https://example.com".to_string()),
            min_photoncast_version: Some("0.1.0".to_string()),
            api_version: SUPPORTED_API_VERSION,
        },
        entry: ExtensionEntry {
            kind: "dylib".to_string(),
            path: "test.dylib".to_string(),
        },
        permissions: Permissions {
            network: true,
            clipboard: true,
            notifications: true,
            filesystem: vec!["~/Documents".to_string(), "/tmp".to_string()],
        },
        commands,
        preferences,
    }
}

fn create_manifest_file(dir: &TempDir, manifest: &ExtensionManifest) -> PathBuf {
    let dylib_path = dir.path().join("test.dylib");
    std::fs::write(&dylib_path, b"fake dylib").unwrap();

    let manifest_content = format!(
        r#"
schema_version = 1

[extension]
id = "{}"
name = "{}"
version = "{}"
description = "{}"
author = "Test Author"
license = "MIT"
homepage = "https://example.com"
min_photoncast_version = "0.1.0"
api_version = {}

[entry]
kind = "dylib"
path = "{}"

[permissions]
network = true
clipboard = true
notifications = true
filesystem = ["~/Documents", "/tmp"]
"#,
        manifest.extension.id,
        manifest.extension.name,
        manifest.extension.version,
        manifest.extension.description,
        SUPPORTED_API_VERSION,
        dylib_path.display()
    );

    let manifest_path = dir.path().join("manifest.toml");
    std::fs::write(&manifest_path, manifest_content).unwrap();
    manifest_path
}

// =============================================================================
// Task 10.7: Manifest Parsing Benchmarks
// =============================================================================

fn bench_manifest_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("manifest_parsing");

    // Benchmark parsing manifests of different complexity
    for num_commands in [0, 5, 10, 20].iter() {
        let dir = TempDir::new().unwrap();
        let manifest = create_test_manifest("com.example.bench", *num_commands, 3);
        let manifest_path = create_manifest_file(&dir, &manifest);

        group.throughput(Throughput::Elements(1));
        group.bench_with_input(
            BenchmarkId::new("load_manifest", format!("{}_commands", num_commands)),
            &manifest_path,
            |b, path| {
                b.iter(|| {
                    let result = load_manifest(black_box(path));
                    black_box(result)
                })
            },
        );
    }

    group.finish();
}

fn bench_manifest_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("manifest_validation");

    let dir = TempDir::new().unwrap();
    let manifest = create_test_manifest("com.example.bench", 10, 5);
    let manifest_path = create_manifest_file(&dir, &manifest);
    let loaded_manifest = load_manifest(&manifest_path).unwrap();

    group.bench_function("validate_manifest", |b| {
        b.iter(|| {
            let result = validate_manifest(black_box(&loaded_manifest), black_box(&manifest_path));
            black_box(result)
        })
    });

    group.finish();
}

fn bench_manifest_cache(c: &mut Criterion) {
    let mut group = c.benchmark_group("manifest_cache");

    // Setup: Create multiple manifests
    let num_manifests = 50;
    let manifests: Vec<ExtensionManifest> = (0..num_manifests)
        .map(|i| create_test_manifest(&format!("com.example.ext{}", i), 5, 2))
        .collect();

    // Benchmark cache insertion
    group.bench_function("cache_insert_50", |b| {
        b.iter(|| {
            let mut cache = ManifestCache::new();
            for (i, manifest) in manifests.iter().enumerate() {
                cache.insert(ManifestCacheEntry {
                    manifest: manifest.clone(),
                    path: PathBuf::from(format!("/tmp/ext{}/manifest.toml", i)),
                    modified_at: i as i64,
                });
            }
            black_box(cache)
        })
    });

    // Benchmark cache lookup
    let mut pre_populated_cache = ManifestCache::new();
    for (i, manifest) in manifests.iter().enumerate() {
        pre_populated_cache.insert(ManifestCacheEntry {
            manifest: manifest.clone(),
            path: PathBuf::from(format!("/tmp/ext{}/manifest.toml", i)),
            modified_at: i as i64,
        });
    }

    group.bench_function("cache_lookup", |b| {
        b.iter(|| {
            let result = pre_populated_cache.get(black_box("com.example.ext25"));
            black_box(result)
        })
    });

    group.finish();
}

// =============================================================================
// Task 10.7: Registry Operations Benchmarks
// =============================================================================

fn bench_registry_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("registry_operations");

    // Benchmark registry insert
    let manifests: Vec<ExtensionManifest> = (0..100)
        .map(|i| create_test_manifest(&format!("com.example.ext{}", i), 3, 2))
        .collect();

    group.bench_function("registry_insert_100", |b| {
        b.iter(|| {
            let mut registry = ExtensionRegistry::new();
            for manifest in &manifests {
                registry.insert(manifest.clone(), true);
            }
            black_box(registry)
        })
    });

    // Benchmark state transitions
    group.bench_function("state_transition_chain", |b| {
        b.iter(|| {
            let mut registry = ExtensionRegistry::new();
            let manifest = create_test_manifest("com.example.test", 3, 2);
            registry.insert(manifest, true);

            // Full lifecycle: Discovered -> Loaded -> Active -> Disabled -> Unloaded
            registry
                .update_state("com.example.test", ExtensionState::Loaded)
                .unwrap();
            registry
                .update_state("com.example.test", ExtensionState::Active)
                .unwrap();
            registry
                .update_state("com.example.test", ExtensionState::Disabled)
                .unwrap();
            registry
                .update_state("com.example.test", ExtensionState::Unloaded)
                .unwrap();

            black_box(registry)
        })
    });

    // Benchmark registry lookup in populated registry
    let mut pre_populated = ExtensionRegistry::new();
    for manifest in &manifests {
        pre_populated.insert(manifest.clone(), true);
    }

    group.bench_function("registry_lookup", |b| {
        b.iter(|| {
            let result = pre_populated.get(black_box("com.example.ext50"));
            black_box(result)
        })
    });

    group.bench_function("registry_list_100", |b| {
        b.iter(|| {
            let result = pre_populated.list();
            black_box(result)
        })
    });

    group.finish();
}

// =============================================================================
// Task 10.7: ID Validation Benchmarks
// =============================================================================

fn bench_id_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("id_validation");

    // Various ID formats
    let valid_ids = vec![
        "com.example.test",
        "org.photoncast.extension",
        "io.github.user.my-extension",
        "com.company.product.feature.v2",
    ];

    let invalid_ids = vec![
        "invalid",              // single segment
        "com..test",            // empty segment
        "com.test@invalid",     // special character
        "com.test.with spaces", // spaces
    ];

    for id in valid_ids {
        group.bench_with_input(BenchmarkId::new("valid", id), &id, |b, id| {
            b.iter(|| {
                // Inline validation logic for benchmarking
                let parts: Vec<&str> = id.split('.').collect();
                let valid = parts.len() >= 2
                    && !parts.iter().any(|p| p.is_empty())
                    && parts.iter().all(|p| {
                        p.chars()
                            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
                    });
                black_box(valid)
            })
        });
    }

    for id in invalid_ids {
        group.bench_with_input(BenchmarkId::new("invalid", id), &id, |b, id| {
            b.iter(|| {
                let parts: Vec<&str> = id.split('.').collect();
                let valid = parts.len() >= 2
                    && !parts.iter().any(|p| p.is_empty())
                    && parts.iter().all(|p| {
                        p.chars()
                            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
                    });
                black_box(valid)
            })
        });
    }

    group.finish();
}

// =============================================================================
// Task 10.8: Simulated Hot Reload Benchmark
// =============================================================================

fn bench_hot_reload_simulation(c: &mut Criterion) {
    let mut group = c.benchmark_group("hot_reload");
    group.sample_size(50); // Fewer samples for this longer benchmark

    // Simulate a hot reload cycle:
    // 1. Detect file change (manifest read)
    // 2. Unload extension (state transition)
    // 3. Reload manifest
    // 4. Re-validate
    // 5. Reactivate (state transitions)

    let dir = TempDir::new().unwrap();
    let manifest = create_test_manifest("com.example.hotreload", 10, 5);
    let manifest_path = create_manifest_file(&dir, &manifest);

    group.bench_function("reload_cycle", |b| {
        b.iter(|| {
            let mut registry = ExtensionRegistry::new();

            // Initial load
            let loaded = load_manifest(&manifest_path).unwrap();
            validate_manifest(&loaded, &manifest_path).unwrap();
            registry.insert(loaded.clone(), true);
            registry
                .update_state("com.example.hotreload", ExtensionState::Loaded)
                .unwrap();
            registry
                .update_state("com.example.hotreload", ExtensionState::Active)
                .unwrap();

            // Simulate reload: deactivate, unload, reload, reactivate
            registry
                .update_state("com.example.hotreload", ExtensionState::Disabled)
                .unwrap();
            registry
                .update_state("com.example.hotreload", ExtensionState::Unloaded)
                .unwrap();

            // Reload manifest
            let reloaded = load_manifest(&manifest_path).unwrap();
            validate_manifest(&reloaded, &manifest_path).unwrap();

            // Remove and re-insert
            registry.remove("com.example.hotreload");
            registry.insert(reloaded, true);
            registry
                .update_state("com.example.hotreload", ExtensionState::Loaded)
                .unwrap();
            registry
                .update_state("com.example.hotreload", ExtensionState::Active)
                .unwrap();

            black_box(registry)
        })
    });

    group.finish();
}

// =============================================================================
// Criterion Configuration
// =============================================================================

criterion_group!(
    benches,
    bench_manifest_parsing,
    bench_manifest_validation,
    bench_manifest_cache,
    bench_registry_operations,
    bench_id_validation,
    bench_hot_reload_simulation,
);

criterion_main!(benches);
