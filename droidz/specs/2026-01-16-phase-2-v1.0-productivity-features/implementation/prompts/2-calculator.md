# Implementation: Calculator (4.2)

## Task Assignment

You are implementing the **Built-in Calculator** feature for PhotonCast - a Rust-based macOS launcher using GPUI.

### Task Group: 4.2 Built-in Calculator (47 tasks)

#### 4.2.1 Infrastructure & Crate Setup

- [ ] **Task 4.2.1.1:** Create `photoncast-calculator` crate structure **(S)**
  - [ ] Set up Cargo.toml with dependencies (evalexpr, rust_decimal, chrono-tz, reqwest)
  - [ ] Create lib.rs with module structure
  - [ ] Define public API surface
  - **Dependencies:** None
  - **Acceptance:** Crate compiles, modules defined

- [ ] **Task 4.2.1.2:** Design `Calculator` struct **(S)**
  - [ ] Define struct with currency/crypto rate caches
  - [ ] Add last_update timestamp
  - [ ] Add city_timezones map
  - [ ] Implement constructor with defaults
  - **Dependencies:** None
  - **Acceptance:** Struct defined with all fields

#### 4.2.2 Math Expression Evaluation

- [ ] **Task 4.2.2.1:** Integrate evalexpr crate **(M)**
  - [ ] Add evalexpr dependency
  - [ ] Create context with built-in functions
  - [ ] Add constants: pi, e
  - [ ] Add basic functions: sqrt, abs, floor, ceil, round
  - [ ] Add trigonometric: sin, cos, tan, asin, acos, atan
  - [ ] Add hyperbolic: sinh, cosh, tanh
  - [ ] Add logarithmic: log, ln, exp
  - [ ] Add other: pow, mod, min, max, factorial
  - [ ] Write unit tests for all functions
  - **Dependencies:** 4.2.1.1
  - **Acceptance:**
    - All math functions work correctly
    - Evaluation <5ms for complex expressions

- [ ] **Task 4.2.2.2:** Implement expression preprocessing **(M)**
  - [ ] Handle implicit multiplication (2pi → 2*pi)
  - [ ] Handle percentage expressions (32% of 500)
  - [ ] Normalize input (whitespace, case)
  - [ ] Detect and route to specialized handlers
  - **Dependencies:** 4.2.2.1
  - **Acceptance:** Natural expressions evaluate correctly

#### 4.2.3 Currency Conversion

- [ ] **Task 4.2.3.1:** Implement fiat currency fetcher **(M)**
  - [ ] Create async fetcher using reqwest
  - [ ] Integrate frankfurter.app API
  - [ ] Parse response JSON to rate map
  - [ ] Handle 150+ fiat currencies
  - [ ] Implement error handling (network failures)
  - [ ] Add retry logic with backoff
  - [ ] Write tests with mock responses
  - **Dependencies:** 4.2.1.2
  - **Acceptance:**
    - Rates fetched successfully
    - All major currencies supported
    - Graceful error handling

- [ ] **Task 4.2.3.2:** Implement cryptocurrency fetcher **(M)**
  - [ ] Integrate CoinGecko API
  - [ ] Support top 15 cryptocurrencies:
    - BTC, ETH, USDT, BNB, XRP, ADA, DOGE, SOL
    - USDC, MATIC, AVAX, DOT, LINK
  - [ ] Parse response to rate map
  - [ ] Handle API rate limits
  - **Dependencies:** 4.2.1.2
  - **Acceptance:**
    - All listed cryptocurrencies supported
    - Rates accurate to CoinGecko

- [ ] **Task 4.2.3.3:** Implement SQLite cache for rates **(M)**
  - [ ] Create `currency_rates` table
  - [ ] Store base/target/rate/source/updated_at
  - [ ] Implement cache read on startup
  - [ ] Implement cache write after fetch
  - [ ] Add "rates as of X" display for offline mode
  - **Dependencies:** 4.2.3.1, 4.2.3.2
  - **Acceptance:**
    - Rates persist across restarts
    - Offline mode shows cached rates with timestamp

- [ ] **Task 4.2.3.4:** Implement update scheduler **(S)**
  - [ ] Schedule rate updates every 6 hours
  - [ ] Use tokio timer
  - [ ] Update both fiat and crypto rates
  - [ ] Handle update failures gracefully
  - **Dependencies:** 4.2.3.1, 4.2.3.2
  - **Acceptance:** Rates update automatically every 6 hours

- [ ] **Task 4.2.3.5:** Implement currency parser **(M)**
  - [ ] Parse expressions: "100 usd in eur", "100 usd to eur"
  - [ ] Support various formats: "$100 to €", "100$ in EUR"
  - [ ] Support cryptocurrency: "0.5 btc in usd"
  - [ ] Use Decimal128 for precision
  - [ ] Return formatted result with rate info
  - **Dependencies:** 4.2.3.3
  - **Acceptance:**
    - All currency formats parsed correctly
    - Decimal precision maintained

#### 4.2.4 Unit Conversion

- [ ] **Task 4.2.4.1:** Implement unit conversion engine **(L)**
  - [ ] Define unit categories and conversion factors
  - [ ] **Length:** mm, cm, m, km, in, ft, yd, mi
  - [ ] **Weight:** mg, g, kg, oz, lb, ton
  - [ ] **Volume:** ml, l, tsp, tbsp, cup, pt, qt, gal
  - [ ] **Temperature:** C, F, K (formulas)
  - [ ] **Data:** B, KB, MB, GB, TB, PB
  - [ ] **Speed:** m/s, km/h, mph, knots, ft/s
  - [ ] Implement bidirectional conversion
  - [ ] Support aliases: "kilometers", "km", "kms", "kilometre"
  - [ ] Make case-insensitive
  - [ ] Write unit tests for all conversions
  - **Dependencies:** 4.2.1.1
  - **Acceptance:**
    - All units convert correctly
    - Aliases recognized
    - Temperature formulas accurate

- [ ] **Task 4.2.4.2:** Implement unit parser **(M)**
  - [ ] Parse expressions: "5 km to miles", "100f in c"
  - [ ] Support natural language: "convert 5 miles to km"
  - [ ] Handle compound units where applicable
  - **Dependencies:** 4.2.4.1
  - **Acceptance:** Natural unit expressions evaluate correctly

#### 4.2.5 Date/Time Calculations

- [ ] **Task 4.2.5.1:** Implement natural language date parser **(L)**
  - [ ] Evaluate dateparser vs chrono-english crates
  - [ ] Parse relative dates: "monday in 3 weeks", "35 days ago"
  - [ ] Parse duration calculations: "days until dec 25"
  - [ ] Handle various date formats
  - [ ] Return DateTime<Local>
  - **Dependencies:** 4.2.1.1
  - **Acceptance:**
    - Common date phrases parsed correctly
    - Edge cases handled (year boundaries, DST)

- [ ] **Task 4.2.5.2:** Bundle city timezone database **(M)**
  - [ ] Create ~500 city to IANA timezone mapping
  - [ ] Include major cities worldwide
  - [ ] Support common abbreviations (ldn, sf, nyc)
  - [ ] Load at startup
  - **Dependencies:** None
  - **Acceptance:** 500 cities mapped to timezones

- [ ] **Task 4.2.5.3:** Implement timezone converter **(M)**
  - [ ] Parse: "time in dubai"
  - [ ] Parse: "5pm ldn in sf"
  - [ ] Parse: "2pm est to pst"
  - [ ] Use chrono-tz for conversions
  - [ ] Format output with timezone indicator
  - [ ] Handle DST correctly
  - **Dependencies:** 4.2.5.2
  - **Acceptance:**
    - All timezone expressions work
    - DST transitions handled correctly

#### 4.2.6 UI Components

- [ ] **Task 4.2.6.1:** Create calculator command **(S)**
  - [ ] Register calculator trigger in launcher
  - [ ] Detect math-like input patterns
  - [ ] Auto-activate on numeric input with operators
  - **Dependencies:** 4.2.2.1
  - **Acceptance:** Calculator activates automatically on math input

- [ ] **Task 4.2.6.2:** Implement calculator result view **(M)**
  - [ ] Create GPUI result component
  - [ ] Show formatted result prominently
  - [ ] Show expression being evaluated
  - [ ] Show rate/conversion info where applicable
  - [ ] Show "Updated X ago" for currency rates
  - [ ] Real-time evaluation with debounce
  - **Dependencies:** 4.2.6.1
  - **Acceptance:**
    - Results display clearly
    - Updates in real-time
    - Rate freshness visible

- [ ] **Task 4.2.6.3:** Implement calculator actions **(S)**
  - [ ] **Copy Formatted (Enter):** Copy "€92.47"
  - [ ] **Copy Raw (Cmd+Enter):** Copy "92.47"
  - [ ] **Refresh Rates (Cmd+R):** Force rate update
  - [ ] Show action panel
  - **Dependencies:** 4.2.6.2
  - **Acceptance:** All copy actions work correctly

- [ ] **Task 4.2.6.4:** Implement calculator history command **(M)**
  - [ ] Create separate "Calculator History" command
  - [ ] Store recent calculations
  - [ ] Allow re-running past calculations
  - [ ] Clear history option
  - **Dependencies:** 4.2.6.2
  - **Acceptance:** History persists, recallable

#### 4.2.7 Testing

- [ ] **Task 4.2.7.1:** Write unit tests **(M)**
  - [ ] Test all math functions
  - [ ] Test currency conversion accuracy
  - [ ] Test unit conversions (all categories)
  - [ ] Test date parsing
  - [ ] Test timezone conversions
  - [ ] Test edge cases (division by zero, overflow)
  - **Dependencies:** 4.2.2-4.2.5
  - **Acceptance:** 80%+ unit test coverage

- [ ] **Task 4.2.7.2:** Write integration tests **(M)**
  - [ ] Test currency rate fetch + cache + convert flow
  - [ ] Test offline mode fallback
  - [ ] Test full expression evaluation pipeline
  - **Dependencies:** 4.2.7.1
  - **Acceptance:** All integration tests pass

- [ ] **Task 4.2.7.3:** Add benchmarks **(S)**
  - [ ] Benchmark calc_basic_math (<5ms)
  - [ ] Benchmark calc_currency_conversion (<5ms after cache)
  - [ ] Benchmark calc_unit_conversion (<5ms)
  - **Dependencies:** 4.2.7.1
  - **Acceptance:** Performance targets met

---

## Context Files

Read these for requirements and patterns:
- **Spec:** `droidz/specs/2026-01-16-phase-2-v1.0-productivity-features/spec.md`
- **Requirements:** `droidz/specs/2026-01-16-phase-2-v1.0-productivity-features/requirements-answers.md`

## Standards to Follow

Read and follow ALL standards from `droidz/standards/`:
- `global/tech-stack.md` - Rust, GPUI, Tokio stack
- `global/coding-style.md` - Rust conventions (type safety, iterators, etc.)
- `global/error-handling.md` - Use thiserror + anyhow patterns
- `global/crate-first.md` - Always search for crates before implementing
- `backend/builtin-commands.md` - Calculator implementation patterns
- `frontend/components.md` - GPUI component patterns
- `testing/test-writing.md` - Test patterns (80% coverage required)

## Key Requirements

From the requirements answers:
- **Expression Parser:** Use `evalexpr` crate
- **Precision:** f64 for most, Decimal128 for currency
- **Currency API:** frankfurter.app for fiat (free, no API key)
- **Crypto API:** CoinGecko (free, no API key)
- **Update Frequency:** Every 6 hours with SQLite cache
- **Cryptocurrencies:** BTC, ETH, USDT, BNB, XRP, ADA, DOGE, SOL, USDC, MATIC, AVAX, DOT, LINK
- **Units:** Case-insensitive with comprehensive aliases
- **Date Parser:** Evaluate dateparser vs chrono-english
- **Timezone Database:** Bundle ~500 cities
- **Real-time Results:** Evaluate as user types with debounce
- **Copy Actions:** Enter = formatted, Cmd+Enter = raw number
- **Calculator History:** Separate command for past calculations

## Instructions

1. Read and analyze the spec.md for detailed requirements
2. Study existing codebase patterns in `crates/` directory
3. Create `crates/photoncast-calculator/` with proper structure
4. Implement features in dependency order (infrastructure → math → currency → units → dates → UI)
5. Write tests alongside implementation (aim for 80%+ coverage)
6. Run `cargo test` and `cargo clippy` to verify
7. Mark completed tasks with `[x]` in `droidz/specs/2026-01-16-phase-2-v1.0-productivity-features/tasks.md`

## Crates to Evaluate/Use

- `evalexpr` - Expression parsing and evaluation
- `rust_decimal` - High-precision decimal arithmetic
- `chrono` + `chrono-tz` - Date/time and timezone handling
- `dateparser` or `chrono-english` - Natural language date parsing (evaluate both)
- `reqwest` - Async HTTP for API calls
- `rusqlite` - SQLite for rate caching
- `serde` + `serde_json` - JSON parsing for API responses
- `tokio` - Async runtime and timers

## API References

### frankfurter.app (Fiat Currency)
```
GET https://api.frankfurter.app/latest?from=USD&to=EUR
Response: { "amount": 1, "base": "USD", "date": "2026-01-16", "rates": { "EUR": 0.92 } }
```

### CoinGecko (Crypto)
```
GET https://api.coingecko.com/api/v3/simple/price?ids=bitcoin,ethereum&vs_currencies=usd
Response: { "bitcoin": { "usd": 42000 }, "ethereum": { "usd": 2500 } }
```
