# Decisions Log: Search & UX Improvements

## Status: ✅ All Decisions Resolved

All decisions have been finalized based on user input. Ready for spec writing.

---

## Decisions Made

### D1: Frecency Weight Strategy
**Question**: How aggressively should frecency override match quality?
**Decision**: **Option A — Bump the FRECENCY_MULTIPLIER (25-50x)** so heavily-used apps always dominate match quality. Increase from current 10.0 to 25-50x range.
**Rationale**: Simpler than tiered system, directly addresses the "sh" → Shortwave problem by ensuring high-frecency apps outrank slightly better string matches.
**Status**: ✅ Resolved

### D2: Per-Query vs Global Frecency
**Question**: Should frecency be tracked per search query or globally?
**Decision**: **Option B — Per-query frecency tracking** (like Raycast). Track which app the user selects for each query prefix, so "sh" → Shortwave gets boosted specifically for "sh" queries.
**Rationale**: This is what makes Raycast feel "smart". More complex (requires schema migration, more storage) but significantly better UX.
**Status**: ✅ Resolved

### D3: Recency Half-Life
**Question**: Should the recency half-life be tuned?
**Decision**: **Keep current 72 hours**. No change needed.
**Status**: ✅ Resolved

### D4: Word Boundary Matching
**Question**: Should word-boundary/acronym matching get explicit bonus scoring?
**Decision**: **Option B — Add explicit word-boundary/acronym bonus scoring**. Don't rely solely on nucleo's built-in handling; add explicit bonuses for matching initial letters of words (e.g., "ss" → **S**ystem **S**ettings, "vsc" → **V**isual **S**tudio **C**ode).
**Status**: ✅ Resolved

### D5: CoreServices Handling
**Question**: How to handle /System/Library/CoreServices indexing?
**Decision**: **Option C — Only index CoreServices apps where LSUIElement=false in Info.plist**. This filters to user-facing apps (ones that show in the Dock) and excludes system helper apps, setup assistants, etc.
**Rationale**: Most accurate filtering method. Keeps Finder.app and other user-facing CoreServices apps while excluding background services.
**Status**: ✅ Resolved

### D6: User-Configurable Search Scope
**Question**: Should app search directories be user-configurable?
**Decision**: **Option B — Add user-configurable search scope in preferences**. Allow users to add/remove directories from the app search scope, matching Raycast's "Settings → Extensions → Applications → Search Scope" pattern.
**Status**: ✅ Resolved

### D7: Quit Behavior
**Question**: Should quit be fire-and-forget or wait for confirmation?
**Decision**: **Option A — Fire-and-forget**. Send `NSRunningApplication.terminate()`, immediately dismiss the launcher without waiting. No polling loop, no timeout, no toast notification.
**Rationale**: This is what Raycast does. The terminate signal is fast; it's the 5-second polling loop that causes perceived latency. Silent dismiss is the cleanest UX.
**Status**: ✅ Resolved

---

## Additional Decisions

### Pinning Feature
**Decision**: **Not needed**. Frecency (especially per-query frecency) handles the "favorite apps" use case naturally. Can revisit if users request it.

### New App Learning Period
**Decision**: **No special handling**. Match quality already ensures new apps appear for close matches. Frecency will build naturally as the user interacts.

### Quit Visual Feedback
**Decision**: **Silent dismiss**. No toast or status indicator. Launcher closes immediately after sending the terminate signal.

### Priority Order
**Decision**: Confirmed order:
1. **Smarter app indexing** — fixes bad data at the source
2. **Frecency-based sorting** — biggest UX impact
3. **Better fuzzy matching** — refines search quality
4. **Faster app quitting** — quality of life improvement
