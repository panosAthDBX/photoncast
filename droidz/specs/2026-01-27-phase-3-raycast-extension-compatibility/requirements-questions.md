# Phase 3: Raycast Extension Compatibility — Requirements Questions

> Generated: 2026-01-27  
> Status: Awaiting user responses

---

## Research Summary

### What Exists Today

**Native Extension System (Fully Implemented):**
- ABI-stable extension API via `abi_stable` crate (`photoncast-extension-api`)
- Extension Manager with full lifecycle: discover → load → activate → deactivate → unload
- Hot-reload support in dev mode with versioned dylib caching
- Permission system with user consent dialogs and persistence
- Code signature verification for non-dev extensions
- Extension views: `ListView`, `DetailView`, `FormView`, `GridView` — all rendered natively in GPUI
- Host protocol (`ExtensionHostProtocol`) with: toast, HUD, clipboard, open URL/file, reveal in Finder, render/update views, preferences, storage, launch command, get frontmost application
- Navigation system with push/pop/replace and animated transitions
- All view types implement `Serialize`/`Deserialize` — ready for JSON-RPC transport
- Action system with handlers: Callback, OpenUrl, OpenFile, RevealInFinder, QuickLook, CopyToClipboard, PushView, SubmitForm
- PreferenceStore and ExtensionStorage (SQLite-backed)
- Cache API with TTL support
- View updates: `ViewHandle` with `update()`, `update_items()`, `set_loading()`, `set_error()`
- Fuzzy search integration for extension commands and search providers

**Phase 3 Planning Already Done (in spec.md §14):**
- Architecture decided: Node.js sidecar per extension, JSON-RPC over stdio
- IPC protocol: JSON-RPC with JSON Patch (RFC 6902) for incremental UI updates
- Manifest translation rules: Raycast `package.json` → PhotonCast internal format
- Sidecar lifecycle: Dormant → Starting → Ready → Active → Stopping → Stopped
- Lazy spawn, 5-min idle timeout, crash recovery (3 retries), 512MB memory limit
- Dependency resolution: esbuild bundling, `@raycast/api` replaced by `@photoncast/raycast-compat` shim
- Implementation phases: 3a (Core Sidecar), 3b (Full API), 3c (DX), 3d (Polish)
- Host services parity table with P0/P1/P2/P3 priorities
- Compatibility shim design for React → declarative schema serialization
- Open questions documented: Node.js version management, extension store, React version, menu bar, AI API

**Raycast API Reference (Comprehensive, in `droidz/standards/backend/raycast-api-reference.md`):**
- Complete TypeScript interfaces for: List, Grid, Detail, Form, Action/ActionPanel
- All data hooks: `useCachedPromise`, `useCachedState`, `usePromise`, `useFetch`, `useLocalStorage`
- Navigation hooks: `useNavigation` (push/pop)
- Storage APIs: `LocalStorage`, `Cache`
- Clipboard API with content types
- Toast & HUD notifications
- OAuth support (built-in providers + custom PKCE)
- Environment & Preferences APIs
- Image/Icon system with 100+ built-in icons
- Compatibility matrix (fully/partially/not supported)
- Implementation priority table

**Key Architectural Insight:** The existing native extension API already maps closely to Raycast's view model. `ExtensionView::List`, `Detail`, `Form`, `Grid` directly correspond to Raycast's UI components. The `ExtensionHostProtocol` trait already abstracts the host interface, meaning the sidecar just needs to implement the same protocol over JSON-RPC.

---

## Clarifying Questions

### A. Scope & Compatibility Level

1. **What compatibility target are you aiming for?**
   Context: The spec mentions "80%+ extension compatibility" in the API reference. The existing plan has 4 sub-phases (3a-3d).
   Options:
   - A) MVP: Core views (List, Detail) + essential host services (toast, clipboard, storage, preferences) — enough for ~40% of extensions
   - B) Solid: All 4 view types + ActionPanel + Navigation + data hooks — enough for ~70% of extensions
   - C) Comprehensive: Full API parity including OAuth, all hooks, Form validation — targeting 80%+ compatibility
   - D) Start with Phase 3a only, then iterate
   Suggested default: D (Phase 3a first, but designed for B/C)

2. **Should Phase 3 be scoped as a single spec, or should we create separate specs for each sub-phase (3a, 3b, 3c, 3d)?**
   Context: The existing spec §14 defines 4 sub-phases. A single spec risks being too large to implement in one sprint.
   Options:
   - A) Single comprehensive spec covering all sub-phases
   - B) Separate spec per sub-phase, starting with 3a
   Suggested default: B

3. **Which Raycast extensions do you want to test against as the primary compatibility targets?**
   Context: The spec mentions "top 50 Raycast extensions" for Phase 3d. Having specific targets earlier helps prioritize API coverage.
   Examples: GitHub, Jira, Linear, Notion, Todoist, Clipboard History, Color Picker, etc.
   Request: Please list 5-10 specific Raycast extensions you'd most like to see working.

### B. Runtime Architecture

4. **Confirm: Node.js sidecar per extension (as planned in §14.4)?**
   Context: The spec already decided on per-extension Node.js processes. Alternatives considered were: shared process (rejected for isolation), embedded V8/QuickJS (rejected for Node.js API compatibility), Deno (mixed compatibility).
   Options:
   - A) Per-extension Node.js process (as planned) — best compatibility, ~30-50MB per process
   - B) Shared Node.js process with Worker Threads — lower memory, less isolation
   - C) Deno sidecar — better security sandboxing, but some npm compatibility gaps
   Suggested default: A (as already decided)

5. **Node.js version management strategy?**
   Context: Spec §14.10 lists this as an open question. Raycast bundles its own Node.js.
   Options:
   - A) Bundle Node.js with PhotonCast app (like Raycast) — ~40MB added to app size, most reliable
   - B) Use system Node.js — smallest app size, but unpredictable versions
   - C) Managed download on first use — compromise, requires network on setup
   Suggested default: A (bundle with app, matching Raycast's approach)

6. **Should the sidecar Node.js runtime be a separate binary/crate, or embedded in the main PhotonCast binary?**
   Context: A separate `photoncast-sidecar` binary can be developed/tested independently. The main app would spawn it as a subprocess.
   Options:
   - A) Separate binary (`photoncast-sidecar`) spawned as subprocess
   - B) Node.js process spawned directly by the main app with a JS bootstrap script
   Suggested default: B (simpler; the "sidecar" is just Node.js running the shim package)

### C. API Shim Design

7. **React reconciler approach: How should React component trees be serialized?**
   Context: Raycast extensions use React JSX. The shim needs to capture the rendered component tree and serialize it to PhotonCast's declarative view schema. The spec §14.6 shows the general approach.
   Options:
   - A) Custom React reconciler that outputs JSON (like React Native, but targeting our schema)
   - B) Babel/SWC transform that converts JSX to direct JSON-RPC calls at build time
   - C) Runtime interception: override React.createElement to capture the tree
   Suggested default: A (custom reconciler — proven pattern, most compatible)

8. **How should Raycast's `useCachedPromise` and other React hooks be handled?**
   Context: Raycast has 5+ data-fetching hooks that manage loading states, caching, and revalidation. These are React hooks that trigger re-renders.
   Options:
   - A) Implement all hooks in the shim (full compatibility, more work)
   - B) Implement core hooks (`useCachedPromise`, `useNavigation`) only, stub others
   - C) Use `@raycast/utils` package directly (it's open-source), only shim the core `@raycast/api`
   Suggested default: C (leverage existing open-source `@raycast/utils`)

9. **How should callback-based actions (e.g., `onAction: () => void`) be handled across the IPC boundary?**
   Context: Raycast actions have JS callbacks. These can't be serialized over JSON-RPC. The current native API has `ActionHandler::Callback` which is `#[serde(skip)]`.
   Options:
   - A) Register callbacks in a sidecar-side registry, send callback IDs over IPC, host sends action events back
   - B) Only support declarative actions (OpenUrl, CopyToClipboard, etc.), convert callbacks where possible
   Suggested default: A (callback registry is the standard approach for this)

### D. Extension Installation & Discovery

10. **Extension store integration: What's the initial scope?**
    Context: Spec §14.10 lists this as an open question. The extensions standard mentions fetching from `api.raycast.com`.
    Options:
    - A) Manual install only (user clones/downloads extension, places in directory)
    - B) CLI tool: `photoncast install <extension-name>` that fetches from a registry
    - C) In-app store browser with search and one-click install from Raycast's store API
    - D) Git clone from Raycast's GitHub extensions monorepo
    Suggested default: A for Phase 3a, then D or B for later phases

11. **How should Raycast extensions be discovered alongside native extensions?**
    Context: Native extensions use `extension.toml` + `.dylib`. Raycast extensions use `package.json` + `.js` bundle. The `ExtensionManager` currently only handles native extensions.
    Options:
    - A) Separate `RaycastExtensionManager` that runs in parallel
    - B) Extend existing `ExtensionManager` to handle both types via an `ExtensionKind` enum
    - C) A `RaycastBridge` that wraps Raycast extensions as native `Extension` trait objects
    Suggested default: C (cleanest integration — the bridge presents Raycast extensions through the existing interface)

### E. Build Pipeline

12. **Should PhotonCast build Raycast extensions from source, or only run pre-built bundles?**
    Context: Raycast extensions are distributed as source (TypeScript + React). They need esbuild bundling. Pre-built bundles are single JS files.
    Options:
    - A) Build from source: include esbuild, run `npm install` + bundle on install
    - B) Pre-built only: require users to build before installing, or provide a separate build CLI
    - C) Hybrid: try to use pre-built `dist/` if available, fall back to building from source
    Suggested default: C (most flexible)

13. **Should we bundle esbuild with PhotonCast or use the system's?**
    Context: esbuild is a single Go binary (~9MB). Bundling it ensures consistent builds.
    Options:
    - A) Bundle esbuild with the app
    - B) Require system esbuild (via npm)
    - C) Download on first use
    Suggested default: A (reliable, small size)

### F. Security & Sandboxing

14. **What security model for Raycast extensions?**
    Context: Native extensions already have a permissions system. Raycast extensions run in a separate Node.js process which provides natural isolation. The extensions standard says "DON'T run extension code in main process."
    Options:
    - A) Process isolation only (Node.js subprocess with env_clear, limited env vars)
    - B) Process isolation + filesystem sandboxing (restrict to extension dir + temp)
    - C) Process isolation + full macOS sandbox profile (most secure, most complex)
    Suggested default: A for Phase 3a, evolve to B

15. **Should Raycast extensions go through the same permission consent flow as native extensions?**
    Context: Native extensions show a permissions dialog. Raycast extensions declare permissions differently (via `package.json`).
    Options:
    - A) Yes, translate Raycast permissions to PhotonCast format and show consent dialog
    - B) Auto-grant basic permissions (network, clipboard), only prompt for sensitive ones (filesystem)
    - C) No consent flow — rely on process isolation for security
    Suggested default: A (consistent UX, user trust)

### G. Performance & Resource Management

16. **What are the acceptable performance targets for Raycast extension operations?**
    Context: Native extensions have strict targets (load <50ms, search <20ms). Raycast extensions will inherently be slower due to Node.js process spawn and IPC overhead.
    Options:
    - A) Relaxed: First command <2s (cold start), subsequent <500ms
    - B) Moderate: First command <1s (cold start with pre-warming), subsequent <200ms
    - C) Aggressive: Match native extension performance (requires persistent sidecar)
    Suggested default: B

17. **Should Raycast extension sidecar processes be pre-warmed or lazy-spawned?**
    Context: Spec §14.4 says "lazy spawn on first command invocation" with 5-min idle timeout.
    Options:
    - A) Lazy spawn (as planned) — lower idle resource usage
    - B) Pre-warm top N used extensions at app startup
    - C) Configurable per-extension
    Suggested default: A for Phase 3a, consider B later based on user feedback

### H. Migration & Developer Experience

18. **Should PhotonCast support Raycast's `ray` CLI for extension development?**
    Context: Raycast developers use `ray develop` to test extensions. PhotonCast could provide a similar `photoncast develop` command.
    Options:
    - A) No CLI initially — use manual install for dev
    - B) Basic CLI: `photoncast dev <path>` to load a local Raycast extension
    - C) Full CLI with hot-reload, logging, and error overlay
    Suggested default: B for Phase 3a

19. **Error reporting: How should Raycast extension errors be surfaced?**
    Context: Raycast extensions can throw JS errors. These need to be caught and shown to the user.
    Options:
    - A) Toast notification with error message
    - B) Error view replacing the extension's view, with stack trace in dev mode
    - C) Both: toast for transient errors, error view for fatal errors
    Suggested default: C

20. **Source map support for debugging?**
    Context: Raycast extensions are bundled (esbuild output). Source maps help trace errors to original TypeScript source.
    Options:
    - A) No source maps initially
    - B) Source maps in dev mode only
    - C) Source maps always (stored alongside bundles)
    Suggested default: B

### I. Crate & Module Structure

21. **How should the Raycast compatibility layer be organized in the codebase?**
    Context: PhotonCast uses a workspace with multiple crates. The native extension API is in `photoncast-extension-api`.
    Options:
    - A) New crate: `photoncast-raycast-bridge` (Rust side) + `@photoncast/raycast-compat` (npm package)
    - B) Add to existing `photoncast-core/src/extensions/` with a `raycast/` submodule
    - C) New crate for the bridge + new crate for IPC protocol (`photoncast-extension-ipc`)
    Suggested default: C (separation of concerns: IPC protocol could be reused by future extension runtimes)

22. **Where should the Node.js shim package live?**
    Context: The `@photoncast/raycast-compat` npm package is a significant piece of TypeScript code.
    Options:
    - A) In-repo: `packages/raycast-compat/` (monorepo style)
    - B) Separate repo: `photoncast/raycast-compat`
    - C) In-repo: `crates/photoncast-raycast-bridge/sidecar/`
    Suggested default: A (keeps everything together for coordinated development)

---

## Visual Assets & Reference Materials Request

23. **Can you provide or point to:**
    - Screenshots of specific Raycast extensions you want to replicate (to verify visual parity)
    - The Raycast extension development docs (https://developers.raycast.com/) — any specific pages you consider critical
    - Any Raycast extensions you've personally built or use heavily
    - The Raycast extensions GitHub monorepo (https://github.com/raycast/extensions) — any specific extensions to study
    - Performance benchmarks from Raycast (command load times, search latency) if you have any

24. **Do you have a Raycast account/subscription that could be used to:**
    - Test the Raycast Store API for extension metadata fetching
    - Analyze the JSON-RPC protocol by intercepting Raycast's own sidecar communication
    - Profile real Raycast extension load times for benchmark targets

---

*Please respond with question numbers and your answers. Use "default" to accept the suggested default.*
