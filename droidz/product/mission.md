# PhotonCast - Product Mission

> Lightning-fast macOS launcher built in pure Rust

## Vision

PhotonCast aims to be the fastest, most reliable macOS application launcher that respects user privacy and never requires a subscription. We believe power users deserve a launcher that's as fast as their thoughts—without AI gimmicks, subscription fees, or privacy compromises.

## Problem Statement

### The Current Landscape

Modern macOS power users face a frustrating choice:

1. **Spotlight** - Native but limited in extensibility and workflow automation
2. **Raycast** - Feature-rich but increasingly AI-focused with subscription pressure
3. **Alfred** - Powerful but dated UI and paid Powerpack for advanced features
4. **Electron-based alternatives** - Slow, resource-hungry, non-native feel

### Pain Points We Address

| Problem | Current Solutions | PhotonCast Approach |
|---------|------------------|---------------------|
| **Slow startup** | Electron apps take 200-500ms to show | Sub-50ms activation |
| **Subscription fatigue** | Raycast pushing Pro/AI tiers | Forever free, open source |
| **Privacy concerns** | Cloud-synced data, telemetry | 100% local, zero telemetry |
| **Resource bloat** | 200MB+ RAM usage | Under 50MB target |
| **Limited customization** | Closed ecosystems | Fully open extension system |

## Target Users

### Primary: Power Users & Developers

- **macOS daily drivers** who live in their launcher
- **Developers** needing quick access to projects, terminals, and tools
- **Keyboard enthusiasts** who minimize mouse usage
- **Privacy-conscious users** who avoid cloud-dependent tools
- **Open source advocates** who prefer transparent software

### User Personas

#### "Alex" - The Developer
> "I switch between projects dozens of times per day. I need my launcher to be instant and remember my workflows."

- Uses launcher 50+ times daily
- Wants quick terminal access and project switching
- Values keyboard-driven workflows
- Frustrated by Raycast's AI push

#### "Jordan" - The Privacy-Focused Professional
> "I don't want my search history in anyone's cloud. Local-first or nothing."

- Works with sensitive documents
- Refuses cloud-synced tools
- Wants full control over their data
- Values open source transparency

#### "Sam" - The Efficiency Enthusiast
> "Every millisecond matters. I can feel the difference between native and Electron."

- Notices performance differences
- Tracks productivity metrics
- Optimizes every workflow
- Willing to configure for speed

## Solution Overview

PhotonCast is a **pure Rust** application launcher using **GPUI** (from Zed editor) for GPU-accelerated rendering at 120 FPS. It provides:

### Core Capabilities

1. **Instant Application Launching** - Fuzzy search across all macOS apps with intelligent ranking based on usage patterns

2. **Universal File Search** - Quick access to files and folders with preview support, powered by Spotlight metadata

3. **Built-in Calculator** - Math expressions evaluated inline with unit conversions

4. **Clipboard History** - Searchable history of recent clipboard items

5. **Window Management** - Keyboard-driven window positioning and snapping

6. **System Commands** - Quick access to sleep, restart, lock, empty trash, and more

7. **Raycast Extension Compatibility** - Run thousands of existing Raycast extensions without modification

8. **Raycast Store Integration** - Browse, install, and update extensions directly from the Raycast marketplace

9. **Native Extension System** - High-performance Rust extensions for deep system integration

10. **Custom Commands** - User-defined shortcuts and script execution

### Key Differentiator: Raycast Extension Ecosystem

PhotonCast isn't just another launcher—it's **compatible with the entire Raycast extension ecosystem**. This means:

- **Thousands of extensions available on day one** from the Raycast Store
- **No need to rebuild the ecosystem** from scratch
- **Community-driven** extensions without vendor lock-in
- **Open alternative** that respects the work of extension developers

## Value Proposition

### Why PhotonCast?

| Feature | PhotonCast | Raycast | Alfred | Spotlight |
|---------|-----------|---------|--------|-----------|
| **Price** | Free forever | Free + $10/mo Pro | $34 Powerpack | Free |
| **Performance** | ~50ms | ~100ms | ~80ms | ~60ms |
| **AI Features** | ❌ By design | ✅ Core focus | ✅ Add-on | ✅ Limited |
| **Open Source** | ✅ Full | ❌ Closed | ❌ Closed | ❌ Closed |
| **Privacy** | ✅ 100% local | ⚠️ Telemetry | ✅ Local | ⚠️ Apple |
| **Extensions** | ✅ Raycast-compatible | ✅ Store | ✅ Workflows | ❌ None |
| **Extension Store** | ✅ Raycast Store | ✅ Native | ❌ Manual | ❌ None |
| **Native** | ✅ Rust/GPUI | ❌ Electron | ✅ Native | ✅ Native |

### Our Competitive Edge

1. **Raycast Extensions, Zero Lock-in** - Use the thousands of Raycast extensions without the Raycast subscription or AI upselling.

2. **No AI, No Compromise** - We focus on core utility. AI is noise, not signal.

3. **Native Performance** - Pure Rust with GPU rendering means instant response.

4. **True Open Source** - Community-driven, auditable, forkable.

5. **Zero Data Collection** - What you search stays on your machine.

6. **One-Time Setup** - No accounts, no subscriptions, no upsells.

## Success Criteria

### MVP Success (v0.1)
- [ ] Cold start under 100ms
- [ ] App launch results in under 50ms
- [ ] Stable global hotkey activation
- [ ] Positive feedback from 100+ early adopters

### v1.0 Success
- [ ] 1,000+ GitHub stars
- [ ] 500+ daily active users (via opt-in analytics checkbox)
- [ ] Featured in macOS productivity lists
- [ ] 80%+ Raycast extension compatibility rate
- [ ] Raycast Store browser integrated

### Long-term Success
- [ ] 10,000+ stars
- [ ] Recognized as the "Raycast without AI" alternative
- [ ] 90%+ Raycast extension compatibility
- [ ] Sustainable through GitHub Sponsors
- [ ] Active native extension ecosystem alongside Raycast compatibility

## Non-Goals

To maintain focus, PhotonCast explicitly **will not**:

- ❌ Add AI/LLM features (use dedicated AI tools)
- ❌ Require accounts or cloud sync
- ❌ Collect telemetry without explicit opt-in
- ❌ Charge for core functionality
- ❌ Support Windows/Linux (macOS-first, native experience)
- ❌ Bundle a web browser or complex integrations

## Project Principles

### 1. Speed is a Feature
Every interaction must feel instant. If it's slow, it's broken.

### 2. Privacy by Default
No data leaves the machine unless the user explicitly enables it.

### 3. Simplicity Over Features
Do fewer things exceptionally well rather than many things poorly.

### 4. Open Development
All decisions, code, and roadmap are public and community-influenced.

### 5. Sustainable Simplicity
The codebase should be maintainable by a small team indefinitely.

---

*PhotonCast: Cast your commands at the speed of light.*
