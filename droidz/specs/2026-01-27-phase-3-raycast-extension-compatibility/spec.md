# Phase 3a: Raycast Extension Compatibility — Core Sidecar Infrastructure

> **Version:** 1.0  
> **Date:** 2026-01-27  
> **Status:** Draft  
> **Phase:** 3a of 4 (3a Core Sidecar, 3b Full API, 3c DX, 3d Polish)  
> **Target:** 6 reference extensions working end-to-end  

---

## 1. Overview

### 1.1 Goal

Enable PhotonCast to run existing Raycast extensions with no modification. Phase 3a delivers the core sidecar infrastructure: Node.js process management, JSON-RPC transport, essential API shim, and enough host services to run 6 target extensions.

### 1.2 Target Extensions

| Extension | Key APIs Used | Complexity |
|-----------|--------------|------------|
| **Brew** | List, ActionPanel, shell exec, LocalStorage | Medium |
| **Kill Process** | List, shell exec (`ps`), ActionPanel | Low |
| **System Monitor** | List, Detail, shell exec, polling | Medium |
| **Set Audio Device** | List, ActionPanel, shell exec | Low |
| **Clean Keyboard** | Detail, Timer, showHUD | Low |
| **Home Assistant** | List, Detail, Form, network requests, Preferences | High |

### 1.3 Compatibility Target

80%+ of Raycast extensions should work. Phase 3a specifically targets extensions that use:
- `List`, `Detail` components (P0)
- `ActionPanel` with standard actions (P0)
- `showToast`, `showHUD` (P0)
- `Clipboard` API (P0)
- `LocalStorage` (P0)
- `getPreferenceValues` (P0)
- `environment` (P0)
- Shell command execution via Node.js `child_process` (P0)
- Network requests via `fetch` / `node-fetch` (P0)

### 1.4 Non-Goals (Phase 3a)

- `Form` component (Phase 3b)
- `Grid` component (Phase 3b)
- OAuth (Phase 3b)
- Full macOS sandbox (Phase 3b/3c — process isolation only in 3a)
- In-app extension store (Phase 3c/3d)
- Hot-reload for Raycast extensions (Phase 3c)
- Menu bar commands
- AI API

---

## 2. Architecture

### 2.1 System Overview

```
┌──────────────────────────────────────────────────────────────────┐
│                       PhotonCast App (Rust/GPUI)                 │
│                                                                  │
│  ┌──────────────────────┐   ┌─────────────────────────────────┐  │
│  │  Extension Manager   │   │  Raycast Extension Bridge       │  │
│  │  (existing)          │◄──┤  (NEW: photoncast-raycast-bridge│  │
│  │                      │   │   implements Extension trait)    │  │
│  └──────────┬───────────┘   └──────────────┬──────────────────┘  │
│             │                              │                     │
│  ┌──────────▼───────────┐   ┌──────────────▼──────────────────┐  │
│  │  Native Extensions   │   │  IPC Transport Layer            │  │
│  │  (.dylib, direct)    │   │  (NEW: photoncast-extension-ipc)│  │
│  └──────────────────────┘   │  JSON-RPC over stdio            │  │
│                             └──────────────┬──────────────────┘  │
│                                            │ stdin/stdout        │
└────────────────────────────────────────────┼─────────────────────┘
                                             │
┌────────────────────────────────────────────┼─────────────────────┐
│  Node.js Sidecar Process (per extension)   │                     │
│                                            ▼                     │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │  Bootstrap Script (host-provided)                           │ │
│  │  ├── JSON-RPC handler (stdio)                               │ │
│  │  ├── @photoncast/raycast-compat (replaces @raycast/api)     │ │
│  │  │   ├── Custom React Reconciler → JSON view tree           │ │
│  │  │   ├── Callback Registry (action IDs)                     │ │
│  │  │   ├── Host service proxies (toast, clipboard, storage)   │ │
│  │  │   └── @raycast/utils (open-source hooks)                 │ │
│  │  └── Extension bundle (dist/command.js)                     │ │
│  └─────────────────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────────────────┘
```

### 2.2 Crate Structure

```
photoncast/
├── crates/
│   ├── photoncast-extension-ipc/          # NEW — Reusable IPC protocol
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs                     # Re-exports
│   │       ├── protocol.rs               # JSON-RPC message types
│   │       ├── transport.rs              # Stdio transport (read/write)
│   │       ├── process.rs                # Process lifecycle management
│   │       └── error.rs                  # IPC error types
│   │
│   ├── photoncast-raycast-bridge/         # NEW — Raycast-specific bridge
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs                     # Re-exports
│   │       ├── bridge.rs                 # RaycastBridge: implements Extension trait
│   │       ├── manifest.rs               # package.json → internal manifest
│   │       ├── discovery.rs              # Detect Raycast extensions on disk
│   │       ├── builder.rs                # esbuild bundling pipeline
│   │       ├── host_services.rs          # Map JSON-RPC calls to host protocol
│   │       ├── permissions.rs            # Translate Raycast → PhotonCast perms
│   │       └── error.rs                  # Bridge error types
│   │
│   └── (existing crates...)
│
├── packages/
│   └── raycast-compat/                    # NEW — Node.js shim package
│       ├── package.json
│       ├── tsconfig.json
│       ├── src/
│       │   ├── index.ts                  # Main entry — re-exports @raycast/api
│       │   ├── ipc.ts                    # JSON-RPC client (stdio)
│       │   ├── reconciler.ts             # Custom React reconciler
│       │   ├── callback-registry.ts      # Action callback ID management
│       │   ├── components/
│       │   │   ├── List.tsx
│       │   │   ├── Detail.tsx
│       │   │   ├── ActionPanel.tsx
│       │   │   └── Action.tsx
│       │   ├── hooks/
│       │   │   ├── useNavigation.ts
│       │   │   └── index.ts              # Re-export @raycast/utils hooks
│       │   ├── services/
│       │   │   ├── toast.ts
│       │   │   ├── clipboard.ts
│       │   │   ├── storage.ts
│       │   │   ├── cache.ts
│       │   │   ├── preferences.ts
│       │   │   └── environment.ts
│       │   └── types.ts                  # Shared TypeScript types
│       ├── bootstrap.js                  # Entry point loaded by Node.js
│       └── dist/                         # Pre-built bundle
│           └── bootstrap.js
```

---

## 3. IPC Protocol

### 3.1 Transport

JSON-RPC 2.0 over stdio (newline-delimited JSON). Each message is a single JSON object terminated by `\n`.

```
PhotonCast (Rust)  ←──stdin/stdout──→  Node.js Sidecar
```

### 3.2 Message Types

#### Host → Sidecar (Requests)

```typescript
// Run a command
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "command.run",
  "params": {
    "command": "search-repos",
    "arguments": {},
    "preferences": { "token": "ghp_..." },
    "environment": {
      "commandName": "search-repos",
      "extensionName": "github",
      "isDevelopment": false,
      "appearance": "dark",
      "supportPath": "/Users/.../extensions/github/support",
      "assetsPath": "/Users/.../extensions/github/assets"
    }
  }
}

// Search text changed
{
  "jsonrpc": "2.0",
  "id": 2,
  "method": "search.update",
  "params": { "text": "photon" }
}

// Action triggered by user
{
  "jsonrpc": "2.0",
  "id": 3,
  "method": "action.execute",
  "params": { "callbackId": "cb_42", "itemId": "repo_123" }
}

// Selection changed
{
  "jsonrpc": "2.0",
  "id": 4,
  "method": "selection.change",
  "params": { "itemId": "repo_456" }
}

// Lifecycle
{
  "jsonrpc": "2.0",
  "id": 5,
  "method": "lifecycle.unload",
  "params": {}
}
```

#### Sidecar → Host (Requests — host services)

```typescript
// Render a view
{
  "jsonrpc": "2.0",
  "id": 100,
  "method": "ui.render",
  "params": {
    "view": {
      "type": "List",
      "isLoading": false,
      "searchBarPlaceholder": "Search packages...",
      "sections": [
        {
          "title": "Installed",
          "items": [
            {
              "id": "pkg_1",
              "title": "node",
              "subtitle": "v20.11.0",
              "icon": { "type": "builtin", "name": "Box" },
              "accessories": [{ "text": "stable" }],
              "actions": [
                {
                  "id": "cb_1",
                  "title": "Upgrade",
                  "type": "callback",
                  "shortcut": { "modifiers": ["cmd"], "key": "u" }
                },
                {
                  "id": "cb_2",
                  "title": "Uninstall",
                  "type": "callback",
                  "style": "destructive"
                },
                {
                  "title": "Open Homepage",
                  "type": "openUrl",
                  "url": "https://nodejs.org"
                }
              ]
            }
          ]
        }
      ]
    }
  }
}

// Incremental update (JSON Patch RFC 6902)
{
  "jsonrpc": "2.0",
  "id": 101,
  "method": "ui.patch",
  "params": {
    "patches": [
      { "op": "replace", "path": "/isLoading", "value": false },
      { "op": "replace", "path": "/sections/0/items/0/subtitle", "value": "v20.12.0" },
      { "op": "add", "path": "/sections/0/items/-", "value": { "id": "pkg_new", "title": "yarn" } }
    ]
  }
}

// Show toast
{
  "jsonrpc": "2.0",
  "id": 102,
  "method": "toast.show",
  "params": {
    "style": "success",
    "title": "Package upgraded",
    "message": "node v20.12.0"
  }
}

// Show HUD
{
  "jsonrpc": "2.0",
  "id": 103,
  "method": "hud.show",
  "params": { "title": "Copied to clipboard" }
}

// Clipboard
{
  "jsonrpc": "2.0",
  "id": 104,
  "method": "clipboard.copy",
  "params": { "content": "brew install node" }
}

// Storage
{
  "jsonrpc": "2.0",
  "id": 105,
  "method": "storage.set",
  "params": { "key": "favorites", "value": "[\"node\",\"python\"]" }
}

// Navigation
{
  "jsonrpc": "2.0",
  "id": 106,
  "method": "navigation.push",
  "params": { "view": { "type": "Detail", "markdown": "# Package Info\n..." } }
}

// Open URL/file
{
  "jsonrpc": "2.0",
  "id": 107,
  "method": "open.url",
  "params": { "url": "https://formulae.brew.sh/formula/node" }
}

// Close main window
{
  "jsonrpc": "2.0",
  "id": 108,
  "method": "window.close",
  "params": { "clearRootSearch": true }
}
```

### 3.3 Response Format

```typescript
// Success
{ "jsonrpc": "2.0", "id": 100, "result": { "viewHandle": "vh_1" } }

// Error
{ "jsonrpc": "2.0", "id": 100, "error": { "code": -32600, "message": "Invalid request" } }
```

### 3.4 Error Codes

| Code | Meaning |
|------|---------|
| -32700 | Parse error |
| -32600 | Invalid request |
| -32601 | Method not found |
| -32602 | Invalid params |
| -32603 | Internal error |
| -1 | Extension error (with stack trace in dev mode) |

---

## 4. Sidecar Lifecycle

### 4.1 State Machine

```
                   first command
    ┌─────────┐   invoked      ┌──────────┐
    │ Dormant ├───────────────►│ Starting │
    └─────────┘                └────┬─────┘
         ▲                          │ bootstrap.js loaded,
         │ 5-min idle               │ JSON-RPC ready
         │ timeout                  ▼
    ┌────┴────┐   command     ┌──────────┐
    │ Stopped │◄──────────────┤  Ready   │
    └────┬────┘   completed   └────┬─────┘
         │        + idle           │ command.run
         │                        ▼
         │                   ┌──────────┐
         │                   │  Active  │
         │                   └────┬─────┘
         │                        │ error / crash
         ▼                        ▼
    ┌─────────┐              ┌──────────┐
    │ Failed  │◄─────────────┤ Crashed  │ (retry up to 3x)
    └─────────┘              └──────────┘
```

### 4.2 Process Management

```rust
// crates/photoncast-extension-ipc/src/process.rs

pub struct SidecarProcess {
    child: tokio::process::Child,
    stdin: FramedWrite<ChildStdin, LinesCodec>,
    stdout: FramedRead<ChildStdout, LinesCodec>,
    state: SidecarState,
    last_activity: Instant,
    crash_count: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SidecarState {
    Dormant,
    Starting,
    Ready,
    Active,
    Stopping,
    Stopped,
    Crashed { error: String },
    Failed { reason: String },
}

pub struct SidecarConfig {
    pub node_path: PathBuf,          // Bundled Node.js binary
    pub bootstrap_path: PathBuf,     // bootstrap.js entry point
    pub extension_dir: PathBuf,      // Extension install directory
    pub support_dir: PathBuf,        // Extension data directory
    pub idle_timeout: Duration,      // Default: 5 minutes
    pub memory_limit_mb: u64,        // Default: 512 MB
    pub max_crash_retries: u32,      // Default: 3
    pub dev_mode: bool,
}

impl SidecarProcess {
    pub async fn spawn(config: &SidecarConfig) -> Result<Self, IpcError> {
        let child = Command::new(&config.node_path)
            .arg(&config.bootstrap_path)
            .arg("--extension-dir")
            .arg(&config.extension_dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .env_clear()
            .env("NODE_ENV", if config.dev_mode { "development" } else { "production" })
            .env("PHOTONCAST_EXTENSION_DIR", &config.extension_dir)
            .env("PHOTONCAST_SUPPORT_DIR", &config.support_dir)
            .env("PHOTONCAST_DEV_MODE", if config.dev_mode { "1" } else { "0" })
            .kill_on_drop(true)
            .spawn()?;
        // ...
    }

    pub async fn send(&mut self, request: RpcRequest) -> Result<RpcResponse, IpcError> { ... }
    pub async fn shutdown(&mut self) -> Result<(), IpcError> { ... }
}
```

### 4.3 Memory & Resource Limits

| Resource | Limit | Action on Exceed |
|----------|-------|-----------------|
| Heap memory | 512 MB | SIGTERM → SIGKILL after 5s |
| Open files | 1024 | OS-enforced |
| CPU time | No hard limit (monitored) | Log warning at 100% for 30s |
| Idle timeout | 5 minutes | Graceful shutdown |
| Crash retries | 3 | Mark as Failed, disable |
| Startup timeout | 10 seconds | Kill, increment crash count |

---

## 5. Raycast Bridge

### 5.1 Bridge as Extension Trait Object

The bridge presents each Raycast extension as a native `Extension` trait object, so the existing `ExtensionManager` handles it transparently.

```rust
// crates/photoncast-raycast-bridge/src/bridge.rs

use photoncast_extension_api::*;
use photoncast_extension_ipc::*;

pub struct RaycastBridge {
    manifest: RaycastManifest,
    sidecar: Option<SidecarProcess>,
    config: SidecarConfig,
    callback_registry: CallbackRegistry,
}

impl Extension for RaycastBridge {
    fn metadata(&self) -> ExtensionMetadata {
        self.manifest.to_extension_metadata()
    }

    fn activate(&mut self) -> RResult<(), ExtensionApiError> {
        // Sidecar is lazy-spawned, nothing to do here
        ROk(())
    }

    fn deactivate(&mut self) -> RResult<(), ExtensionApiError> {
        if let Some(ref mut sidecar) = self.sidecar {
            // Block on async shutdown (bridge is called from sync context)
            let _ = tokio::runtime::Handle::current()
                .block_on(sidecar.shutdown());
        }
        self.sidecar = None;
        ROk(())
    }

    fn search(&self, query: &str) -> RResult<RVec<SearchItem>, ExtensionApiError> {
        // Return static command list from manifest (no sidecar needed)
        let items = self.manifest.commands.iter()
            .filter(|cmd| fuzzy_matches(query, &cmd.title))
            .map(|cmd| SearchItem {
                id: cmd.name.clone().into(),
                title: cmd.title.clone().into(),
                subtitle: self.manifest.title.clone().into(),
                icon: self.manifest.icon_path().into(),
                // ...
            })
            .collect();
        ROk(items)
    }

    fn run_command(
        &mut self,
        command_id: &str,
        args: ROption<CommandArguments>,
        host: &dyn ExtensionHostProtocol,
    ) -> RResult<CommandInvocationResult, ExtensionApiError> {
        // Lazy-spawn sidecar if needed
        if self.sidecar.is_none() {
            self.sidecar = Some(self.spawn_sidecar()?);
        }

        let sidecar = self.sidecar.as_mut().unwrap();

        // Send command.run via JSON-RPC
        let response = self.send_command_run(sidecar, command_id, host)?;

        // Process response — render view, show toast, etc.
        self.handle_response(response, host)
    }
}
```

### 5.2 Manifest Translation

```rust
// crates/photoncast-raycast-bridge/src/manifest.rs

#[derive(Debug, Deserialize)]
pub struct RaycastManifest {
    pub name: String,
    pub title: String,
    pub description: String,
    pub icon: Option<String>,
    pub author: String,
    pub license: Option<String>,
    pub commands: Vec<RaycastCommand>,
    pub preferences: Option<Vec<RaycastPreference>>,
    pub dependencies: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
pub struct RaycastCommand {
    pub name: String,
    pub title: String,
    pub subtitle: Option<String>,
    pub description: Option<String>,
    pub mode: String,          // "view" | "no-view"
    pub keywords: Option<Vec<String>>,
    pub arguments: Option<Vec<RaycastArgument>>,
}

impl RaycastManifest {
    pub fn from_package_json(path: &Path) -> Result<Self, ManifestError> {
        let content = std::fs::read_to_string(path)?;
        let package: serde_json::Value = serde_json::from_str(&content)?;
        // Extract Raycast-specific fields from package.json
        // ...
    }

    pub fn to_extension_metadata(&self) -> ExtensionMetadata {
        ExtensionMetadata {
            id: format!("com.raycast.{}", self.name),
            name: self.title.clone(),
            description: self.description.clone(),
            version: "1.0.0".to_string(), // from package.json version field
            author: self.author.clone(),
            entry_point: "dist/index.js".to_string(),
        }
    }
}
```

### 5.3 Discovery

```rust
// crates/photoncast-raycast-bridge/src/discovery.rs

pub fn discover_raycast_extensions(extensions_dir: &Path) -> Vec<RaycastManifest> {
    let mut manifests = Vec::new();
    for entry in fs::read_dir(extensions_dir).ok().into_iter().flatten() {
        let path = entry.path();
        let package_json = path.join("package.json");
        if package_json.exists() {
            if let Ok(manifest) = RaycastManifest::from_package_json(&package_json) {
                if manifest.is_raycast_extension() {
                    manifests.push(manifest);
                }
            }
        }
    }
    manifests
}

impl RaycastManifest {
    pub fn is_raycast_extension(&self) -> bool {
        self.dependencies.contains_key("@raycast/api")
    }
}
```

### 5.4 Build Pipeline

```rust
// crates/photoncast-raycast-bridge/src/builder.rs

pub struct ExtensionBuilder {
    esbuild_path: PathBuf,    // Bundled esbuild binary
    node_path: PathBuf,       // Bundled Node.js
}

impl ExtensionBuilder {
    /// Build a Raycast extension from source
    pub async fn build(&self, extension_dir: &Path) -> Result<BuildResult, BuildError> {
        // 1. Check if pre-built bundle exists
        let dist = extension_dir.join("dist");
        if dist.exists() && self.is_bundle_fresh(&dist, extension_dir) {
            return Ok(BuildResult::PreBuilt(dist));
        }

        // 2. Install dependencies if needed
        let node_modules = extension_dir.join("node_modules");
        if !node_modules.exists() {
            self.npm_install(extension_dir).await?;
        }

        // 3. Bundle with esbuild
        let output = Command::new(&self.esbuild_path)
            .args(&[
                "src/*.tsx", "src/*.ts",
                "--bundle",
                "--outdir=dist",
                "--format=cjs",
                "--platform=node",
                "--target=node18",
                "--external:@raycast/api",     // Replaced by shim at runtime
                "--external:react",             // Provided by shim
                "--external:react-reconciler",  // Provided by shim
            ])
            .current_dir(extension_dir)
            .output()
            .await?;

        if !output.status.success() {
            return Err(BuildError::EsbuildFailed(
                String::from_utf8_lossy(&output.stderr).to_string()
            ));
        }

        Ok(BuildResult::Built(dist))
    }
}
```

---

## 6. React Reconciler & API Shim

### 6.1 Custom Reconciler

The reconciler captures React's component tree and serializes it to our JSON view schema.

```typescript
// packages/raycast-compat/src/reconciler.ts

import Reconciler from 'react-reconciler';

interface ViewNode {
  type: string;
  props: Record<string, unknown>;
  children: ViewNode[];
}

let currentTree: ViewNode | null = null;
let pendingUpdates = false;

const reconciler = Reconciler({
  createInstance(type: string, props: Record<string, unknown>) {
    return { type, props: filterProps(props), children: [] };
  },

  appendInitialChild(parent: ViewNode, child: ViewNode) {
    parent.children.push(child);
  },

  appendChild(parent: ViewNode, child: ViewNode) {
    parent.children.push(child);
    scheduleFlush();
  },

  removeChild(parent: ViewNode, child: ViewNode) {
    parent.children = parent.children.filter(c => c !== child);
    scheduleFlush();
  },

  commitUpdate(instance: ViewNode, _type: string, _old: any, newProps: any) {
    instance.props = filterProps(newProps);
    scheduleFlush();
  },

  // ... other reconciler methods (minimal implementation)

  supportsMutation: true,
  supportsPersistence: false,
});

function scheduleFlush() {
  if (!pendingUpdates) {
    pendingUpdates = true;
    queueMicrotask(() => {
      pendingUpdates = false;
      if (currentTree) {
        ipc.send('ui.render', { view: serializeTree(currentTree) });
      }
    });
  }
}

function serializeTree(node: ViewNode): unknown {
  // Convert ViewNode tree → JSON matching our IPC protocol schema
  // Maps React component types to our view types:
  //   <List> → { type: "List", ... }
  //   <List.Item> → item in sections array
  //   <ActionPanel> → actions array on items
}
```

### 6.2 Callback Registry

```typescript
// packages/raycast-compat/src/callback-registry.ts

type CallbackFn = () => void | Promise<void>;

const callbacks = new Map<string, CallbackFn>();
let nextId = 0;

export function registerCallback(fn: CallbackFn): string {
  const id = `cb_${nextId++}`;
  callbacks.set(id, fn);
  return id;
}

export function executeCallback(id: string): Promise<void> {
  const fn = callbacks.get(id);
  if (!fn) throw new Error(`Unknown callback: ${id}`);
  return Promise.resolve(fn());
}

export function clearCallbacks(): void {
  callbacks.clear();
  nextId = 0;
}
```

### 6.3 Component Implementations

```typescript
// packages/raycast-compat/src/components/List.tsx

import React from 'react';
import { registerCallback } from '../callback-registry';

export function List({ children, isLoading, searchBarPlaceholder, ...props }) {
  return React.createElement('List', {
    isLoading,
    searchBarPlaceholder,
    onSearchTextChange: props.onSearchTextChange
      ? registerCallback(props.onSearchTextChange)
      : undefined,
    ...props,
  }, children);
}

List.Item = function ListItem({ title, subtitle, icon, accessories, actions, detail, ...props }) {
  return React.createElement('List.Item', {
    id: props.id,
    title,
    subtitle,
    icon: serializeIcon(icon),
    accessories: accessories?.map(serializeAccessory),
    actions: actions, // ActionPanel renders as children
    ...props,
  });
};

List.Section = function ListSection({ title, subtitle, children }) {
  return React.createElement('List.Section', { title, subtitle }, children);
};

List.EmptyView = function EmptyView({ title, description, icon }) {
  return React.createElement('List.EmptyView', { title, description, icon: serializeIcon(icon) });
};
```

### 6.4 Host Service Proxies

```typescript
// packages/raycast-compat/src/services/toast.ts

import { ipc } from '../ipc';

export enum ToastStyle {
  Success = 'success',
  Failure = 'failure',
  Animated = 'animated',
}

export async function showToast(options: {
  style?: ToastStyle;
  title: string;
  message?: string;
}): Promise<Toast> {
  const result = await ipc.call('toast.show', {
    style: options.style ?? ToastStyle.Success,
    title: options.title,
    message: options.message,
  });
  return new Toast(result.id, options);
}

export async function showHUD(title: string): Promise<void> {
  await ipc.call('hud.show', { title });
}
```

```typescript
// packages/raycast-compat/src/services/storage.ts

import { ipc } from '../ipc';

export const LocalStorage = {
  async getItem(key: string): Promise<string | undefined> {
    const result = await ipc.call('storage.get', { key });
    return result.value ?? undefined;
  },
  async setItem(key: string, value: string): Promise<void> {
    await ipc.call('storage.set', { key, value });
  },
  async removeItem(key: string): Promise<void> {
    await ipc.call('storage.remove', { key });
  },
  async allItems(): Promise<Record<string, string>> {
    const result = await ipc.call('storage.allItems', {});
    return result.items;
  },
  async clear(): Promise<void> {
    await ipc.call('storage.clear', {});
  },
};
```

```typescript
// packages/raycast-compat/src/services/environment.ts

export const environment = {
  get commandName() { return globalThis.__PHOTONCAST_COMMAND__ ?? ''; },
  get extensionName() { return globalThis.__PHOTONCAST_EXTENSION__ ?? ''; },
  get isDevelopment() { return process.env.PHOTONCAST_DEV_MODE === '1'; },
  get appearance() { return process.env.PHOTONCAST_APPEARANCE ?? 'dark'; },
  get supportPath() { return process.env.PHOTONCAST_SUPPORT_DIR ?? ''; },
  get assetsPath() { return process.env.PHOTONCAST_ASSETS_DIR ?? ''; },
  commandMode: 'view' as const,
  launchType: 'userInitiated' as const,
  textSize: 'medium' as const,
  raycastVersion: '1.50.0', // Compatibility stub
};

export function getPreferenceValues<T>(): T {
  return globalThis.__PHOTONCAST_PREFERENCES__ as T;
}
```

### 6.5 Bootstrap Script

```javascript
// packages/raycast-compat/bootstrap.js

const { createInterface } = require('readline');
const { executeCallback, clearCallbacks } = require('./dist/callback-registry');

// JSON-RPC handler over stdio
const rl = createInterface({ input: process.stdin });
const pending = new Map(); // id → { resolve, reject }
let nextId = 1;

// Receive messages from host
rl.on('line', (line) => {
  try {
    const msg = JSON.parse(line);

    if (msg.result !== undefined || msg.error !== undefined) {
      // Response to our request
      const handler = pending.get(msg.id);
      if (handler) {
        pending.delete(msg.id);
        if (msg.error) handler.reject(new Error(msg.error.message));
        else handler.resolve(msg.result);
      }
    } else {
      // Request from host
      handleHostRequest(msg);
    }
  } catch (e) {
    sendError(null, -32700, `Parse error: ${e.message}`);
  }
});

async function handleHostRequest(msg) {
  try {
    switch (msg.method) {
      case 'command.run':
        clearCallbacks();
        await runCommand(msg.params);
        sendResult(msg.id, { status: 'ok' });
        break;

      case 'action.execute':
        await executeCallback(msg.params.callbackId);
        sendResult(msg.id, { status: 'ok' });
        break;

      case 'search.update':
        // Trigger onSearchTextChange callback if registered
        if (globalThis.__onSearchTextChange__) {
          globalThis.__onSearchTextChange__(msg.params.text);
        }
        sendResult(msg.id, { status: 'ok' });
        break;

      case 'lifecycle.unload':
        sendResult(msg.id, { status: 'ok' });
        process.exit(0);
        break;

      default:
        sendError(msg.id, -32601, `Unknown method: ${msg.method}`);
    }
  } catch (e) {
    sendError(msg.id, -1, e.message, e.stack);
  }
}

// Send request to host
globalThis.__ipc_call__ = function(method, params) {
  return new Promise((resolve, reject) => {
    const id = nextId++;
    pending.set(id, { resolve, reject });
    const msg = JSON.stringify({ jsonrpc: '2.0', id, method, params });
    process.stdout.write(msg + '\n');
  });
};

function sendResult(id, result) {
  process.stdout.write(JSON.stringify({ jsonrpc: '2.0', id, result }) + '\n');
}

function sendError(id, code, message, data) {
  process.stdout.write(JSON.stringify({
    jsonrpc: '2.0', id,
    error: { code, message, data }
  }) + '\n');
}

async function runCommand(params) {
  // Inject environment
  globalThis.__PHOTONCAST_COMMAND__ = params.command;
  globalThis.__PHOTONCAST_EXTENSION__ = params.environment.extensionName;
  globalThis.__PHOTONCAST_PREFERENCES__ = params.preferences;

  // Set environment variables
  Object.assign(process.env, {
    PHOTONCAST_SUPPORT_DIR: params.environment.supportPath,
    PHOTONCAST_ASSETS_DIR: params.environment.assetsPath,
    PHOTONCAST_APPEARANCE: params.environment.appearance,
    PHOTONCAST_DEV_MODE: params.environment.isDevelopment ? '1' : '0',
  });

  // Load and render the command
  const React = require('react');
  const { render } = require('./dist/reconciler');
  const commandModule = require(
    path.join(params.environment.extensionDir, 'dist', `${params.command}.js`)
  );

  const Command = commandModule.default || commandModule;
  render(React.createElement(Command));
}

// Signal ready
process.stdout.write(JSON.stringify({
  jsonrpc: '2.0', method: 'lifecycle.ready', params: {}
}) + '\n');
```

---

## 7. Host Services Mapping

### 7.1 Phase 3a Scope

| JSON-RPC Method | Rust Handler | Maps To |
|-----------------|-------------|---------|
| `ui.render` | `RaycastBridge::handle_render` | `ExtensionHostProtocol::render_view` |
| `ui.patch` | `RaycastBridge::handle_patch` | `ExtensionHostProtocol::update_view` |
| `toast.show` | `RaycastBridge::handle_toast` | `ExtensionHostProtocol::show_toast` |
| `hud.show` | `RaycastBridge::handle_hud` | `ExtensionHostProtocol::show_hud` |
| `clipboard.copy` | `RaycastBridge::handle_clipboard_copy` | `ExtensionHostProtocol::copy_to_clipboard` |
| `clipboard.read` | `RaycastBridge::handle_clipboard_read` | `ExtensionHostProtocol::read_clipboard` |
| `storage.get` | `RaycastBridge::handle_storage_get` | `ExtensionStorage::get` |
| `storage.set` | `RaycastBridge::handle_storage_set` | `ExtensionStorage::set` |
| `storage.remove` | `RaycastBridge::handle_storage_remove` | `ExtensionStorage::remove` |
| `storage.allItems` | `RaycastBridge::handle_storage_all` | `ExtensionStorage::all_items` |
| `storage.clear` | `RaycastBridge::handle_storage_clear` | `ExtensionStorage::clear` |
| `open.url` | `RaycastBridge::handle_open_url` | `ExtensionHostProtocol::open_url` |
| `open.file` | `RaycastBridge::handle_open_file` | `ExtensionHostProtocol::open_file` |
| `window.close` | `RaycastBridge::handle_close` | Hide launcher window |
| `navigation.push` | `RaycastBridge::handle_nav_push` | `ExtensionHostProtocol::render_view` (push) |
| `navigation.pop` | `RaycastBridge::handle_nav_pop` | Navigation stack pop |

### 7.2 View Type Mapping

```rust
// In host_services.rs — convert JSON view to ExtensionView

fn json_to_extension_view(json: &serde_json::Value) -> Result<ExtensionView, BridgeError> {
    match json["type"].as_str() {
        Some("List") => Ok(ExtensionView::List(parse_list_view(json)?)),
        Some("Detail") => Ok(ExtensionView::Detail(parse_detail_view(json)?)),
        // Form and Grid deferred to Phase 3b
        Some(other) => Err(BridgeError::UnsupportedViewType(other.to_string())),
        None => Err(BridgeError::MissingViewType),
    }
}
```

---

## 8. Permission Translation

```rust
// crates/photoncast-raycast-bridge/src/permissions.rs

pub fn translate_permissions(manifest: &RaycastManifest) -> Vec<PhotonCastPermission> {
    let mut perms = vec![
        // All Raycast extensions get basic permissions
        PhotonCastPermission::Network,       // fetch, node-fetch
        PhotonCastPermission::Storage,       // LocalStorage, Cache
    ];

    // Infer from command modes
    if manifest.commands.iter().any(|c| c.mode == "view") {
        perms.push(PhotonCastPermission::UserInterface);
    }

    // Infer from dependencies
    if manifest.dependencies.contains_key("run-applescript") {
        perms.push(PhotonCastPermission::SystemCommands);
    }

    // Infer from preferences
    if let Some(prefs) = &manifest.preferences {
        if prefs.iter().any(|p| p.pref_type == "appPicker") {
            perms.push(PhotonCastPermission::ApplicationAccess);
        }
    }

    // Shell access — needed by Brew, Kill Process, System Monitor, etc.
    // Grant by default since most target extensions need it
    perms.push(PhotonCastPermission::ShellAccess);

    perms
}
```

---

## 9. Security Model (Phase 3a)

### 9.1 Process Isolation

- Each extension runs in a separate Node.js process
- `env_clear()` — no inherited environment variables
- Only whitelisted env vars injected: `NODE_ENV`, `PHOTONCAST_*`, `HOME`, `PATH`
- `kill_on_drop(true)` — child killed if parent crashes
- Stderr captured and logged (not passed to user)

### 9.2 Future Sandbox (Phase 3b/3c)

Full macOS sandbox profile deferred. Phase 3a relies on:
- Process isolation (separate address space)
- Controlled environment variables
- Memory limits (512MB, enforced via monitoring)
- Controlled IPC (all host access goes through JSON-RPC)

---

## 10. Performance Targets

| Operation | Target | Notes |
|-----------|--------|-------|
| Cold start (first command) | < 1s | Includes Node.js spawn + bootstrap + render |
| Warm command execution | < 200ms | Sidecar already running |
| UI render (initial) | < 100ms | JSON parse + GPUI render |
| UI update (incremental) | < 50ms | JSON Patch application |
| Search text update | < 100ms | Round-trip to sidecar |
| Idle memory per sidecar | < 50MB | Node.js baseline |
| Active memory per sidecar | < 150MB | With extension loaded |

---

## 11. Error Handling

### 11.1 Extension Errors

```rust
#[derive(Debug, thiserror::Error)]
pub enum BridgeError {
    #[error("Sidecar process failed to start: {0}")]
    SpawnFailed(String),

    #[error("Sidecar process crashed: {0}")]
    SidecarCrashed(String),

    #[error("IPC timeout after {0:?}")]
    Timeout(Duration),

    #[error("JSON-RPC error {code}: {message}")]
    RpcError { code: i32, message: String },

    #[error("Unsupported view type: {0}")]
    UnsupportedViewType(String),

    #[error("Build failed: {0}")]
    BuildFailed(String),

    #[error("Invalid manifest: {0}")]
    InvalidManifest(String),
}
```

### 11.2 Error Display Strategy

- **Transient errors** (network, timeout): Show toast with retry option
- **Fatal errors** (crash, invalid manifest): Show error view replacing extension view, with stack trace in dev mode
- **Build errors**: Show detail view with esbuild output

---

## 12. Developer CLI

### 12.1 `photoncast dev <path>`

Loads a local Raycast extension in development mode:

```bash
# Load extension from local directory
photoncast dev ~/code/my-extension

# Behavior:
# 1. Reads package.json from the directory
# 2. Runs npm install if node_modules missing
# 3. Bundles with esbuild
# 4. Registers extension with ExtensionManager (dev_mode=true)
# 5. Enables source maps
# 6. Logs stderr output to terminal
# 7. File watcher triggers rebuild on source changes (Phase 3c)
```

Implementation: CLI writes a registration request to a Unix domain socket or named pipe that the running PhotonCast app listens on.

---

## 13. Testing Strategy

### 13.1 Unit Tests

- JSON-RPC message serialization/deserialization
- Manifest translation (package.json → internal)
- Permission mapping
- View type conversion (JSON → ExtensionView)

### 13.2 Integration Tests

- Sidecar lifecycle: spawn → ready → active → idle → stop
- Full command round-trip: run command → render view → execute action
- Host service calls: toast, clipboard, storage through IPC
- Error recovery: crash → retry → fail

### 13.3 Extension Compatibility Tests

For each of the 6 target extensions:
1. Verify manifest parses correctly
2. Verify extension builds (if from source)
3. Verify first command renders a view
4. Verify primary action works
5. Verify search/filtering works
6. Measure cold start and warm operation times

---

## 14. Bundled Assets

### 14.1 Size Budget

| Asset | Size | Notes |
|-------|------|-------|
| Node.js binary | ~40 MB | Stripped, compressed in app bundle |
| esbuild binary | ~9 MB | Single Go binary |
| raycast-compat package | ~500 KB | Pre-bundled |
| React + reconciler | ~150 KB | Bundled with shim |
| **Total** | ~50 MB | Added to app size |

### 14.2 Node.js Version

Bundle Node.js 20 LTS (matching Raycast's runtime). Managed as a binary asset within the app bundle at `PhotonCast.app/Contents/Resources/node`.

---

## 15. Success Criteria

### 15.1 Phase 3a Complete When

- [ ] All 6 target extensions install and run
- [ ] List and Detail views render correctly
- [ ] ActionPanel with callback actions works
- [ ] Toast and HUD notifications display
- [ ] Clipboard copy/read works
- [ ] LocalStorage persists across sessions
- [ ] Preferences UI collects and passes values
- [ ] Cold start < 1s, warm operations < 200ms
- [ ] Sidecar idle timeout and crash recovery work
- [ ] `photoncast dev <path>` loads local extensions
- [ ] All unit and integration tests pass

### 15.2 Metrics

- 6/6 target extensions working end-to-end
- 0 crashes in 1 hour of normal use per extension
- Memory stays under 150MB per active sidecar
- IPC round-trip latency < 50ms (p95)

---

## 16. Risks

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Custom React reconciler complexity | Medium | High | Start minimal (List only), iterate |
| Node.js cold start too slow | Low | Medium | Pre-warm, optimize bootstrap |
| Target extensions use unsupported APIs | Medium | Medium | Stub with warnings, prioritize by usage |
| IPC overhead for rapid UI updates | Low | Medium | JSON Patch batching, debounce |
| esbuild bundling failures | Low | Low | Pre-built bundles as fallback |
| Memory bloat from multiple sidecars | Medium | Medium | Aggressive idle timeout, limit concurrent |

---

## 17. Dependencies

### 17.1 Existing Components Used

- `ExtensionManager` — handles registration and lifecycle
- `ExtensionHostProtocol` — host service interface
- `ExtensionView` (List, Detail) — native view rendering
- `ExtensionStorage` — SQLite-backed storage
- `PreferenceStore` — preference management
- Navigation system — push/pop views
- Action system — action handlers and execution

### 17.2 New Dependencies

```toml
# photoncast-extension-ipc/Cargo.toml
[dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1", features = ["process", "io-util", "sync", "time"] }
tokio-util = { version = "0.7", features = ["codec"] }
json-patch = "3.0"           # RFC 6902 JSON Patch
thiserror = "2.0"
tracing = "0.1"

# photoncast-raycast-bridge/Cargo.toml
[dependencies]
photoncast-extension-api = { path = "../photoncast-extension-api" }
photoncast-extension-ipc = { path = "../photoncast-extension-ipc" }
photoncast-core = { path = "../photoncast-core" }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1", features = ["rt", "process"] }
thiserror = "2.0"
tracing = "0.1"
nucleo = "0.5"               # Fuzzy matching for search
```

```json
// packages/raycast-compat/package.json
{
  "dependencies": {
    "react": "^18.2.0",
    "react-reconciler": "^0.29.0",
    "@raycast/utils": "^1.10.0"
  },
  "devDependencies": {
    "typescript": "^5.3.0",
    "esbuild": "^0.19.0"
  }
}
```
