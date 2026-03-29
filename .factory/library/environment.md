# Environment

**What belongs here:** Required env vars, external dependencies, setup notes.
**What does NOT belong here:** Service ports/commands (use `.factory/services.yaml`).

---

## Build Requirements
- macOS 12.0+
- Rust stable toolchain (MSRV 1.80, see rust-toolchain.toml)
- Xcode Command Line Tools (for macOS framework headers)

## No External Services
PhotonCast is a native desktop application with zero external service dependencies.
All data is stored locally in SQLite databases.

## Key Paths
- App data: `~/Library/Application Support/PhotonCast/`
- Config: `~/.config/photoncast/`
- Extensions: `~/Library/Application Support/PhotonCast/extensions/`
