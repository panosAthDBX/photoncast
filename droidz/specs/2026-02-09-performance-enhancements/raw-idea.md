# PhotonCast Performance Enhancements

## 1. Overview

### Purpose
Improve PhotonCast's runtime performance based on recommendations from recent codebase review. Focus on reducing latency, minimizing unnecessary allocations, and adding instrumentation for ongoing performance monitoring.

### Goals
- Improve search path responsiveness and reduce perceived latency
- Reduce unnecessary caching overhead and excessive cloning
- Add backpressure mechanisms to file system watchers
- Enhance instrumentation and tracing for performance visibility

### Target Audience
- All PhotonCast users (improved responsiveness)
- PhotonCast developers (better instrumentation and profiling)

## 2. Features

### 2.1 Search Path Responsiveness
- Optimize hot paths in the search/ranking pipeline
- Reduce allocation pressure during search operations
- Improve incremental search update latency

### 2.2 Caching & Cloning Reductions
- Audit and reduce excessive `.clone()` calls in hot paths
- Replace owned types with borrowed references where possible
- Optimize cache invalidation strategies to avoid redundant work
- Evaluate `Arc`/`Rc` usage vs deep clones for shared data

### 2.3 Watcher Backpressure
- Implement backpressure/debouncing for file system watcher events
- Prevent watcher event floods from degrading UI responsiveness
- Add rate limiting for index rebuilds triggered by FS events
- Batch watcher notifications to reduce processing overhead

### 2.4 Instrumentation Improvements
- Add `tracing` spans to critical performance paths
- Implement timing metrics for search, indexing, and rendering
- Add configurable performance logging levels
- Enable runtime performance profiling hooks

## 3. Technical Requirements
(To be determined)

### Known Considerations
- PhotonCast is built in Rust with GPUI framework
- Must not regress existing functionality or stability
- Changes should be measurable with before/after benchmarks
- Instrumentation should have negligible overhead when disabled
- File system watcher uses `notify` crate

## 4. Success Criteria
(To be defined)

### Preliminary Targets
- Search response latency reduction (target: measurable improvement)
- Reduction in unnecessary allocations in hot paths
- No watcher event floods under heavy filesystem activity
- Tracing spans covering all critical performance paths
- Zero regressions in existing test suite

## 5. Key Questions & Research Needed
- Which specific search paths have the highest latency?
- What is the current clone/allocation profile in hot paths?
- What debounce interval is optimal for FS watcher events?
- Which `tracing` subscriber configuration best suits development vs production?
- Are there GPUI-specific performance patterns to leverage?

## 6. Dependencies
- Existing search engine and ranking system
- File system watcher infrastructure
- GPUI rendering pipeline
- `tracing` crate (likely already in use)
- `notify` crate for FS watching
