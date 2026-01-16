# PhotonCast Development Standards

> Comprehensive coding standards, best practices, and conventions for PhotonCast - a Rust-based macOS launcher.

## Overview

These standards ensure consistent, high-quality code across the PhotonCast project. They are tailored for:
- **Pure Rust** development
- **GPUI** framework for UI
- **macOS** platform integration
- **Performance-critical** launcher application

## Quick Start

1. **New to the project?** Start with:
   - [Tech Stack](global/tech-stack.md) - Understand the architecture
   - [Coding Style](global/coding-style.md) - Code conventions
   - [Crate-First](global/crate-first.md) - **Always search for crates first!**

2. **Building UI?** Read:
   - [GPUI Components](frontend/components.md) - UI patterns and best practices

3. **Working on search/indexing?** Read:
   - [Search Engine APIs](backend/api.md) - Provider patterns
   - [macOS Platform](backend/platform.md) - System integration

## Standards Categories

### Global Standards

Apply to **ALL code** in PhotonCast:

| Standard | Description |
|----------|-------------|
| [tech-stack.md](global/tech-stack.md) | Tech stack definition and project structure |
| [crate-first.md](global/crate-first.md) | **Always search for crates before implementing** |
| [coding-style.md](global/coding-style.md) | Rust coding conventions and idioms |
| [error-handling.md](global/error-handling.md) | Error types, propagation, and logging |

### Frontend Standards (GPUI)

UI development with GPUI framework:

| Standard | Description |
|----------|-------------|
| [components.md](frontend/components.md) | GPUI component patterns and best practices |
| [css.md](frontend/css.md) | Styling with GPUI's Tailwind-like API |
| [accessibility.md](frontend/accessibility.md) | Accessibility requirements |
| [responsive.md](frontend/responsive.md) | Responsive and adaptive layouts |

### Backend Standards

Core search engine, platform integration, and extensions:

| Standard | Description |
|----------|-------------|
| [api.md](backend/api.md) | Search provider patterns and APIs |
| [platform.md](backend/platform.md) | macOS platform integration |
| [extensions.md](backend/extensions.md) | **Raycast extension compatibility** |
| [models.md](backend/models.md) | Data models and serialization |
| [queries.md](backend/queries.md) | Database and search queries |

### Testing Standards

| Standard | Description |
|----------|-------------|
| [test-writing.md](testing/test-writing.md) | Test patterns, property testing, benchmarks |

## Key Principles

### 1. Crate-First Development

> **ALWAYS search for existing crates before implementing functionality.**

```bash
# Before writing ANY non-trivial code:
cargo search "fuzzy match"
# Check lib.rs, blessed.rs, awesome-rust
```

See [crate-first.md](global/crate-first.md) for the full guide.

### 2. Type Safety

Make illegal states unrepresentable:

```rust
// Instead of
struct Connection {
    is_connected: bool,
    session: Option<Session>,
}

// Use
enum Connection {
    Disconnected,
    Connected { session: Session },
}
```

### 3. Error Handling

- Use `thiserror` for library errors
- Use `anyhow` for application errors
- Always provide context

```rust
let config = load_config(&path)
    .with_context(|| format!("failed to load config from {}", path.display()))?;
```

### 4. Async Everything

All I/O is async. Never block the main thread.

```rust
// ✅ Good
let content = tokio::fs::read_to_string(&path).await?;

// ❌ Bad
let content = std::fs::read_to_string(&path)?;
```

### 5. GPUI Patterns

```rust
impl Render for MyComponent {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .gap_2()
            .child(/* ... */)
    }
}
```

## Tech Stack Summary

| Layer | Technology |
|-------|------------|
| Language | Rust (2021 edition) |
| GUI Framework | GPUI + gpui-component |
| Async Runtime | Tokio |
| Fuzzy Matching | Nucleo |
| Storage | rusqlite / sled |
| Error Handling | thiserror + anyhow |
| macOS Integration | objc2, cocoa, core-foundation |

## How to Use These Standards

### For Developers

1. **Read before coding** - Check relevant standards before starting work
2. **Reference during development** - Standards have copy-paste examples
3. **Update when discovering patterns** - Contribute new patterns you find

### For Code Review

Use as a checklist:
- [ ] Follows [coding style](global/coding-style.md)
- [ ] Checked for [existing crates](global/crate-first.md)
- [ ] Proper [error handling](global/error-handling.md)
- [ ] Has [tests](testing/test-writing.md)
- [ ] GPUI patterns followed (if UI code)

### For AI Assistants

Standards are loaded automatically during:
- Task planning and orchestration
- Implementation guidance
- Code review

## Recommended Workflow

See [RECOMMENDED_WORKFLOW.md](RECOMMENDED_WORKFLOW.md) for the complete development workflow, including:
- Planning with specs
- Task breakdown
- Implementation
- Testing and verification

## Updates

Standards should be updated when:
- New patterns emerge in the codebase
- Framework best practices change
- Team discovers better approaches
- New crates become available

## Resources

### Rust
- [Rust Book](https://doc.rust-lang.org/book/)
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [Effective Rust](https://www.lurklurk.org/effective-rust/)

### GPUI
- [GPUI Documentation](https://gpui.rs)
- [gpui-component](https://longbridge.github.io/gpui-component/)
- [Zed Source](https://github.com/zed-industries/zed)

### Crate Discovery
- [lib.rs](https://lib.rs) - Best search
- [blessed.rs](https://blessed.rs/crates) - Curated recommendations
- [awesome-rust](https://github.com/rust-unofficial/awesome-rust) - Community list

---

*Last Updated: January 2026*
*Standards Version: 1.0.0*
*Droidz Version: 4.13.0*
