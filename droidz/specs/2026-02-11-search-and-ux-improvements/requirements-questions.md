# Clarifying Questions for Search & UX Improvements

## 1. Frecency-Based Result Sorting

### 1.1 **How aggressively should frecency override match quality?**
Context: Currently the formula is `(match_score + frecency * 10.0) * boosts`. A "sh" → "Shortcuts" match might score ~150 (good prefix match with boosts), while "sh" → "Shortwave" might score ~80 (weaker match) + frecency. The FRECENCY_MULTIPLIER of 10.0 may not be enough to overcome match quality differences.

Options:
- A) Increase `FRECENCY_MULTIPLIER` from 10.0 to something higher (e.g., 25.0 or 50.0) so heavily-used apps always dominate
- B) Use a tiered system: if frecency is above a threshold (e.g., app opened 5+ times recently), give it a massive boost that always wins over match quality
- C) Make the frecency weight configurable in preferences so the user can tune it
- Suggested default: **Option B** — tiered system feels most natural (like Raycast)

### 1.2 **Should there be a "pinned" or "favorite" concept?**
Context: Some launchers let you pin specific apps so they always appear first for certain queries. This is separate from frecency (which is automatic).

Options:
- A) No pinning, rely purely on frecency (keep it simple)
- B) Add manual pinning in preferences
- Suggested default: **Option A** — let frecency handle it, add pinning later if needed

### 1.3 **Should frecency be per-query or global?**
Context: Currently frecency is global — an app's score is the same regardless of what query triggered it. Raycast tracks per-query frecency (e.g., "sh" → Shortwave gets boosted specifically for "sh" queries).

Options:
- A) Keep global frecency (simpler, current approach)
- B) Add per-query frecency tracking (more accurate but more complex, more storage)
- C) Hybrid: global frecency + per-prefix frecency for short queries (2-3 chars)
- Suggested default: **Option B** — per-query frecency is what makes Raycast feel "smart"

### 1.4 **How should the recency half-life be tuned?**
Context: Current half-life is 72 hours. An app used 3 days ago has half the recency weight. This may be too aggressive or too slow depending on usage patterns.

Options:
- A) Keep 72 hours (current)
- B) Shorten to 24-48 hours (more responsive to recent usage)
- C) Lengthen to 1-2 weeks (more stable rankings)
- Suggested default: **Option A** — 72 hours seems reasonable

---

## 2. Better Fuzzy Matching + Frecency

### 2.1 **Is the spread factor (1.5x) too restrictive?**
Context: Currently `max_spread_factor: 1.5` filters out matches where characters are spread more than 1.5x the query length. This blocks "test" → "System Settings" (spread 1.75) which is arguably a bad match, but might also block valid fuzzy matches for some apps.

Can you give an example of a fuzzy match that currently fails but should work?

### 2.2 **Should word-boundary matching be prioritized?**
Context: Many launchers prioritize matches at word boundaries (e.g., "ss" matching **S**ystem **S**ettings, or "vsc" matching **V**isual **S**tudio **C**ode). The current nucleo matcher does some of this naturally, but we could add explicit word-boundary bonus scoring.

Options:
- A) Rely on nucleo's built-in word boundary handling (current)
- B) Add explicit word-boundary/acronym matching with bonus scoring
- Suggested default: **Option B** — word boundary matching is key for launcher UX

### 2.3 **Should there be a "learning period" for new apps?**
Context: When a new app is installed, it has zero frecency. It might get buried under frequently-used apps even when the user specifically types a close match. Should new/unrecognized apps get a temporary boost?

Options:
- A) No special handling (match quality still applies)
- B) New apps get a small boost for the first N days
- Suggested default: **Option A** — match quality already handles this well

---

## 3. Smarter App Indexing

### 3.1 **Should `/System/Library/CoreServices` be removed entirely?**
Context: This directory contains things like Finder.app, Bluetooth File Exchange.app, Archive Utility.app, Setup Assistant.app, etc. Some (like Finder) are useful to search for, but most are system services that shouldn't appear.

Options:
- A) Remove `/System/Library/CoreServices` entirely and whitelist only Finder.app
- B) Keep it but add more exclusion patterns (exclude helper apps, setup assistants, utilities that have no user-facing window)
- C) Only index `.app` bundles from CoreServices that have an `LSUIElement` of `false` in their Info.plist (i.e., they show in the Dock)
- Suggested default: **Option C** — most accurate, filters to user-facing apps

### 3.2 **What about Homebrew Cask apps?**
Context: Homebrew installs GUI apps as symlinks in `/Applications` which are already covered. But some users have Homebrew apps in non-standard locations. Currently `~/Applications` is scanned.

Is Homebrew Cask coverage sufficient as-is, or do we need to scan additional paths like `/opt/homebrew-cask/Caskroom`?

### 3.3 **Should app search scope be user-configurable?**
Context: Raycast has "Settings → Extensions → Applications → Search Scope" where users can add/remove directories. This would let power users include custom paths or exclude unwanted directories.

Options:
- A) No configuration, just fix the default paths
- B) Add a "Search Scope" setting in preferences
- Suggested default: **Option B** — gives users control, matches Raycast UX

### 3.4 **Should we add app exclusion/hiding?**
Context: Raycast lets you hide individual apps from search results (right-click → "Hide Application"). Even with good indexing, users may want to hide specific apps.

Options:
- A) No individual hiding (just fix indexing)
- B) Add "Hide from Search" action per app
- Suggested default: **Option B** for completeness, but can be deferred

### 3.5 **What about Setapp/enterprise-managed apps?**
Context: Setapp installs apps in `~/Applications/Setapp` which would be covered by `~/Applications`. Are there other enterprise MDM or non-standard paths to consider?

---

## 4. Faster App Quitting

### 4.1 **What is the perceived latency — is it the quit itself or the UI response?**
Context: The quit flow currently:
1. Calls `quit_app_by_bundle_id()` which iterates all running apps (O(n))
2. Sends terminate request
3. Polls every 100ms for up to 5s
4. Returns control
5. UI calls `self.hide(cx)`

The latency could be:
- A) The initial lookup (iterating all running apps) — unlikely to be slow
- B) The 5-second timeout waiting for the app to actually quit — most likely cause
- C) The UI blocking while waiting — the function is called synchronously on the main thread

Can you describe the latency more specifically? Is it seconds of frozen UI, or does the launcher just take a moment to dismiss?

### 4.2 **Should quit be fire-and-forget?**
Context: Currently the code waits for the app to actually terminate (up to 5s). We could instead send the terminate signal and immediately dismiss the launcher without waiting for confirmation.

Options:
- A) Fire-and-forget: send terminate, immediately dismiss launcher, don't wait
- B) Async with notification: send terminate, dismiss launcher, show a notification if the app didn't quit within timeout
- C) Keep waiting but reduce timeout to 1-2 seconds
- Suggested default: **Option A** — this is what Raycast does, quit is instant

### 4.3 **Should the quit operation be moved off the main thread?**
Context: Currently `quit_app_by_bundle_id()` is called synchronously in the UI handler. Even without the timeout, the `NSRunningApplication` lookup iterates all processes. Moving this to a background task would prevent any UI jank.

Options:
- A) Keep synchronous but make it fire-and-forget (just send the signal)
- B) Move to async task using GPUI's `cx.spawn()`
- Suggested default: **Option A** — simpler, and `NSRunningApplication.terminate()` itself is fast, it's only the polling loop that's slow

### 4.4 **Should there be visual feedback for quit?**
Context: When the user quits an app, should there be any indication that the quit was sent? (e.g., brief toast, status indicator, or just silently dismiss)

Options:
- A) Silent dismiss (quit signal sent, launcher closes)
- B) Brief toast notification "Quitting [App Name]..."
- C) Inline status on the result item before dismissing
- Suggested default: **Option A** — clean and fast

---

## General Questions

### G.1 **Priority ordering of these 4 improvements?**
Which should be tackled first? Suggested order:
1. Smarter app indexing (fixes bad data at the source)
2. Frecency-based sorting (biggest UX impact)
3. Better fuzzy matching (refines search quality)
4. Faster app quitting (quality of life)

### G.2 **Should all 4 be in a single PR or separate?**
Options:
- A) One PR with all 4 improvements
- B) Separate PRs per improvement
- C) Group related ones: indexing+search in one PR, quit in another
- Suggested default: **Option C**

### G.3 **Are there any other search issues beyond these 4?**
Anything else bothering you about search results, ranking, or the general UX?
