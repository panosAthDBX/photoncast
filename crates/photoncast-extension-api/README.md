# photoncast-extension-api

ABI-stable API for building native PhotonCast extensions.

This crate defines the types and traits shared between the PhotonCast host application and dynamically loaded extension dylibs. Extensions are compiled as shared libraries (`.dylib`) and loaded at runtime using [`abi_stable`](https://docs.rs/abi_stable) for safe cross-boundary FFI.

## Quick Start

### 1. Create a new crate

```toml
# Cargo.toml
[package]
name = "photoncast-ext-my-extension"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
photoncast-extension-api = { path = "../photoncast-extension-api" }
abi_stable = "0.11"
```

### 2. Implement the `Extension` trait

```rust
use abi_stable::sabi_trait::prelude::TD_Opaque;
use abi_stable::std_types::{RBox, ROption, RString, RVec};
use photoncast_extension_api::prelude::*;
use photoncast_extension_api::{
    CommandHandlerTrait, ExtensionApiResult, ExtensionManifest, Extension_TO,
};

pub struct MyExtension;

impl Extension for MyExtension {
    fn manifest(&self) -> ExtensionManifest {
        ExtensionManifest {
            id: RString::from("com.example.my-extension"),
            name: RString::from("My Extension"),
            version: RString::from("0.1.0"),
            description: ROption::RSome(RString::from("Does something useful")),
            author: ROption::RSome(RString::from("Your Name")),
            license: ROption::RSome(RString::from("MIT")),
            homepage: ROption::RNone,
            min_photoncast_version: ROption::RNone,
            api_version: 1,
        }
    }

    fn activate(&mut self, ctx: ExtensionContext) -> ExtensionApiResult<()> {
        // Store context, initialize resources
        ExtensionApiResult::ROk(())
    }

    fn deactivate(&mut self) -> ExtensionApiResult<()> {
        // Cleanup resources
        ExtensionApiResult::ROk(())
    }

    fn commands(&self) -> RVec<ExtensionCommand> {
        // Return your extension's commands
        RVec::new()
    }
}

// Required entry points
#[no_mangle]
pub extern "C" fn create_extension() -> ExtensionBox {
    Extension_TO::from_value(MyExtension, TD_Opaque)
}

#[abi_stable::export_root_module]
fn instantiate_root_module() -> ExtensionApiRootModule_Ref {
    ExtensionApiRootModule { create_extension }.leak_into_prefix()
}
```

### 3. Create an `extension.toml` manifest

```toml
schema_version = 1

[extension]
id = "com.example.my-extension"
name = "My Extension"
version = "0.1.0"
description = "Does something useful"
api_version = 1

[entry]
kind = "native"
path = "libphotoncast_ext_my_extension.dylib"

[permissions]
clipboard = true
filesystem = ["~/Documents"]

[[commands]]
id = "my-command"
name = "My Command"
mode = "view"           # "view", "search", or "no-view"
keywords = ["example"]

[[preferences]]
name = "api_key"
type = "secret"
title = "API Key"
description = "Your API key"
```

## Key Types

### Core Traits

| Trait | Purpose |
|---|---|
| `Extension` | Main trait every extension implements. Provides manifest, lifecycle hooks, commands, and optional search provider. |
| `CommandHandlerTrait` | Handles command execution. Receives `ExtensionContext` and `CommandArguments`. |
| `ExtensionSearchProvider` | Optional trait for extensions that provide search results to the launcher. |

### Commands

- **`ExtensionCommand`** — Defines a command with an id, name, mode, keywords, handler, and required permissions.
- **`CommandMode`** — `View` (renders UI), `Search` (provides search results), or `NoView` (runs silently).
- **`CommandArguments`** — Input passed to a command: optional query, selected text, clipboard content, and extra data.

### Views

Extensions render UI by calling `ctx.host.render_view(view)` with one of:

| View | Description |
|---|---|
| `ListView` | Searchable list with sections, items, accessories, and optional preview pane. |
| `DetailView` | Markdown content with metadata sidebar and actions. |
| `FormView` | Input form with typed fields and validation. |
| `GridView` | Image/icon grid layout. |

Supporting types: `ListItem`, `ListSection`, `Action`, `ActionHandler`, `Accessory`, `Preview`, `IconSource`, `ImageSource`, `EmptyState`.

### Host Capabilities

`ExtensionContext` provides access to host services:

- **`host.render_view()`** / **`host.update_view()`** — Display and update UI.
- **`host.copy_to_clipboard()`** / **`host.read_clipboard()`** — Clipboard access.
- **`host.show_toast()`** / **`host.show_hud()`** — User notifications.
- **`host.open_url()`** / **`host.open_file()`** / **`host.reveal_in_finder()`** — OS integration.
- **`host.launch_command()`** — Invoke commands from other extensions.
- **`preferences`** — Read/write user preferences.
- **`storage`** — Persistent key-value storage.
- **`cache`** — TTL-based in-memory cache.

## Permissions

Extensions declare required permissions in `extension.toml`. The host grants access based on user approval:

| Permission | Grants |
|---|---|
| `clipboard` | Read/write system clipboard |
| `filesystem` | Access specified paths (list allowed directories) |

Commands also declare their required permissions via the `permissions` field on `ExtensionCommand`.

## ABI Stability

This crate uses `abi_stable` for safe FFI across dylib boundaries. **Important:**

- The host and all extensions must use the **same `abi_stable` version**.
- Version mismatches cause load-time errors (by design, to prevent UB).
- Always rebuild extensions when updating the API crate.
- The current API version is `1` (see `EXTENSION_API_VERSION`).

## Reference Extensions

See these built-in extensions for working examples:

- [`photoncast-ext-screenshots`](../photoncast-ext-screenshots) — Browse and manage screenshots (ListView with previews, file actions, preferences, background thumbnail caching)
- [`photoncast-ext-github`](../photoncast-ext-github) — GitHub integration
- [`photoncast-ext-system-preferences`](../photoncast-ext-system-preferences) — System preferences access
