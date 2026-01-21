//! Benchmarks for clipboard operations.
//!
//! Performance targets:
//! - `clipboard_load_1000`: <100ms
//! - `clipboard_search`: <50ms
//! - encryption/decryption: <5ms for typical content

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use photoncast_clipboard::{
    config::ClipboardConfig,
    encryption::EncryptionManager,
    models::{ClipboardContentType, ClipboardItem},
    storage::ClipboardStorage,
};

fn bench_encryption(c: &mut Criterion) {
    let manager = EncryptionManager::from_machine_id("bench-machine").unwrap();

    let short_text = "Hello, World!";
    let medium_text = "x".repeat(1000);
    let long_text = "x".repeat(10000);

    let mut group = c.benchmark_group("encryption");

    group.bench_function("encrypt_short", |b| {
        b.iter(|| manager.encrypt_string(black_box(short_text)));
    });

    group.bench_function("encrypt_medium", |b| {
        b.iter(|| manager.encrypt_string(black_box(&medium_text)));
    });

    group.bench_function("encrypt_long", |b| {
        b.iter(|| manager.encrypt_string(black_box(&long_text)));
    });

    // Decrypt benchmarks
    let encrypted_short = manager.encrypt_string(short_text).unwrap();
    let encrypted_medium = manager.encrypt_string(&medium_text).unwrap();
    let encrypted_long = manager.encrypt_string(&long_text).unwrap();

    group.bench_function("decrypt_short", |b| {
        b.iter(|| manager.decrypt_string(black_box(&encrypted_short)));
    });

    group.bench_function("decrypt_medium", |b| {
        b.iter(|| manager.decrypt_string(black_box(&encrypted_medium)));
    });

    group.bench_function("decrypt_long", |b| {
        b.iter(|| manager.decrypt_string(black_box(&encrypted_long)));
    });

    group.finish();
}

fn bench_storage_load(c: &mut Criterion) {
    let config = ClipboardConfig::default();
    let storage = ClipboardStorage::open_in_memory(&config).unwrap();

    // Populate with items
    for i in 0..1000 {
        let item = ClipboardItem::text(format!("Test item number {i} with some content"));
        storage.store(&item).unwrap();
    }

    let mut group = c.benchmark_group("storage_load");

    group.bench_function("load_100", |b| {
        b.iter(|| storage.load_recent(black_box(100)));
    });

    group.bench_function("load_500", |b| {
        b.iter(|| storage.load_recent(black_box(500)));
    });

    group.bench_function("load_1000", |b| {
        b.iter(|| storage.load_recent(black_box(1000)));
    });

    group.finish();
}

fn bench_storage_search(c: &mut Criterion) {
    let config = ClipboardConfig::default();
    let storage = ClipboardStorage::open_in_memory(&config).unwrap();

    // Populate with varied items
    let topics = [
        "rust",
        "python",
        "javascript",
        "code",
        "test",
        "example",
        "function",
        "variable",
    ];
    for i in 0..1000 {
        let topic = topics[i % topics.len()];
        let item = ClipboardItem::text(format!("Item {i} about {topic} programming"));
        storage.store(&item).unwrap();
    }

    let mut group = c.benchmark_group("storage_search");

    group.bench_function("search_common", |b| {
        b.iter(|| storage.search(black_box("code")));
    });

    group.bench_function("search_rare", |b| {
        b.iter(|| storage.search(black_box("variable")));
    });

    group.bench_function("search_multi_word", |b| {
        b.iter(|| storage.search(black_box("rust programming")));
    });

    group.bench_function("search_no_results", |b| {
        b.iter(|| storage.search(black_box("nonexistent")));
    });

    group.finish();
}

fn bench_storage_store(c: &mut Criterion) {
    let config = ClipboardConfig::default();
    let storage = ClipboardStorage::open_in_memory(&config).unwrap();

    let mut group = c.benchmark_group("storage_store");

    group.bench_function("store_text", |b| {
        b.iter(|| {
            let item = ClipboardItem::text("Test content for benchmarking storage");
            storage.store(black_box(&item))
        });
    });

    group.bench_function("store_rich_text", |b| {
        b.iter(|| {
            let item = ClipboardItem::new(ClipboardContentType::RichText {
                plain: "Test content".to_string(),
                html: Some("<b>Test content</b>".to_string()),
                rtf: None,
            });
            storage.store(black_box(&item))
        });
    });

    group.bench_function("store_link", |b| {
        b.iter(|| {
            let item = ClipboardItem::new(ClipboardContentType::Link {
                url: "https://example.com/page".to_string(),
                title: Some("Example Page".to_string()),
                favicon_path: None,
            });
            storage.store(black_box(&item))
        });
    });

    group.finish();
}

fn bench_pin_operations(c: &mut Criterion) {
    let config = ClipboardConfig::default();
    let storage = ClipboardStorage::open_in_memory(&config).unwrap();

    // Create and store an item
    let item = ClipboardItem::text("Item to pin/unpin");
    storage.store(&item).unwrap();
    let id = item.id;

    let mut group = c.benchmark_group("pin_operations");

    group.bench_function("pin", |b| {
        b.iter(|| storage.set_pinned(black_box(&id), true));
    });

    group.bench_function("unpin", |b| {
        b.iter(|| storage.set_pinned(black_box(&id), false));
    });

    group.bench_function("load_pinned", |b| {
        // Pin some items first
        for i in 0..10 {
            let item = ClipboardItem::text(format!("Pinned item {i}"));
            storage.store(&item).unwrap();
            storage.set_pinned(&item.id, true).unwrap();
        }
        b.iter(|| storage.load_pinned());
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_encryption,
    bench_storage_load,
    bench_storage_search,
    bench_storage_store,
    bench_pin_operations,
);
criterion_main!(benches);
