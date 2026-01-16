# PhotonCast

A lightning-fast macOS launcher built in pure Rust using [GPUI](https://github.com/zed-industries/zed).

## Features

- Fuzzy search across applications, commands, and files
- Frecency-based ranking (frequency + recency)
- Catppuccin theming support
- Smooth animations with reduce-motion support
- Global hotkey activation

## Requirements

- macOS 12.0+
- Rust 1.75+ (see `rust-toolchain.toml`)

## Building

```bash
# Debug build
cargo build

# Release build
cargo build --release

# Run
cargo run
```

## Project Structure

```
photoncast/
├── crates/
│   ├── photoncast/           # Main binary
│   └── photoncast-core/      # Core library (search, indexing, UI components)
├── tests/                    # Integration tests
└── droidz/                   # Product specs and standards
```

## Development Status

This is an early-stage project. The following features are currently stubbed:

- **Global hotkey registration** - CGEventTap integration pending
- **Result activation** - Launch/execute functionality pending

## Testing

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Run specific test
cargo test test_name
```

## License

MIT
