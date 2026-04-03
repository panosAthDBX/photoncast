# PhotonCast Performance Tuning Enhancements — Specification

**Date:** 2026-02-09  
**Status:** Draft  
**Feature Area:** Performance, Optimization, Observability  
**Priority:** High

---

## 1. Overview

### 1.1 Purpose

Improve PhotonCast's runtime performance by implementing prioritized enhancements identified during the recent codebase review. Focus areas include prefetch throttling, async search paths, reduced cloning overhead, bounded watcher channels with backpressure, file-search backend consolidation, and observability metrics.

### 1.2 Goals

1. **Prefetch Throttling Fix**: Prevent prefetch storms that can flood the system with unnecessary work
2. **Async Normal-Mode Search Path**: Introduce async execution for the standard search path to avoid blocking the UI thread
3. **Reduced Clone-Heavy Provider Flows**: Minimize excessive `.clone()` calls in hot provider paths to lower allocation pressure
4. **Bounded Watcher Channels & Backpressure**: Prevent unbounded channel growth from file system watcher events
5. **File-Search Backend Consolidation**: Unify duplicated file-search logic into a single, optimized backend
6. **Observability Metrics**: Add tracing spans and metrics for ongoing performance monitoring

### 1.3 Target Audience

- **All PhotonCast users**: Improved responsiveness and lower resource usage
- **PhotonCast developers**: Better instrumentation, profiling, and debugging tools

### 1.4 Success Criteria

(To be defined during spec shaping)

---

## 2. Features

### 2.1 Prefetch Throttling Fix

(To be detailed)

### 2.2 Async Normal-Mode Search Path

(To be detailed)

### 2.3 Reduced Clone-Heavy Provider Flows

(To be detailed)

### 2.4 Bounded Watcher Channels & Backpressure

(To be detailed)

### 2.5 File-Search Backend Consolidation

(To be detailed)

### 2.6 Observability Metrics

(To be detailed)

---

## 3. Technical Requirements

(To be determined during spec shaping)

---

## 4. Implementation Plan

(To be determined after spec review)

---

## 5. Testing Strategy

(To be determined after spec review)

---

## 6. Open Questions

- What are the current prefetch rates and what throttle limits are appropriate?
- Which search provider flows are the most clone-heavy?
- What channel bound sizes are optimal for watcher backpressure?
- Which file-search backends exist today and what are the consolidation trade-offs?
- What tracing subscriber configuration is best for dev vs production?
