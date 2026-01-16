# Raycast Extension Compatibility

## Overview

PhotonCast aims to be **compatible with Raycast extensions**, allowing users to install and run extensions from the Raycast Store. This requires implementing a compatibility layer that translates Raycast's React-based API to PhotonCast's native GPUI interface.

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                      PhotonCast (Rust/GPUI)                 │
├─────────────────────────────────────────────────────────────┤
│                    Extension Host Manager                    │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────┐ │
│  │  Native Rust    │  │  Sidecar Node   │  │  Extension  │ │
│  │  Extensions     │  │  Process        │  │  Store API  │ │
│  └─────────────────┘  └─────────────────┘  └─────────────┘ │
│           │                   │                    │        │
│           └───────────────────┼────────────────────┘        │
│                               │                             │
│              ┌────────────────▼────────────────┐            │
│              │   Extension Bridge Protocol    │            │
│              │   (IPC: JSON-RPC over stdio)   │            │
│              └────────────────┬────────────────┘            │
│                               │                             │
├───────────────────────────────┼─────────────────────────────┤
│                    Sidecar Process (Node.js)                │
│  ┌─────────────────────────────────────────────────────────┐│
│  │                 Raycast API Shim Layer                  ││
│  │  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌────────────┐ ││
│  │  │ @raycast │ │  React   │ │ Storage  │ │ Clipboard  │ ││
│  │  │   /api   │ │ Renderer │ │   API    │ │    API     │ ││
│  │  └──────────┘ └──────────┘ └──────────┘ └────────────┘ ││
│  └─────────────────────────────────────────────────────────┘│
│                               │                             │
│              ┌────────────────▼────────────────┐            │
│              │     Raycast Extension Code      │            │
│              │     (TypeScript/React)          │            │
│              └─────────────────────────────────┘            │
└─────────────────────────────────────────────────────────────┘
```

## Raycast Extension Structure

### package.json Manifest
```json
{
  "name": "my-extension",
  "title": "My Extension",
  "description": "Does something useful",
  "icon": "icon.png",
  "author": "username",
  "license": "MIT",
  "commands": [
    {
      "name": "index",
      "title": "Main Command",
      "description": "The main command",
      "mode": "view"
    },
    {
      "name": "quick-action",
      "title": "Quick Action",
      "mode": "no-view"
    }
  ],
  "preferences": [
    {
      "name": "apiKey",
      "type": "password",
      "required": true,
      "title": "API Key",
      "description": "Your API key"
    }
  ],
  "dependencies": {
    "@raycast/api": "^1.0.0"
  }
}
```

### Command Modes
| Mode | Description | PhotonCast Equivalent |
|------|-------------|----------------------|
| `view` | Shows a UI (List, Grid, Detail, Form) | Push GPUI view |
| `no-view` | Runs without UI, shows toast | Execute action, show notification |
| `menu-bar` | Menu bar item | System tray integration |

## ✅ DO

### DO: Use IPC for Extension Communication

**✅ DO**: Communicate with Node.js sidecar via JSON-RPC
```rust
use serde::{Deserialize, Serialize};
use tokio::process::{Child, Command};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

#[derive(Serialize)]
#[serde(tag = "method", content = "params")]
enum RpcRequest {
    #[serde(rename = "extension.run")]
    RunExtension { extension_id: String, command: String },
    
    #[serde(rename = "extension.action")]
    ExecuteAction { action_id: String, item_id: Option<String> },
    
    #[serde(rename = "ui.update")]
    UpdateUI { component: UIUpdate },
}

#[derive(Deserialize)]
#[serde(tag = "type")]
enum RpcResponse {
    #[serde(rename = "ui.render")]
    Render { view: ExtensionView },
    
    #[serde(rename = "action.complete")]
    ActionComplete { success: bool, message: Option<String> },
    
    #[serde(rename = "toast")]
    ShowToast { style: ToastStyle, title: String, message: Option<String> },
}

pub struct ExtensionHost {
    process: Child,
    stdin: tokio::process::ChildStdin,
    stdout: BufReader<tokio::process::ChildStdout>,
}

impl ExtensionHost {
    pub async fn send(&mut self, request: RpcRequest) -> Result<RpcResponse> {
        let json = serde_json::to_string(&request)?;
        self.stdin.write_all(json.as_bytes()).await?;
        self.stdin.write_all(b"\n").await?;
        self.stdin.flush().await?;
        
        let mut line = String::new();
        self.stdout.read_line(&mut line).await?;
        
        let response: RpcResponse = serde_json::from_str(&line)?;
        Ok(response)
    }
}
```

### DO: Implement Core Raycast UI Components

**✅ DO**: Map Raycast components to GPUI equivalents
```rust
/// Raycast List component -> GPUI equivalent
#[derive(Deserialize)]
pub struct RaycastList {
    pub is_loading: Option<bool>,
    pub search_bar_placeholder: Option<String>,
    pub filtering: Option<bool>,
    pub children: Vec<RaycastListItem>,
}

#[derive(Deserialize)]
pub struct RaycastListItem {
    pub id: String,
    pub title: String,
    pub subtitle: Option<String>,
    pub icon: Option<RaycastIcon>,
    pub accessories: Vec<Accessory>,
    pub actions: Vec<RaycastAction>,
}

/// Convert to GPUI component
impl RaycastList {
    pub fn to_gpui(&self, cx: &mut Context) -> impl IntoElement {
        let items = self.children.iter().map(|item| {
            ListItem::new(&item.id)
                .title(&item.title)
                .subtitle(item.subtitle.as_deref())
                .icon(item.icon.as_ref().map(|i| i.to_gpui()))
                .accessories(item.accessories.iter().map(|a| a.to_gpui()))
        });
        
        v_flex()
            .when(self.is_loading.unwrap_or(false), |el| {
                el.child(LoadingIndicator::new())
            })
            .children(items)
    }
}
```

### DO: Implement the Raycast API Shim

**✅ DO**: Create a Node.js package that mimics `@raycast/api`
```typescript
// sidecar/src/api/index.ts

import { sendToHost, receiveFromHost } from '../ipc';

// UI Components
export { List } from './components/List';
export { Grid } from './components/Grid';
export { Detail } from './components/Detail';
export { Form } from './components/Form';
export { Action, ActionPanel } from './components/Action';

// Hooks
export function useNavigation() {
  return {
    push: (component: React.ReactNode) => {
      sendToHost({ type: 'navigation.push', component: serializeComponent(component) });
    },
    pop: () => {
      sendToHost({ type: 'navigation.pop' });
    },
  };
}

// Storage API
export const LocalStorage = {
  async getItem(key: string): Promise<string | undefined> {
    const response = await sendToHost({ type: 'storage.get', key });
    return response.value;
  },
  async setItem(key: string, value: string): Promise<void> {
    await sendToHost({ type: 'storage.set', key, value });
  },
  async removeItem(key: string): Promise<void> {
    await sendToHost({ type: 'storage.remove', key });
  },
};

// Clipboard API
export const Clipboard = {
  async copy(content: string | { html: string }): Promise<void> {
    await sendToHost({ type: 'clipboard.copy', content });
  },
  async paste(): Promise<string> {
    const response = await sendToHost({ type: 'clipboard.paste' });
    return response.content;
  },
};

// Preferences
export function getPreferenceValues<T>(): T {
  // Injected at extension load time
  return (globalThis as any).__RAYCAST_PREFERENCES__ as T;
}

// Environment
export const environment = {
  commandName: (globalThis as any).__RAYCAST_COMMAND__,
  extensionName: (globalThis as any).__RAYCAST_EXTENSION__,
  isDevelopment: process.env.NODE_ENV === 'development',
  // Note: Some macOS-specific fields will be stubbed
  supportPath: process.env.PHOTONCAST_SUPPORT_PATH,
  assetsPath: process.env.PHOTONCAST_ASSETS_PATH,
};
```

### DO: Handle Platform Differences Gracefully

**✅ DO**: Stub macOS-specific APIs with warnings
```typescript
// sidecar/src/api/macos-stubs.ts

import { showToast, Toast } from './toast';

// These APIs don't exist on macOS but are in @raycast/api
export async function runAppleScript(script: string): Promise<string> {
  console.warn('runAppleScript is not available on PhotonCast');
  await showToast({
    style: Toast.Style.Failure,
    title: 'AppleScript not supported',
    message: 'This extension uses macOS-specific features',
  });
  throw new Error('AppleScript is not supported');
}

export const Application = {
  // Stub that returns empty or throws
  async getFrontmostApplication() {
    console.warn('getFrontmostApplication requires macOS APIs');
    return null;
  }
};
```

### DO: Implement Extension Store Integration

**✅ DO**: Fetch and install extensions from Raycast Store
```rust
const RAYCAST_STORE_API: &str = "https://api.raycast.com/v1";

#[derive(Deserialize)]
pub struct StoreExtension {
    pub id: String,
    pub name: String,
    pub title: String,
    pub description: String,
    pub author: Author,
    pub icon_url: String,
    pub download_url: String,
    pub categories: Vec<String>,
    pub commands: Vec<StoreCommand>,
}

pub struct ExtensionStore {
    client: reqwest::Client,
    extensions_dir: PathBuf,
}

impl ExtensionStore {
    pub async fn search(&self, query: &str) -> Result<Vec<StoreExtension>> {
        let url = format!("{}/extensions/search?q={}", RAYCAST_STORE_API, query);
        let response = self.client.get(&url).send().await?;
        let extensions: Vec<StoreExtension> = response.json().await?;
        Ok(extensions)
    }
    
    pub async fn install(&self, extension_id: &str) -> Result<InstalledExtension> {
        // 1. Fetch extension metadata
        let ext = self.get_extension(extension_id).await?;
        
        // 2. Download extension package
        let package_path = self.download_package(&ext.download_url).await?;
        
        // 3. Extract to extensions directory
        let install_path = self.extensions_dir.join(&ext.name);
        extract_tarball(&package_path, &install_path)?;
        
        // 4. Install npm dependencies
        self.install_dependencies(&install_path).await?;
        
        // 5. Build extension
        self.build_extension(&install_path).await?;
        
        // 6. Register in database
        let installed = self.register_extension(&ext, &install_path)?;
        
        Ok(installed)
    }
}
```

## ❌ DON'T

### DON'T: Run Extension Code in Main Process

**❌ DON'T**:
```rust
// DON'T: Running untrusted JS in main process
fn run_extension(code: &str) -> Result<()> {
    let runtime = deno_core::JsRuntime::new(Default::default());
    runtime.execute_script("extension.js", code)?;  // Security risk!
    Ok(())
}
```

**✅ DO**: Use isolated sidecar process
```rust
// DO: Run in sandboxed subprocess
pub async fn run_extension(extension_id: &str) -> Result<ExtensionProcess> {
    let sidecar_path = get_sidecar_path();
    
    let child = Command::new(&sidecar_path)
        .arg("--extension")
        .arg(extension_id)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        // Sandbox options
        .env_clear()
        .env("NODE_ENV", "production")
        .env("PHOTONCAST_EXTENSION_ID", extension_id)
        .spawn()?;
    
    Ok(ExtensionProcess::new(child))
}
```

### DON'T: Assume All Extensions Will Work

**❌ DON'T**: Promise 100% compatibility

**✅ DO**: Clearly document limitations
```rust
/// Checks if an extension is compatible with PhotonCast
pub fn check_compatibility(manifest: &ExtensionManifest) -> CompatibilityReport {
    let mut issues = Vec::new();
    
    // Check for macOS-specific features
    if manifest.uses_applescript() {
        issues.push(CompatibilityIssue::new(
            Severity::Breaking,
            "Uses AppleScript which is not available on this platform",
        ));
    }
    
    // Check for menu bar commands (may have limited support)
    if manifest.commands.iter().any(|c| c.mode == "menu-bar") {
        issues.push(CompatibilityIssue::new(
            Severity::Warning,
            "Menu bar commands have limited support",
        ));
    }
    
    // Check for native binaries
    if manifest.has_native_binaries() {
        issues.push(CompatibilityIssue::new(
            Severity::Breaking,
            "Contains native macOS binaries",
        ));
    }
    
    CompatibilityReport { issues }
}
```

### DON'T: Block UI on Extension Operations

**❌ DON'T**:
```rust
fn load_extension_sync(id: &str) -> Extension {
    // Blocks UI thread!
    std::thread::sleep(Duration::from_secs(2));
    load_from_disk(id)
}
```

**✅ DO**: Load asynchronously with loading states
```rust
async fn load_extension(&self, id: &str, cx: &mut Context) -> Result<()> {
    // Show loading state immediately
    self.loading_extensions.insert(id.to_string());
    cx.notify();
    
    // Load in background
    let extension = tokio::task::spawn_blocking({
        let id = id.to_string();
        move || load_from_disk(&id)
    }).await??;
    
    // Update state
    self.loaded_extensions.insert(id.to_string(), extension);
    self.loading_extensions.remove(id);
    cx.notify();
    
    Ok(())
}
```

## Extension Protocol

### IPC Message Types

```typescript
// Host -> Sidecar
interface HostMessage {
  id: string;
  type: 
    | 'extension.load'
    | 'extension.run'
    | 'extension.action'
    | 'search.update'
    | 'lifecycle.unload';
  payload: unknown;
}

// Sidecar -> Host
interface SidecarMessage {
  id: string;
  type:
    | 'ui.render'
    | 'ui.loading'
    | 'toast.show'
    | 'navigation.push'
    | 'navigation.pop'
    | 'clipboard.copy'
    | 'open.url'
    | 'error';
  payload: unknown;
}

// UI Render payload
interface UIRenderPayload {
  component: 'List' | 'Grid' | 'Detail' | 'Form';
  props: Record<string, unknown>;
  children: UINode[];
}

interface UINode {
  type: string;
  props: Record<string, unknown>;
  children?: UINode[];
}
```

## Compatibility Matrix

| Raycast Feature | Support Level | Notes |
|-----------------|---------------|-------|
| List component | ✅ Full | Core feature |
| Grid component | ✅ Full | Core feature |
| Detail component | ✅ Full | Markdown support |
| Form component | ✅ Full | All input types |
| Action/ActionPanel | ✅ Full | Core feature |
| LocalStorage | ✅ Full | SQLite backend |
| Clipboard | ✅ Full | Native integration |
| Preferences | ✅ Full | UI for configuration |
| showToast | ✅ Full | Native notifications |
| useNavigation | ✅ Full | GPUI navigation |
| useCachedPromise | ✅ Full | React hook |
| OAuth | ⚠️ Partial | Basic flows work |
| Menu Bar | ⚠️ Partial | System tray fallback |
| runAppleScript | ❌ None | macOS only |
| Application API | ⚠️ Partial | Limited to installed apps |
| System Utilities | ⚠️ Partial | Platform-specific |

## Native Extension API

For extensions that need deep system integration, PhotonCast supports native Rust extensions:

```rust
use photoncast_extension_api::prelude::*;

#[extension]
pub struct MyNativeExtension;

#[extension_commands]
impl MyNativeExtension {
    #[command(title = "My Command", mode = "view")]
    async fn my_command(&self, cx: ExtensionContext) -> Result<impl View> {
        let items = self.fetch_items().await?;
        
        Ok(List::new()
            .children(items.into_iter().map(|item| {
                ListItem::new(&item.id)
                    .title(&item.name)
                    .action(Action::new("Open", || {
                        cx.open_url(&item.url)
                    }))
            })))
    }
}
```

## Resources

- [Raycast API Documentation](https://developers.raycast.com/)
- [Raycast Extension Examples](https://github.com/raycast/extensions)
- [Flare (Raycast for Linux)](https://github.com/ByteAtATime/flare) - Reference implementation
- [Node.js Child Process](https://nodejs.org/api/child_process.html)
