# Tasks: Phase 3a — Raycast Extension Compatibility (Core Sidecar Infrastructure)

> **Spec:** `droidz/specs/2026-01-27-phase-3-raycast-extension-compatibility/spec.md`
> **Created:** 2026-01-27
> **Parallelism:** Groups 1 and 2 have zero file overlap and run in parallel. Groups 3+ are sequential.

---

## Dependency Graph

```
Group 1 (IPC Crate) ──────────┐
                               ├──► Group 3 (Bridge Crate) ──► Group 4 (Integration) ──► Group 5 (Dev CLI) ──► Group 6 (E2E Verification)
Group 2 (Node.js Package) ────┘
```

- **Parallel:** Group 1 + Group 2 (zero file overlap)
- **Sequential:** Group 3 depends on Group 1. Group 4 depends on Groups 1–3. Group 5 depends on Group 4. Group 6 depends on all.

---

## Group 1: `photoncast-extension-ipc` Crate (Rust IPC Protocol)

> **Depends on:** Nothing (can start immediately)
> **Parallel with:** Group 2
> **Files:** `crates/photoncast-extension-ipc/**`

This group creates the reusable IPC protocol crate for JSON-RPC 2.0 over stdio communication with Node.js sidecar processes.

### Task 1.1: Scaffold the `photoncast-extension-ipc` crate
- **Files to create:**
  - `crates/photoncast-extension-ipc/Cargo.toml`
  - `crates/photoncast-extension-ipc/src/lib.rs`
- **What to implement:**
  - Create `Cargo.toml` with dependencies: `serde`, `serde_json`, `tokio` (process, io-util, sync, time features), `tokio-util` (codec feature), `json-patch` (version 3.0), `thiserror` (version 2.0), `tracing`
  - Use `workspace.package` inheritance for version, edition, rust-version, license, authors, repository
  - Use `workspace.dependencies` for `serde`, `serde_json`, `tokio`, `thiserror`, `tracing` — add `tokio-util` and `json-patch` to workspace deps if not present
  - `lib.rs` re-exports all public modules: `protocol`, `transport`, `process`, `error`
- **Acceptance criteria:**
  - `cargo check -p photoncast-extension-ipc` passes
  - Crate is a member of the workspace (auto-discovered via `crates/*` glob)
- **Complexity:** S

### Task 1.2: Implement `error.rs` — IPC error types
- **Files to create:**
  - `crates/photoncast-extension-ipc/src/error.rs`
- **What to implement:**
  - `IpcError` enum using `thiserror`:
    - `SpawnFailed(String)` — process failed to start
    - `TransportClosed` — stdin/stdout closed unexpectedly
    - `SerializationError(serde_json::Error)` — JSON serialization failure
    - `Timeout { duration: Duration }` — operation timed out
    - `ProcessExited { code: Option<i32> }` — sidecar exited unexpectedly
    - `RpcError { code: i32, message: String, data: Option<String> }` — JSON-RPC error response
    - `Io(std::io::Error)` — underlying I/O error
  - `From` impls for `std::io::Error`, `serde_json::Error`
- **Acceptance criteria:**
  - All error variants have descriptive `Display` output
  - `From` conversions compile and work correctly
  - Unit tests verify error formatting
- **Complexity:** S

### Task 1.3: Implement `protocol.rs` — JSON-RPC 2.0 message types
- **Files to create:**
  - `crates/photoncast-extension-ipc/src/protocol.rs`
- **What to implement:**
  - `RpcRequest` struct: `jsonrpc: String`, `id: u64`, `method: String`, `params: serde_json::Value`
  - `RpcResponse` struct: `jsonrpc: String`, `id: u64`, `result: Option<serde_json::Value>`, `error: Option<RpcErrorData>`
  - `RpcErrorData` struct: `code: i32`, `message: String`, `data: Option<serde_json::Value>`
  - `RpcNotification` struct (no `id`): `jsonrpc: String`, `method: String`, `params: serde_json::Value`
  - `RpcMessage` enum: `Request(RpcRequest)`, `Response(RpcResponse)`, `Notification(RpcNotification)`
  - Standard error codes as constants: `PARSE_ERROR = -32700`, `INVALID_REQUEST = -32600`, `METHOD_NOT_FOUND = -32601`, `INVALID_PARAMS = -32602`, `INTERNAL_ERROR = -32603`, `EXTENSION_ERROR = -1`
  - Builder methods on `RpcRequest::new(id, method, params)` and `RpcResponse::success(id, result)`, `RpcResponse::error(id, code, message)`
  - `RpcMessage::parse(line: &str) -> Result<RpcMessage, IpcError>` to parse incoming JSON lines
  - All types derive `Serialize`, `Deserialize`, `Debug`, `Clone`
- **Acceptance criteria:**
  - Round-trip serialization/deserialization tests pass for all message types
  - Parsing matches the exact JSON-RPC 2.0 format from spec sections 3.2–3.4
  - Error code constants match spec section 3.4 table
- **Complexity:** M

### Task 1.4: Implement `transport.rs` — stdio transport layer
- **Files to create:**
  - `crates/photoncast-extension-ipc/src/transport.rs`
- **What to implement:**
  - `StdioTransport` struct wrapping:
    - `writer: FramedWrite<ChildStdin, LinesCodec>` (from `tokio-util`)
    - `reader: FramedRead<ChildStdout, LinesCodec>` (from `tokio-util`)
  - `StdioTransport::new(stdin: ChildStdin, stdout: ChildStdout) -> Self`
  - `async fn send(&mut self, message: &RpcMessage) -> Result<(), IpcError>` — serialize to JSON + newline, write to stdin
  - `async fn receive(&mut self) -> Result<RpcMessage, IpcError>` — read line from stdout, parse as `RpcMessage`
  - `async fn send_request(&mut self, request: &RpcRequest) -> Result<(), IpcError>` — convenience
  - `async fn send_response(&mut self, response: &RpcResponse) -> Result<(), IpcError>` — convenience
  - Use `tokio-util`'s `LinesCodec` for newline-delimited framing
  - Log all sent/received messages at `tracing::debug` level
- **Acceptance criteria:**
  - Unit tests with mock stdin/stdout (use `tokio::io::duplex`) verify send/receive round-trip
  - Handles partial JSON lines gracefully (returns error, doesn't panic)
  - Handles empty lines and whitespace-only lines
- **Complexity:** M

### Task 1.5: Implement `process.rs` — sidecar process lifecycle management
- **Files to create:**
  - `crates/photoncast-extension-ipc/src/process.rs`
- **What to implement:**
  - `SidecarState` enum: `Dormant`, `Starting`, `Ready`, `Active`, `Stopping`, `Stopped`, `Crashed { error: String }`, `Failed { reason: String }` — all derive `Debug, Clone, PartialEq`
  - `SidecarConfig` struct:
    - `node_path: PathBuf` — path to Node.js binary
    - `bootstrap_path: PathBuf` — path to bootstrap.js
    - `extension_dir: PathBuf` — extension install directory
    - `support_dir: PathBuf` — extension data directory
    - `idle_timeout: Duration` — default 5 minutes
    - `startup_timeout: Duration` — default 10 seconds
    - `memory_limit_mb: u64` — default 512 MB
    - `max_crash_retries: u32` — default 3
    - `dev_mode: bool`
  - `SidecarProcess` struct:
    - `child: tokio::process::Child`
    - `transport: StdioTransport`
    - `stderr_handle: tokio::task::JoinHandle<String>` — captures stderr
    - `state: SidecarState`
    - `last_activity: Instant`
    - `crash_count: u32`
    - `pending_requests: HashMap<u64, tokio::sync::oneshot::Sender<Result<serde_json::Value, IpcError>>>`
    - `next_request_id: u64`
  - `SidecarProcess::spawn(config: &SidecarConfig) -> Result<Self, IpcError>`:
    - Spawns Node.js with `env_clear()`, whitelisted env vars only: `NODE_ENV`, `PHOTONCAST_EXTENSION_DIR`, `PHOTONCAST_SUPPORT_DIR`, `PHOTONCAST_DEV_MODE`, `HOME`, `PATH`
    - Sets `kill_on_drop(true)`
    - Pipes stdin/stdout/stderr
    - Spawns stderr reader task
    - Waits for `lifecycle.ready` notification with startup_timeout
    - Transitions state: Dormant → Starting → Ready
  - `async fn send_request(&mut self, method: &str, params: serde_json::Value) -> Result<serde_json::Value, IpcError>`:
    - Assigns request ID, sends via transport, waits for matching response
    - Updates `last_activity`
    - Times out after 30 seconds
  - `async fn handle_incoming(&mut self) -> Result<RpcMessage, IpcError>`:
    - Reads next message from transport
    - Routes responses to pending request senders
    - Returns requests/notifications for caller to handle
  - `async fn shutdown(&mut self) -> Result<(), IpcError>`:
    - Sends `lifecycle.unload` request
    - Waits 5 seconds for graceful exit
    - SIGTERM if still running, SIGKILL after 5 more seconds
    - Transitions to Stopped
  - `fn is_idle(&self, timeout: Duration) -> bool` — checks last_activity
  - `fn state(&self) -> &SidecarState`
  - `fn crash_count(&self) -> u32`
  - `fn increment_crash_count(&mut self)` + check against max retries
- **Acceptance criteria:**
  - Spawn correctly creates child process with sanitized environment
  - `kill_on_drop(true)` is set
  - Ready state detection works (parses `lifecycle.ready` notification)
  - Startup timeout works (fails if ready not received in time)
  - Graceful shutdown sends unload and waits
  - State transitions are correct
  - Unit tests verify state machine transitions (can mock the subprocess with a simple echo script)
- **Complexity:** XL

### Task 1.6: Unit tests for the IPC crate
- **Files to create/modify:**
  - Tests within each source file (`#[cfg(test)] mod tests`)
  - Optionally: `crates/photoncast-extension-ipc/tests/` integration tests
- **What to implement:**
  - `protocol.rs` tests:
    - Serialize/deserialize `RpcRequest`, `RpcResponse`, `RpcNotification`
    - Parse valid JSON-RPC messages
    - Parse invalid JSON returns appropriate error
    - Builder methods produce correct JSON
  - `transport.rs` tests:
    - Send and receive with mock stdio (use `tokio::io::duplex`)
    - Handle malformed JSON line
    - Handle EOF (transport closed)
  - `process.rs` tests:
    - State machine transitions (unit test with manual state changes)
    - Config defaults are correct
    - `is_idle` works correctly with different timeouts
  - `error.rs` tests:
    - Error display messages
    - From conversions
- **Acceptance criteria:**
  - `cargo test -p photoncast-extension-ipc` passes with all tests green
  - All public API functions have at least one test
  - `cargo clippy -p photoncast-extension-ipc` passes with no warnings
- **Complexity:** M

---

## Group 2: `packages/raycast-compat` Node.js Package

> **Depends on:** Nothing (can start immediately)
> **Parallel with:** Group 1
> **Files:** `packages/raycast-compat/**`

This group creates the Node.js shim package that replaces `@raycast/api` at runtime, providing a custom React reconciler and host service proxies over JSON-RPC.

### Task 2.1: Scaffold the `raycast-compat` npm package
- **Files to create:**
  - `packages/raycast-compat/package.json`
  - `packages/raycast-compat/tsconfig.json`
  - `packages/raycast-compat/.gitignore` (ignore `node_modules/`, `dist/`)
- **What to implement:**
  - `package.json`:
    - `name`: `@photoncast/raycast-compat`
    - `version`: `0.1.0`
    - `private`: true
    - `main`: `dist/index.js`
    - `scripts`: `build` (esbuild or tsc), `typecheck` (tsc --noEmit)
    - `dependencies`: `react` (^18.2.0), `react-reconciler` (^0.29.0)
    - `devDependencies`: `typescript` (^5.3.0), `esbuild` (^0.19.0), `@types/react` (^18.2.0), `@raycast/api` (for type reference only)
    - Note: `@raycast/utils` listed as dependency per spec
  - `tsconfig.json`: target ES2020, module commonjs, jsx react-jsx, strict true, outDir dist, rootDir src
  - `.gitignore`: `node_modules/`, `dist/`
- **Acceptance criteria:**
  - `cd packages/raycast-compat && npm install` succeeds
  - `npm run typecheck` runs (may have errors until code is written)
  - Package structure matches spec section 2.2
- **Complexity:** S

### Task 2.2: Implement `src/ipc.ts` — JSON-RPC client (stdio)
- **Files to create:**
  - `packages/raycast-compat/src/ipc.ts`
- **What to implement:**
  - `IpcClient` class:
    - Reads lines from `process.stdin` using `readline`
    - Writes JSON + newline to `process.stdout`
    - `call(method: string, params: Record<string, unknown>): Promise<unknown>` — sends request, waits for response by matching `id`
    - `notify(method: string, params: Record<string, unknown>): void` — sends notification (no id, no response)
    - Tracks pending requests with `Map<number, { resolve, reject }>`
    - Auto-increments request IDs
    - Routes incoming messages: responses go to pending map, requests trigger registered handlers
    - `onRequest(handler: (method: string, params: unknown) => Promise<unknown>): void` — registers handler for host→sidecar requests
  - Export singleton `ipc` instance
  - Set up `globalThis.__ipc_call__` for bootstrap compatibility
- **Acceptance criteria:**
  - `call()` correctly sends JSON-RPC request and resolves with response
  - Handles concurrent requests with different IDs
  - Rejects pending requests on error responses
  - Handles malformed JSON gracefully
  - Unit tests (can run with mocked stdio)
- **Complexity:** M

### Task 2.3: Implement `src/callback-registry.ts` — action callback management
- **Files to create:**
  - `packages/raycast-compat/src/callback-registry.ts`
- **What to implement:**
  - `registerCallback(fn: () => void | Promise<void>): string` — returns `cb_N` ID
  - `executeCallback(id: string): Promise<void>` — runs callback by ID, throws if not found
  - `clearCallbacks(): void` — resets registry (called on each command.run)
  - Auto-incrementing `nextId` counter
  - `Map<string, CallbackFn>` storage
- **Acceptance criteria:**
  - Register returns unique IDs
  - Execute calls the correct function
  - Execute throws for unknown IDs
  - Clear removes all callbacks and resets counter
  - Unit tests pass
- **Complexity:** S

### Task 2.4: Implement `src/types.ts` — shared TypeScript types
- **Files to create:**
  - `packages/raycast-compat/src/types.ts`
- **What to implement:**
  - `ViewNode` interface: `type: string`, `props: Record<string, unknown>`, `children: ViewNode[]`
  - `SerializedView` type matching the JSON schema from spec section 3.2 (ui.render params)
  - `ListViewData`, `DetailViewData` interfaces matching IPC protocol
  - `ListSectionData`, `ListItemData`, `ActionData` interfaces
  - `IconData` type (builtin, file, url variants)
  - `AccessoryData` type
  - `ToastStyle` enum: `Success`, `Failure`, `Animated`
  - `Toast` interface
  - `Environment` interface
  - Re-export key Raycast API types that extensions expect
- **Acceptance criteria:**
  - Types compile cleanly with strict TypeScript
  - Types match the IPC protocol JSON schemas from spec sections 3.2
- **Complexity:** M

### Task 2.5: Implement `src/reconciler.ts` — custom React reconciler
- **Files to create:**
  - `packages/raycast-compat/src/reconciler.ts`
- **What to implement:**
  - Custom React reconciler using `react-reconciler` package
  - `ViewNode` tree construction:
    - `createInstance(type, props)` → `ViewNode`
    - `appendInitialChild(parent, child)` — add child to parent
    - `appendChild(parent, child)` — add child, schedule flush
    - `removeChild(parent, child)` — remove child, schedule flush
    - `commitUpdate(instance, type, oldProps, newProps)` — update props, schedule flush
  - `supportsMutation: true`, `supportsPersistence: false`
  - `scheduleFlush()` — debounce with `queueMicrotask`, sends `ui.render` via IPC when tree changes
  - `serializeTree(node: ViewNode)` — converts ViewNode tree to JSON matching the IPC protocol schema:
    - Maps `List` → `{ type: "List", isLoading, searchBarPlaceholder, sections: [...] }`
    - Maps `List.Item` → items within sections
    - Maps `List.Section` → sections array
    - Maps `Detail` → `{ type: "Detail", markdown, metadata: [...] }`
    - Maps `ActionPanel` → actions array on items
    - Maps `Action` → action entries with callback IDs
  - `render(element: React.ReactElement): void` — creates container, renders into it
  - `filterProps(props)` — strips React internals (children, key, ref)
  - Export `render` function and `currentTree` accessor
- **Acceptance criteria:**
  - React components render to correct ViewNode tree
  - Tree changes trigger `ui.render` IPC call (debounced per microtask)
  - Serialized output matches the JSON format from spec section 3.2
  - Handles re-renders (updates, not full re-create)
  - Unit tests verify tree construction and serialization for List and Detail views
- **Complexity:** XL

### Task 2.6: Implement `src/components/List.tsx` — List component
- **Files to create:**
  - `packages/raycast-compat/src/components/List.tsx`
- **What to implement:**
  - `List` function component: accepts `isLoading`, `searchBarPlaceholder`, `onSearchTextChange`, `navigationTitle`, `filtering`, `children`
  - `List.Item` sub-component: `id`, `title`, `subtitle`, `icon`, `accessories`, `actions`, `detail`, `keywords`
  - `List.Section` sub-component: `title`, `subtitle`, `children`
  - `List.EmptyView` sub-component: `title`, `description`, `icon`, `actions`
  - Register `onSearchTextChange` as callback via callback-registry
  - Serialize icon prop using `serializeIcon()` helper
  - Serialize accessories using `serializeAccessory()` helper
- **Acceptance criteria:**
  - `<List>` renders to `{ type: "List", ... }` JSON
  - `<List.Item>` renders with all props correctly mapped
  - `<List.Section>` groups items correctly
  - `<List.EmptyView>` renders when no items
  - `onSearchTextChange` callback is registered and invocable
- **Complexity:** M

### Task 2.7: Implement `src/components/Detail.tsx` — Detail component
- **Files to create:**
  - `packages/raycast-compat/src/components/Detail.tsx`
- **What to implement:**
  - `Detail` function component: `markdown`, `navigationTitle`, `isLoading`, `actions`, `metadata`
  - `Detail.Metadata` sub-component: children are metadata items
  - `Detail.Metadata.Label` sub-component: `title`, `text`, `icon`
  - `Detail.Metadata.Link` sub-component: `title`, `target`, `text`
  - `Detail.Metadata.TagList` sub-component: `title`, children are tags
  - `Detail.Metadata.Separator` sub-component
- **Acceptance criteria:**
  - `<Detail markdown="# Hello" />` renders to `{ type: "Detail", markdown: "# Hello", ... }`
  - Metadata items serialize correctly
  - Actions on Detail view work
- **Complexity:** M

### Task 2.8: Implement `src/components/ActionPanel.tsx` and `src/components/Action.tsx`
- **Files to create:**
  - `packages/raycast-compat/src/components/ActionPanel.tsx`
  - `packages/raycast-compat/src/components/Action.tsx`
- **What to implement:**
  - `ActionPanel` component: wraps children as actions array
  - `ActionPanel.Section` sub-component: groups actions with title
  - `ActionPanel.Submenu` sub-component: nested action panel
  - `Action` component: `title`, `icon`, `shortcut`, `onAction`, `style`
    - `onAction` callback registered via callback-registry → `{ id: "cb_N", title, type: "callback" }`
  - `Action.CopyToClipboard`: `content`, `title` → `{ type: "copyToClipboard", content }`
  - `Action.OpenInBrowser`: `url`, `title` → `{ type: "openUrl", url }`
  - `Action.Push`: `title`, `target` (React element) → `{ type: "callback" }` (renders pushed view)
  - `Action.ShowInFinder`: `path`, `title` → `{ type: "revealInFinder", path }`
  - Shortcut serialization: `{ modifiers: ["cmd"], key: "c" }`
- **Acceptance criteria:**
  - ActionPanel renders as `actions` array on parent item
  - Callback-based actions get registered IDs
  - Built-in action types (CopyToClipboard, OpenInBrowser) serialize correctly
  - Shortcuts serialize to the correct format
- **Complexity:** L

### Task 2.9: Implement `src/services/` — host service proxies
- **Files to create:**
  - `packages/raycast-compat/src/services/toast.ts`
  - `packages/raycast-compat/src/services/clipboard.ts`
  - `packages/raycast-compat/src/services/storage.ts`
  - `packages/raycast-compat/src/services/cache.ts`
  - `packages/raycast-compat/src/services/preferences.ts`
  - `packages/raycast-compat/src/services/environment.ts`
- **What to implement:**
  - **toast.ts**: `showToast(options)` → IPC `toast.show`, `showHUD(title)` → IPC `hud.show`, `Toast` class with `hide()` method, `Toast.Style` enum
  - **clipboard.ts**: `Clipboard.copy(content)` → IPC `clipboard.copy`, `Clipboard.read()` → IPC `clipboard.read`, `Clipboard.paste()` → IPC `clipboard.read`
  - **storage.ts**: `LocalStorage.getItem(key)` → IPC `storage.get`, `.setItem(key, value)` → IPC `storage.set`, `.removeItem(key)` → IPC `storage.remove`, `.allItems()` → IPC `storage.allItems`, `.clear()` → IPC `storage.clear`
  - **cache.ts**: `Cache` class wrapping `LocalStorage` with TTL support (in-memory first, persist via storage IPC)
  - **preferences.ts**: `getPreferenceValues<T>()` reads from `globalThis.__PHOTONCAST_PREFERENCES__`
  - **environment.ts**: `environment` object with getters: `commandName`, `extensionName`, `isDevelopment`, `appearance`, `supportPath`, `assetsPath`, `commandMode`, `launchType`, `textSize`, `raycastVersion` (stub as "1.50.0"). Reads from `globalThis.__PHOTONCAST_*` and `process.env.PHOTONCAST_*`
- **Acceptance criteria:**
  - Each service function calls the correct IPC method with correct params
  - Return types match `@raycast/api` TypeScript types
  - `showToast` returns a `Toast` object as per Raycast API
  - `environment` getters return correct values from injected globals
  - `getPreferenceValues()` returns injected preferences
- **Complexity:** L

### Task 2.10: Implement `src/hooks/` — navigation and re-exported hooks
- **Files to create:**
  - `packages/raycast-compat/src/hooks/useNavigation.ts`
  - `packages/raycast-compat/src/hooks/index.ts`
- **What to implement:**
  - `useNavigation()` hook returning `{ push, pop }`:
    - `push(component: React.ReactElement)` → renders component, sends `navigation.push` IPC
    - `pop()` → sends `navigation.pop` IPC
  - `index.ts`: re-export `@raycast/utils` hooks (useCachedPromise, useExec, useFetch, etc.)
  - `open(target: string)` utility → IPC `open.url` or `open.file`
  - `closeMainWindow(options?)` utility → IPC `window.close`
- **Acceptance criteria:**
  - `useNavigation().push()` triggers navigation.push IPC call
  - `useNavigation().pop()` triggers navigation.pop IPC call
  - Re-exported hooks from `@raycast/utils` are accessible
- **Complexity:** M

### Task 2.11: Implement `src/index.ts` — main entry point and `bootstrap.js`
- **Files to create:**
  - `packages/raycast-compat/src/index.ts`
  - `packages/raycast-compat/bootstrap.js`
- **What to implement:**
  - **`src/index.ts`**: re-exports everything that `@raycast/api` exports:
    - Components: `List`, `Detail`, `ActionPanel`, `Action`, `Icon`, `Color`, `Image`, `Keyboard`
    - Services: `showToast`, `showHUD`, `Clipboard`, `LocalStorage`, `Cache`, `getPreferenceValues`, `environment`
    - Hooks: `useNavigation`, re-exports from `@raycast/utils`
    - Utilities: `open`, `closeMainWindow`
    - Types: all TypeScript types that extensions import
  - **`bootstrap.js`**: standalone entry point loaded by Node.js sidecar:
    - Sets up `readline` interface on stdin
    - JSON-RPC message routing (responses → pending map, requests → handler)
    - `handleHostRequest(msg)` switch on method:
      - `command.run` → clearCallbacks, set globals, require + render command module
      - `action.execute` → executeCallback(callbackId)
      - `search.update` → trigger `__onSearchTextChange__` global
      - `selection.change` → trigger selection callback
      - `lifecycle.unload` → respond OK, process.exit(0)
    - `sendResult(id, result)` and `sendError(id, code, message, data)` helpers
    - `runCommand(params)`:
      - Inject `globalThis.__PHOTONCAST_COMMAND__`, `__PHOTONCAST_EXTENSION__`, `__PHOTONCAST_PREFERENCES__`
      - Set `process.env.PHOTONCAST_*` variables
      - Require the command module from `dist/<command>.js`
      - Call `render(React.createElement(Command))`
    - Signal ready: write `lifecycle.ready` notification on startup
    - Export `__ipc_call__` on globalThis for service proxies
- **Acceptance criteria:**
  - `import { List, showToast, environment } from '@photoncast/raycast-compat'` works
  - Bootstrap script signals ready on startup
  - Bootstrap correctly routes incoming JSON-RPC messages
  - `command.run` loads and renders extension commands
  - `action.execute` finds and runs registered callbacks
  - `lifecycle.unload` exits cleanly
- **Complexity:** L

### Task 2.12: Build setup and pre-built bundle
- **Files to create/modify:**
  - `packages/raycast-compat/package.json` (add build scripts)
  - Build output: `packages/raycast-compat/dist/`
- **What to implement:**
  - Add `build` script to package.json using esbuild:
    - Bundle `src/index.ts` → `dist/index.js` (CJS format, node platform, target node18)
    - Bundle `src/reconciler.ts` → `dist/reconciler.js`
    - Bundle `src/callback-registry.ts` → `dist/callback-registry.js`
    - External: `react`, `react-reconciler` (bundled separately or included)
  - Add `typecheck` script: `tsc --noEmit`
  - Ensure `bootstrap.js` references `dist/` correctly
  - Verify the bundle works by requiring it in a Node.js test
- **Acceptance criteria:**
  - `npm run build` produces `dist/` with all required files
  - `npm run typecheck` passes with zero errors
  - Bundle size is under 500KB (excluding react)
  - `node -e "require('./dist/index.js')"` doesn't crash
- **Complexity:** M

### Task 2.13: Unit tests for the Node.js package
- **Files to create:**
  - `packages/raycast-compat/src/__tests__/ipc.test.ts`
  - `packages/raycast-compat/src/__tests__/callback-registry.test.ts`
  - `packages/raycast-compat/src/__tests__/reconciler.test.ts`
  - `packages/raycast-compat/src/__tests__/services.test.ts`
- **What to implement:**
  - Test IPC client message formatting and routing
  - Test callback registry register/execute/clear
  - Test reconciler tree construction: `<List><List.Item title="test" /></List>` → correct JSON
  - Test serialization of List, Detail, ActionPanel
  - Test service proxies call correct IPC methods
  - Add test runner config (jest or vitest) to package.json
- **Acceptance criteria:**
  - `npm test` passes all tests
  - Coverage of critical paths: IPC, reconciler serialization, callback registry
- **Complexity:** L

---

## Group 3: `photoncast-raycast-bridge` Crate (Rust Bridge)

> **Depends on:** Group 1 (IPC crate must compile)
> **Files:** `crates/photoncast-raycast-bridge/**`

This group creates the Raycast-specific bridge crate that presents Raycast extensions as native `Extension` trait objects.

### Task 3.1: Scaffold the `photoncast-raycast-bridge` crate
- **Files to create:**
  - `crates/photoncast-raycast-bridge/Cargo.toml`
  - `crates/photoncast-raycast-bridge/src/lib.rs`
- **What to implement:**
  - `Cargo.toml` with dependencies:
    - `photoncast-extension-api = { path = "../photoncast-extension-api" }`
    - `photoncast-extension-ipc = { path = "../photoncast-extension-ipc" }`
    - `photoncast-core = { path = "../photoncast-core" }`
    - Workspace deps: `serde`, `serde_json`, `tokio`, `thiserror`, `tracing`, `nucleo`
  - Use `workspace.package` inheritance
  - `lib.rs` re-exports: `bridge`, `manifest`, `discovery`, `builder`, `host_services`, `permissions`, `error`
- **Acceptance criteria:**
  - `cargo check -p photoncast-raycast-bridge` passes
  - All internal crate dependencies resolve
- **Complexity:** S

### Task 3.2: Implement `error.rs` — bridge error types
- **Files to create:**
  - `crates/photoncast-raycast-bridge/src/error.rs`
- **What to implement:**
  - `BridgeError` enum using `thiserror`:
    - `SpawnFailed(String)`
    - `SidecarCrashed(String)`
    - `Timeout(Duration)`
    - `RpcError { code: i32, message: String }`
    - `UnsupportedViewType(String)`
    - `BuildFailed(String)`
    - `InvalidManifest(String)`
    - `MissingViewType`
    - `Ipc(photoncast_extension_ipc::IpcError)` — `#[from]`
    - `Json(serde_json::Error)` — `#[from]`
    - `Io(std::io::Error)` — `#[from]`
  - Conversion: `impl From<BridgeError> for ExtensionApiError`
- **Acceptance criteria:**
  - All error variants have descriptive messages
  - `BridgeError` converts to `ExtensionApiError` for the Extension trait
- **Complexity:** S

### Task 3.3: Implement `manifest.rs` — Raycast manifest parsing
- **Files to create:**
  - `crates/photoncast-raycast-bridge/src/manifest.rs`
- **What to implement:**
  - `RaycastManifest` struct (Deserialize from package.json):
    - `name: String`, `title: String`, `description: String`, `icon: Option<String>`, `author: String`, `license: Option<String>`, `version: Option<String>`
    - `commands: Vec<RaycastCommand>`
    - `preferences: Option<Vec<RaycastPreference>>`
    - `dependencies: HashMap<String, String>`
    - `dev_dependencies: Option<HashMap<String, String>>`
  - `RaycastCommand` struct: `name`, `title`, `subtitle`, `description`, `mode` (String), `keywords`, `arguments`
  - `RaycastPreference` struct: `name`, `title`, `description`, `required`, `pref_type` (mapped from "type" JSON field), `default`, `placeholder`, `label`, `data` (for dropdowns)
  - `RaycastArgument` struct: `name`, `placeholder`, `arg_type`, `required`
  - `RaycastManifest::from_package_json(path: &Path) -> Result<Self, BridgeError>`:
    - Parse package.json, extract Raycast-specific fields
    - Handle both top-level fields (name, title, description) and nested ones
  - `RaycastManifest::is_raycast_extension(&self) -> bool`:
    - Returns true if `dependencies` contains `@raycast/api`
  - `RaycastManifest::to_extension_metadata(&self) -> ExtensionManifest`:
    - Maps to the ABI-stable `ExtensionManifest` from `photoncast-extension-api`
    - ID format: `com.raycast.<name>`
  - `RaycastManifest::to_extension_commands(&self) -> Vec<ExtensionCommand>`:
    - Maps each `RaycastCommand` to an `ExtensionCommand`
  - `RaycastManifest::icon_path(&self) -> Option<PathBuf>`:
    - Resolves icon relative to extension directory
- **Acceptance criteria:**
  - Parses real Raycast extension package.json files correctly (test with Brew, Kill Process manifests)
  - `is_raycast_extension()` correctly identifies Raycast vs non-Raycast packages
  - Manifest translation produces valid `ExtensionManifest`
  - Handles missing optional fields gracefully
  - Unit tests with sample package.json fixtures
- **Complexity:** L

### Task 3.4: Implement `discovery.rs` — extension discovery
- **Files to create:**
  - `crates/photoncast-raycast-bridge/src/discovery.rs`
- **What to implement:**
  - `discover_raycast_extensions(extensions_dir: &Path) -> Vec<(PathBuf, RaycastManifest)>`:
    - Scans directory for subdirectories containing `package.json`
    - Filters to only Raycast extensions (`is_raycast_extension()`)
    - Returns extension path + parsed manifest
    - Logs warnings for parse failures, doesn't fail on single extension
  - `is_raycast_extension_dir(dir: &Path) -> bool`:
    - Checks for `package.json` with `@raycast/api` dependency
  - Consider the extensions directory location: `~/Library/Application Support/PhotonCast/extensions/raycast/`
- **Acceptance criteria:**
  - Discovers extensions in a test directory
  - Skips non-Raycast packages (e.g., regular npm packages)
  - Handles empty directories, missing package.json
  - Logs warnings for malformed manifests without crashing
- **Complexity:** M

### Task 3.5: Implement `builder.rs` — esbuild bundling pipeline
- **Files to create:**
  - `crates/photoncast-raycast-bridge/src/builder.rs`
- **What to implement:**
  - `ExtensionBuilder` struct:
    - `esbuild_path: PathBuf` — path to bundled esbuild binary
    - `node_path: PathBuf` — path to bundled Node.js
  - `ExtensionBuilder::new(esbuild_path, node_path) -> Self`
  - `async fn build(&self, extension_dir: &Path) -> Result<BuildResult, BridgeError>`:
    - Check if `dist/` exists and is fresh (compare mtime to `src/`)
    - If fresh, return `BuildResult::PreBuilt(dist_path)`
    - Otherwise:
      1. Run `npm install` if `node_modules/` missing (via `self.node_path` running npm)
      2. Run esbuild with args: `src/*.tsx src/*.ts --bundle --outdir=dist --format=cjs --platform=node --target=node18 --external:@raycast/api --external:react --external:react-reconciler`
      3. Return `BuildResult::Built(dist_path)` or `BuildResult::BuildFailed(stderr)`
  - `BuildResult` enum: `PreBuilt(PathBuf)`, `Built(PathBuf)`, `NoBuildNeeded`
  - `fn is_bundle_fresh(&self, dist: &Path, src: &Path) -> bool`:
    - Compare modification times
  - `async fn npm_install(&self, dir: &Path) -> Result<(), BridgeError>`:
    - Runs npm install using bundled Node.js
- **Acceptance criteria:**
  - Pre-built extensions are detected and not rebuilt
  - esbuild is invoked with correct args
  - npm install runs when node_modules is missing
  - Build errors are captured and returned as `BridgeError::BuildFailed`
  - Unit tests verify build result detection logic
- **Complexity:** L

### Task 3.6: Implement `host_services.rs` — JSON-RPC to host protocol mapping
- **Files to create:**
  - `crates/photoncast-raycast-bridge/src/host_services.rs`
- **What to implement:**
  - `fn handle_sidecar_request(method: &str, params: &serde_json::Value, host: &ExtensionHost) -> Result<serde_json::Value, BridgeError>`:
    - Switch on method:
      - `ui.render` → parse JSON view, call `host.render_view()`, return `{ viewHandle: "vh_N" }`
      - `ui.patch` → apply JSON Patch (RFC 6902) to current view, call `host.update_view()`
      - `toast.show` → parse params, call `host.show_toast(Toast { style, title, message })`
      - `hud.show` → call `host.show_hud(params.title)`
      - `clipboard.copy` → call `host.copy_to_clipboard(content)`
      - `clipboard.read` → call `host.read_clipboard()`
      - `storage.get` → call storage get
      - `storage.set` → call storage set
      - `storage.remove` → call storage delete
      - `storage.allItems` → call storage list
      - `storage.clear` → clear all storage
      - `open.url` → call `host.open_url(url)`
      - `open.file` → call `host.open_file(path)`
      - `window.close` → hide launcher window
      - `navigation.push` → parse JSON view, call `host.render_view()` (push context)
      - `navigation.pop` → pop navigation stack
      - Unknown → return RPC error -32601
  - `fn json_to_extension_view(json: &serde_json::Value) -> Result<ExtensionView, BridgeError>`:
    - Parse `type` field: `"List"` → `ExtensionView::List(parse_list_view(json))`, `"Detail"` → `ExtensionView::Detail(parse_detail_view(json))`
    - Unsupported types return `BridgeError::UnsupportedViewType`
  - `fn parse_list_view(json: &serde_json::Value) -> Result<ListView, BridgeError>`:
    - Map JSON sections → `ListSection` with `ListItem` entries
    - Map JSON actions → `Action` entries (map callback type to `ActionHandler::Callback`, openUrl to `ActionHandler::OpenUrl`, etc.)
    - Map JSON icons → `IconSource`
    - Map JSON accessories → `Accessory`
  - `fn parse_detail_view(json: &serde_json::Value) -> Result<DetailView, BridgeError>`:
    - Map markdown, metadata items, actions
- **Acceptance criteria:**
  - All 16 JSON-RPC methods from spec section 7.1 are handled
  - JSON view → `ExtensionView` conversion produces valid views
  - List items with actions, icons, accessories all map correctly
  - Toast/HUD/clipboard calls reach the host protocol
  - Unknown methods return proper JSON-RPC error
  - Unit tests with sample JSON payloads from spec
- **Complexity:** XL

### Task 3.7: Implement `permissions.rs` — permission translation
- **Files to create:**
  - `crates/photoncast-raycast-bridge/src/permissions.rs`
- **What to implement:**
  - `fn translate_permissions(manifest: &RaycastManifest) -> Vec<String>`:
    - All Raycast extensions get: `Network`, `Storage`
    - If any command has mode "view": add `UserInterface`
    - If dependencies include `run-applescript`: add `SystemCommands`
    - If preferences include `appPicker` type: add `ApplicationAccess`
    - Add `ShellAccess` by default (most target extensions need it)
  - Return permission strings that match PhotonCast's `Permissions` struct from manifest.rs in photoncast-core
- **Acceptance criteria:**
  - Brew extension gets: Network, Storage, UserInterface, ShellAccess
  - Clean Keyboard gets: Network, Storage, UserInterface
  - Permission strings are valid PhotonCast permission identifiers
- **Complexity:** S

### Task 3.8: Implement `bridge.rs` — RaycastBridge (Extension trait impl)
- **Files to create:**
  - `crates/photoncast-raycast-bridge/src/bridge.rs`
- **What to implement:**
  - `RaycastBridge` struct:
    - `manifest: RaycastManifest`
    - `extension_dir: PathBuf`
    - `sidecar: Option<SidecarProcess>`
    - `config: SidecarConfig`
    - `callback_registry: HashMap<String, ()>` (tracks active callback IDs for the host side)
    - `current_view: Option<serde_json::Value>` (for JSON Patch)
  - `RaycastBridge::new(manifest, extension_dir, config) -> Self`
  - Implement the `Extension` trait (from `photoncast-extension-api`):
    - `manifest()` → `self.manifest.to_extension_metadata()`
    - `activate(ctx)` → no-op (lazy spawn)
    - `deactivate()` → shutdown sidecar if running
    - `on_startup(ctx)` → no-op
    - `search_provider()` → return None (commands are listed directly)
    - `commands()` → map manifest commands to `ExtensionCommand` entries, each with a `CommandHandler` that calls `self.run_raycast_command()`
  - Private method `fn ensure_sidecar(&mut self) -> Result<&mut SidecarProcess, BridgeError>`:
    - Spawn if `self.sidecar.is_none()`
    - Check if existing sidecar is alive, respawn if crashed (up to max retries)
  - Private method `fn run_raycast_command(&mut self, command_id: &str, ctx: ExtensionContext) -> ExtensionApiResult<()>`:
    - Call `ensure_sidecar()`
    - Build `command.run` JSON-RPC request with: command name, preferences from ctx, environment info
    - Send request to sidecar
    - Enter message loop: process sidecar requests (ui.render, toast.show, etc.) via `host_services::handle_sidecar_request`
    - Loop until command completes or errors
  - Private method `fn handle_action_execute(&mut self, callback_id: &str) -> Result<(), BridgeError>`:
    - Send `action.execute` to sidecar with callback ID
  - Private method `fn handle_search_update(&mut self, text: &str) -> Result<(), BridgeError>`:
    - Send `search.update` to sidecar
  - Idle timeout handling: background task or check on each call
- **Acceptance criteria:**
  - `RaycastBridge` implements the `Extension` trait correctly
  - Lazy sidecar spawn works (first command triggers spawn)
  - Command execution sends correct JSON-RPC and processes responses
  - Sidecar crash triggers retry up to max_crash_retries
  - Deactivate cleanly shuts down sidecar
  - `commands()` returns correct list from manifest
  - Unit tests verify manifest mapping and state transitions
- **Complexity:** XL

### Task 3.9: Unit tests for the bridge crate
- **Files to create/modify:**
  - Tests within each source file (`#[cfg(test)] mod tests`)
  - `crates/photoncast-raycast-bridge/tests/` integration tests
- **What to implement:**
  - `manifest.rs` tests:
    - Parse sample Brew extension package.json
    - Parse sample Kill Process package.json
    - Handle missing optional fields
    - `is_raycast_extension()` returns true/false correctly
    - `to_extension_metadata()` produces valid metadata
  - `discovery.rs` tests:
    - Discover extensions in a temp directory with sample packages
    - Skip non-Raycast packages
    - Handle empty directory
  - `host_services.rs` tests:
    - Convert sample `ui.render` JSON to `ExtensionView::List`
    - Convert sample `ui.render` JSON to `ExtensionView::Detail`
    - Handle all host service methods with mock host
    - Unknown method returns error
  - `permissions.rs` tests:
    - Verify permission output for each target extension type
  - `builder.rs` tests:
    - Detect fresh vs stale builds
    - Detect pre-built extensions
  - Include sample package.json fixtures in `tests/fixtures/`
- **Acceptance criteria:**
  - `cargo test -p photoncast-raycast-bridge` passes all tests
  - `cargo clippy -p photoncast-raycast-bridge` passes with no warnings
  - Sample manifests from real Raycast extensions parse correctly
- **Complexity:** L

---

## Group 4: Integration with Existing Extension System

> **Depends on:** Groups 1, 2, and 3
> **Files:** Modifications to existing crates + workspace `Cargo.toml`

This group wires the new Raycast bridge into the existing `ExtensionManager` and discovery system.

### Task 4.1: Add new crates to workspace dependencies
- **Files to modify:**
  - `Cargo.toml` (workspace root)
- **What to implement:**
  - Add `tokio-util = { version = "0.7", features = ["codec"] }` to `[workspace.dependencies]`
  - Add `json-patch = "3.0"` to `[workspace.dependencies]`
  - Verify `crates/*` glob auto-discovers the two new crates
- **Acceptance criteria:**
  - `cargo check --workspace` passes
  - New crates are recognized as workspace members
- **Complexity:** S

### Task 4.2: Add `photoncast-raycast-bridge` dependency to `photoncast-core`
- **Files to modify:**
  - `crates/photoncast-core/Cargo.toml`
- **What to implement:**
  - Add `photoncast-raycast-bridge = { path = "../photoncast-raycast-bridge" }` as an optional dependency behind a `raycast-compat` feature flag
  - Add feature: `raycast-compat = ["dep:photoncast-raycast-bridge"]`
  - Default features should include `raycast-compat`
- **Acceptance criteria:**
  - `cargo check -p photoncast-core --features raycast-compat` passes
  - Feature gate works: `cargo check -p photoncast-core --no-default-features` compiles without bridge
- **Complexity:** S

### Task 4.3: Integrate Raycast discovery into `ExtensionManager`
- **Files to modify:**
  - `crates/photoncast-core/src/extensions/manager.rs`
  - `crates/photoncast-core/src/extensions/config.rs` (if needed for raycast extensions dir)
- **What to implement:**
  - Add Raycast extensions directory to config: `~/Library/Application Support/PhotonCast/extensions/raycast/`
  - In `ExtensionManager::discover()`:
    - After native extension discovery, call `discover_raycast_extensions()` (behind `#[cfg(feature = "raycast-compat")]`)
    - For each discovered Raycast extension, create a `RaycastBridge` and register it
  - Add `HashMap<String, RaycastBridge>` field to `ExtensionManager` (or integrate into `loaded` map)
  - In `search()`: include Raycast extension commands in search results
  - In `launch_command()`: route to `RaycastBridge::run_raycast_command` for Raycast extensions
  - Handle the async nature: `RaycastBridge` uses tokio, the manager may need to block_on() in some places or restructure
- **Acceptance criteria:**
  - Raycast extensions are discovered and appear in search results
  - Selecting a Raycast extension command triggers the sidecar lifecycle
  - Extension manager correctly routes commands to native vs Raycast bridges
  - Feature gate compiles both with and without `raycast-compat`
- **Complexity:** XL

### Task 4.4: Add ActionHandler::Callback support for Raycast actions
- **Files to modify:**
  - `crates/photoncast-extension-api/src/lib.rs` (if needed — `ActionHandler::Callback` already exists but is `#[serde(skip)]`)
  - `crates/photoncast-core/src/extensions/` (action execution path)
- **What to implement:**
  - Ensure `ActionHandler::Callback` variant can carry a callback ID string for Raycast bridges
  - Consider adding `ActionHandler::RaycastCallback { callback_id: RString }` variant (or use a different mechanism)
  - Wire action execution: when user triggers an action on a Raycast extension view, the bridge receives the callback ID and sends `action.execute` to the sidecar
  - Handle async round-trip: action → bridge → IPC → sidecar → callback → IPC → bridge → possible UI update
- **Acceptance criteria:**
  - User clicking an action on a Raycast extension view triggers the correct callback
  - The sidecar receives `action.execute` with the correct callback ID
  - Any UI updates from the callback (re-renders) are applied
- **Complexity:** L

### Task 4.5: Integration tests for the full pipeline
- **Files to create:**
  - `tests/raycast_bridge_integration.rs` (workspace-level integration test)
- **What to implement:**
  - Create a minimal test Raycast extension (in `tests/fixtures/raycast/test-extension/`):
    - `package.json` with `@raycast/api` dependency
    - `src/index.tsx` that renders a `<List>` with items
    - Pre-built `dist/index.js`
  - Test: discover → build → spawn sidecar → run command → receive view → verify view structure
  - Test: action execution round-trip (callback-based action)
  - Test: search text update
  - Test: toast/HUD display
  - Test: sidecar idle timeout
  - Test: sidecar crash recovery
  - Note: these tests require Node.js to be available on the test system
- **Acceptance criteria:**
  - Full round-trip integration test passes
  - Sidecar spawns and communicates correctly
  - View rendering produces valid `ExtensionView`
  - Action callbacks work end-to-end
- **Complexity:** XL

---

## Group 5: Developer CLI (`photoncast dev`)

> **Depends on:** Group 4
> **Files:** CLI entry point modifications

### Task 5.1: Implement `photoncast dev <path>` CLI command
- **Files to modify:**
  - `crates/photoncast/src/main.rs` or CLI entry point (identify actual location)
  - Possibly create `crates/photoncast-core/src/extensions/dev.rs`
- **What to implement:**
  - Parse `dev <path>` CLI subcommand
  - Behavior:
    1. Validate the path contains a valid Raycast extension (`package.json` with `@raycast/api`)
    2. Run `npm install` if `node_modules/` missing
    3. Bundle with esbuild (via `ExtensionBuilder`)
    4. Create `RaycastBridge` with `dev_mode: true`
    5. Register with `ExtensionManager`
    6. Enable source maps (set `PHOTONCAST_DEV_MODE=1`)
    7. Log stderr output to terminal
    8. Print success message with extension name and commands
  - Error handling: clear error messages for common failures (missing package.json, build errors, missing Node.js)
- **Acceptance criteria:**
  - `photoncast dev ~/code/my-extension` loads a local extension
  - Dev mode enables source maps and stderr logging
  - Build errors are displayed clearly
  - Invalid paths produce helpful error messages
- **Complexity:** L

---

## Group 6: End-to-End Verification with Target Extensions

> **Depends on:** All previous groups
> **Files:** Test infrastructure + documentation

This group verifies all 6 target extensions work end-to-end.

### Task 6.1: Set up target extension test infrastructure
- **Files to create:**
  - `tests/e2e/raycast_extensions/README.md`
  - `tests/e2e/raycast_extensions/setup.sh` (downloads/clones target extensions)
- **What to implement:**
  - Script to clone or download the 6 target extensions:
    1. **Brew** (`raycast/extensions/extensions/brew`)
    2. **Kill Process** (`raycast/extensions/extensions/kill-process`)
    3. **System Monitor** (`raycast/extensions/extensions/system-monitor`)
    4. **Set Audio Device** (`raycast/extensions/extensions/set-audio-device`)
    5. **Clean Keyboard** (`raycast/extensions/extensions/clean-keyboard`)
    6. **Home Assistant** (`raycast/extensions/extensions/homeassistant`)
  - Install dependencies for each
  - Build each extension
  - Document any manual setup needed (e.g., Home Assistant requires a server)
- **Acceptance criteria:**
  - All 6 extensions are available for testing
  - Each extension builds successfully
  - Setup script is reproducible
- **Complexity:** M

### Task 6.2: Verify extension: Brew
- **What to verify:**
  - Manifest parses correctly (List, ActionPanel, shell exec, LocalStorage)
  - Extension builds (if from source)
  - First command renders a List view with brew packages
  - Search/filtering works (typing filters the list)
  - Primary action (Install/Upgrade) triggers correctly
  - Clipboard actions work (copy formula name)
  - LocalStorage persists favorites across sessions
- **Acceptance criteria:**
  - ✅ Manifest parses, ✅ Builds, ✅ Renders view, ✅ Actions work, ✅ Search works
- **Complexity:** M

### Task 6.3: Verify extension: Kill Process
- **What to verify:**
  - Manifest parses correctly (List, shell exec via `ps`)
  - Extension renders list of running processes
  - Search filters processes
  - Kill action triggers correctly
  - Process list refreshes
- **Acceptance criteria:**
  - ✅ Manifest parses, ✅ Builds, ✅ Renders view, ✅ Actions work, ✅ Search works
- **Complexity:** S

### Task 6.4: Verify extension: System Monitor
- **What to verify:**
  - Manifest parses correctly (List, Detail, shell exec, polling)
  - Extension renders system stats (CPU, Memory, Disk, Network)
  - Detail view shows expanded information
  - Polling updates values periodically
  - Navigation between list and detail works
- **Acceptance criteria:**
  - ✅ Manifest parses, ✅ Builds, ✅ Renders view, ✅ Detail works, ✅ Polling updates
- **Complexity:** M

### Task 6.5: Verify extension: Set Audio Device
- **What to verify:**
  - Manifest parses correctly (List, ActionPanel, shell exec)
  - Extension renders list of audio devices
  - Select action changes audio device
  - showHUD notification appears
- **Acceptance criteria:**
  - ✅ Manifest parses, ✅ Builds, ✅ Renders view, ✅ Actions work
- **Complexity:** S

### Task 6.6: Verify extension: Clean Keyboard
- **What to verify:**
  - Manifest parses correctly (Detail, Timer, showHUD)
  - Extension renders Detail view with instructions
  - Timer countdown works
  - showHUD displays completion message
- **Acceptance criteria:**
  - ✅ Manifest parses, ✅ Builds, ✅ Renders view, ✅ HUD works
- **Complexity:** S

### Task 6.7: Verify extension: Home Assistant
- **What to verify:**
  - Manifest parses correctly (List, Detail, Form, network requests, Preferences)
  - Preferences UI collects server URL and API token
  - Extension renders entity list (with network request to HA server)
  - Detail view shows entity details
  - Actions trigger HA service calls
  - Note: Requires a Home Assistant instance (or mock server) for full verification
- **Acceptance criteria:**
  - ✅ Manifest parses, ✅ Builds, ✅ Preferences work, ✅ Renders view (with mock or real HA), ✅ Actions work
- **Complexity:** L

### Task 6.8: Performance verification
- **What to verify against spec targets (section 10):**
  - Cold start (first command) < 1s
  - Warm command execution < 200ms
  - UI render (initial) < 100ms
  - UI update (incremental) < 50ms
  - Search text update < 100ms
  - Idle memory per sidecar < 50MB
  - Active memory per sidecar < 150MB
- **What to implement:**
  - Timing instrumentation around key operations
  - Memory measurement using `ps` or process stats
  - Run each target extension and record metrics
  - Report results
- **Acceptance criteria:**
  - All performance targets are met or have documented reasons for misses
  - No regressions in native extension performance
- **Complexity:** M

### Task 6.9: Stability verification
- **What to verify against spec targets (section 15.2):**
  - 0 crashes in 1 hour of normal use per extension
  - Memory stays under 150MB per active sidecar
  - IPC round-trip latency < 50ms (p95)
  - Sidecar idle timeout works (stops after 5 min idle)
  - Sidecar crash recovery works (retry up to 3x)
- **What to implement:**
  - Soak test script that runs each extension for extended period
  - Monitor memory usage over time
  - Verify crash recovery by killing sidecar process
  - Verify idle cleanup
- **Acceptance criteria:**
  - No unrecoverable crashes during 1-hour test
  - Memory stays within limits
  - Crash recovery works as specified
  - Idle timeout reclaims resources
- **Complexity:** L

---

## Summary

| Group | Tasks | Parallel With | Key Deliverable |
|-------|-------|--------------|-----------------|
| **1** | 1.1–1.6 | Group 2 | `photoncast-extension-ipc` crate |
| **2** | 2.1–2.13 | Group 1 | `packages/raycast-compat` npm package |
| **3** | 3.1–3.9 | — | `photoncast-raycast-bridge` crate |
| **4** | 4.1–4.5 | — | Integration with ExtensionManager |
| **5** | 5.1 | — | `photoncast dev` CLI command |
| **6** | 6.1–6.9 | — | E2E verification (6 extensions) |

**Total tasks:** 38
**Estimated total complexity:** ~8 XL, ~10 L, ~10 M, ~10 S
