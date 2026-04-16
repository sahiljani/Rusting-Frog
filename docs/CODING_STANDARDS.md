# Coding Standards — SEO Crawler (Rust)

**Scope:** Phase 1 and beyond. Every pattern here earns its keep — if it stops doing so, it's removed.

**The meta-rule:** a convention exists to (a) prevent a specific class of bug, (b) make a specific change cheap, or (c) hit a specific performance target. If you can't say which one, don't adopt it.

---

## Workspace layout

```
sf-clone-rust/
├── Cargo.toml                       ← workspace manifest
├── Cargo.lock                       ← committed
├── rust-toolchain.toml              ← pinned toolchain (stable)
├── .env.example                     ← template for local dev
├── docker-compose.yml               ← local PG + Redis
├── docker-compose.test.yml          ← ephemeral test infra
├── docs/
│   ├── SYSTEM_DESIGN.md
│   └── CODING_STANDARDS.md          ← this file
├── migrations/                      ← sqlx-managed, forward-only
│   ├── 0001_init.sql
│   └── ...
├── crates/
│   ├── core/                        ← domain types, NO IO
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── id.rs                ← newtype IDs
│   │       ├── tab.rs               ← TabKey enum
│   │       ├── filter_key.rs        ← FilterKey enum (ported from Java)
│   │       ├── config.rs            ← CrawlConfig struct + builder
│   │       ├── extracted.rs         ← ExtractedUrl, Finding
│   │       ├── error.rs             ← thiserror enums
│   │       └── repository.rs        ← traits (ports)
│   ├── api/                         ← HTTP/WS adapter
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs
│   │       ├── state.rs
│   │       ├── middleware/
│   │       ├── routes/              ← one file per resource
│   │       ├── repository/          ← sqlx impls of core traits
│   │       ├── ws.rs
│   │       └── error.rs             ← Problem (RFC 7807)
│   └── crawl-worker/                ← pipeline adapter
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs
│           ├── frontier.rs
│           ├── fetcher.rs
│           ├── parser.rs
│           ├── evaluator/
│           │   ├── mod.rs           ← Evaluator trait + registry
│           │   ├── internal.rs
│           │   ├── response_codes.rs
│           │   └── page_titles.rs
│           └── writer.rs
└── tests/                           ← workspace-level integration tests
```

**Dependency direction (enforced by `cargo-deny` / code review):**
- `core` depends on: serde, thiserror, std. Nothing IO-shaped.
- `api` depends on: core, axum, sqlx, redis, tokio, tracing.
- `crawl-worker` depends on: core, reqwest, scraper, sqlx, redis, tokio.
- **core does not depend on api or crawl-worker.** If it does, the abstraction leaked.

---

## Naming

| Kind | Convention | Example |
|---|---|---|
| Crate | `kebab-case` | `crawl-worker` |
| Module | `snake_case` | `mod page_titles` |
| Type (struct, enum, trait) | `PascalCase` | `CrawlConfig`, `Evaluator` |
| Function / method | `snake_case` | `fn extract_title` |
| Constant | `SCREAMING_SNAKE_CASE` | `MAX_URL_LENGTH` |
| Newtype ID wrapping UUID | `FooId` | `CrawlId`, `ProjectId` |
| Newtype ID wrapping i64 | `FooId` | `UrlId` |
| Error enum | `FooError` | `EvaluatorError`, `ApiError` |
| Trait for a port | noun ending in `-er` or bare noun | `UrlRepository`, `Fetcher` |
| Test module | `mod tests` at bottom of file | `#[cfg(test)] mod tests { ... }` |

**Don't:**
- Abbreviate when it saves nothing: `cfg` vs `config` — spell it out.
- Prefix with type: `StringUrl`, `IntCount` — the type system shows this.
- Hungarian notation: `m_crawl_id`, `s_name` — not this codebase.

---

## Rust idioms — the short list

### 1. Newtype IDs everywhere

```rust
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct CrawlId(pub uuid::Uuid);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct UrlId(pub i64);

#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct TenantId(pub String);
```

No function ever takes a bare `Uuid` or bare `i64` for an ID.

**Prevents:** passing `UrlId` where `CrawlId` was expected. Compile error, not production incident.

### 2. Enums, not booleans, for state

```rust
pub enum CrawlStatus {
    Queued,
    Running { started_at: OffsetDateTime },
    Completed { finished_at: OffsetDateTime, url_count: u32 },
    Failed { error: String },
}
```

Not three booleans. Not a status string. Not any other escape hatch.

**Prevents:** `{running: true, completed: true}` states that shouldn't exist.

### 3. Errors — `thiserror` for libraries, `anyhow` at the edge

```rust
// core/src/error.rs
#[derive(thiserror::Error, Debug)]
pub enum EvaluatorError {
    #[error("missing watermark config for {0:?}")]
    MissingWatermark(FilterKey),
    #[error("content selector '{0}' is not valid CSS")]
    InvalidSelector(String),
}

// api/src/main.rs
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // anyhow at the top-level for one-line error propagation
}
```

**Rule:** never `unwrap()` outside tests. Never `expect()` except in `main` at startup for required env vars.

### 4. `Result` everywhere, `?` to propagate

```rust
async fn fetch_url(url: &Url) -> Result<Response, FetchError> {
    let res = client.get(url.as_str()).send().await?;
    Ok(res)
}
```

**Don't:** panic on recoverable errors. Don't swallow errors with `.ok()` unless there's a documented reason.

### 5. `From`/`TryFrom` for conversion, not ad-hoc `parse_foo`

```rust
impl TryFrom<&str> for TabKey { ... }  // then TabKey::try_from(s)
```

### 6. Builder pattern for large configs

`CrawlConfig` has many fields. Use `typed-builder`:

```rust
#[derive(typed_builder::TypedBuilder)]
pub struct CrawlConfig {
    pub seed_url: String,
    #[builder(default = 5)]
    pub max_depth: u32,
    #[builder(default = 10_000)]
    pub max_urls: u32,
    #[builder(default = 5)]
    pub max_concurrency: u32,
    #[builder(default = true)]
    pub respect_robots: bool,
}
```

Usage:
```rust
let c = CrawlConfig::builder().seed_url("https://x".into()).build();
```

**Prevents:** 20-argument constructors. Compile-error on missing required fields.

---

## HTTP API conventions

### 7. Resource-shaped routes

```
GOOD:  POST /v1/projects/:id/crawls
BAD:   POST /v1/createCrawlForProject
```

### 8. Versioned at path prefix

All endpoints under `/v1/`. Breaking change → `/v2/`, not rename.

### 9. One canonical error response (RFC 7807 Problem)

```rust
#[derive(Serialize)]
pub struct Problem {
    #[serde(rename = "type")]
    pub type_: String,      // URI, e.g. "/errors/invalid-token"
    pub title: String,      // short human text
    pub status: u16,
    pub detail: Option<String>,
    pub instance: Option<String>,
    pub trace_id: String,
}
```

Content-Type: `application/problem+json`. Every non-2xx response is a `Problem`. No ad-hoc error bodies.

### 10. Cursor pagination, never offset

```
GET /v1/crawls/:id/urls?cursor=eyJpZCI6MTAwfQ&limit=100
```

Cursor is base64(JSON). Opaque to clients. Never `?offset=10000` — that's O(n) at scale.

### 11. Idempotency-Key on all mutating POSTs

```
POST /v1/crawls/:id/exports
Idempotency-Key: 3e4b0b8d-...
```

Server caches the response for 24h keyed by `(tenant_id, key)`. Retries return cached response.

### 12. Request validation at the boundary

```rust
#[derive(Deserialize, validator::Validate)]
pub struct CreateProjectRequest {
    #[validate(length(min = 1, max = 200))]
    pub name: String,
    #[validate(url)]
    pub seed_url: String,
}
```

Validation runs *before* the handler. Invalid → `400 Problem`.

### 13. OpenAPI generated from code (via `utoipa`)

```rust
#[utoipa::path(post, path = "/v1/projects",
    request_body = CreateProjectRequest,
    responses((status = 201, body = Project))
)]
async fn create_project(...) { ... }
```

Swagger UI at `/docs` in dev. Spec is the source of truth for the TS client.

---

## Async / concurrency

### 14. `tokio` everywhere, no `std::thread`

All IO is async. Pure-CPU hot paths use `tokio::task::spawn_blocking` or a `rayon` threadpool for parallelism.

### 15. No `.await` while holding a `Mutex`

```rust
// BAD
let guard = mutex.lock().await;
db.query(...).await?;

// GOOD
let value = { mutex.lock().await.clone() };
db.query(value).await?;
```

Prefer `parking_lot::Mutex` for short critical sections; use `tokio::sync::Mutex` only when you must await under the lock (rare).

### 16. Bounded channels only

`tokio::sync::mpsc::channel::<T>(N)` with explicit capacity. Never `unbounded_channel` without a written justification.

**Why:** unbounded = the producer can outrun the consumer = OOM.

### 17. Every external call has a deadline

- `reqwest::Client::builder().timeout(Duration::from_secs(20))`
- `sqlx` statement timeout via `PgPoolOptions`
- Redis operations wrapped in `tokio::time::timeout`

### 18. Graceful shutdown

Wire `tokio::signal::ctrl_c()` to a cancellation token. Handlers check `.is_cancelled()` and drain cleanly. `axum::Server::with_graceful_shutdown(...)`.

---

## Data / persistence

### 19. `sqlx::query!` / `sqlx::query_as!` for compile-time checked SQL

```rust
let row = sqlx::query_as!(
    ProjectRow,
    "SELECT id as \"id: ProjectId\", name, seed_url
     FROM projects WHERE tenant_id = $1 AND id = $2",
    tenant_id.0,
    project_id.0
).fetch_optional(&pool).await?;
```

**Prevents:** silent schema drift.
**Cost:** `DATABASE_URL` must point at a real DB at compile time (or use `sqlx prepare` + committed `sqlx-data.json`).

### 20. Tenant column on every tenanted table

First column index-wise, always: `(tenant_id, ...)`.
App-layer filter every query. RLS in Phase 2.

### 21. UUIDv7 for external IDs, bigint for internal

External (appearing in URLs/JSON): UUIDv7, time-ordered, globally unique, no info leak.
Internal (FKs only): `BIGSERIAL`, compact, fast joins.

### 22. Migrations forward-only

`migrations/NNNN_description.sql`. Never edit an applied migration. To undo: write a new `NNNN+1_revert_x.sql`.

Drop-column migrations are separate from code changes; ship in two deploys (stop reading, then drop).

### 23. JSONB for genuinely schemaless blobs only

Response headers → JSONB (truly variable).
`title_length` → column (it has a schema).
`CrawlConfig` → JSONB (many versions over time, evolves often).

---

## Logging / observability

### 24. `tracing` — structured, with spans

```rust
#[tracing::instrument(skip(pool), fields(crawl_id = %crawl_id))]
async fn run_crawl(pool: &PgPool, crawl_id: CrawlId) -> Result<()> {
    tracing::info!("starting crawl");
    ...
}
```

- Prod format: JSON (one line per event) for log aggregation.
- Dev format: pretty console.
- Filter: `RUST_LOG=info,sf_crawler=debug,sqlx=warn`.

### 25. No `println!`, no `eprintln!`, no `log!`

Only `tracing::{trace, debug, info, warn, error}`. Enforced by clippy config.

### 26. Trace correlation across services

Laravel sends `X-Trace-Id` header. Middleware extracts it and sets it as the root span's trace ID. Same ID appears in every downstream log line.

### 27. Metrics — RED per endpoint

Every HTTP handler emits: request rate, error rate, p50/p95/p99 duration. Prometheus endpoint at `/metrics`. Use `metrics` crate + `metrics-exporter-prometheus`.

### 28. Never log sensitive values

- `Authorization` header: redacted at the tracing layer.
- Any struct field named `*_secret`, `*_token`, `*_password`: `#[tracing::skip]` or custom Display.
- Query bodies: log only field names, not values, for endpoints carrying PII.

---

## Testing

### 29. Three tiers, explicit

| Tier | Scope | Run when |
|---|---|---|
| Unit (`#[test]`) | One fn or module, no IO | Every save |
| Integration (`crates/*/tests/`) | Crate with real PG via testcontainers | Pre-commit |
| E2E (`/tests/`) | Full stack via docker-compose | CI + pre-deploy |

### 30. Integration tests use real Postgres (testcontainers), not mocks

```rust
#[tokio::test]
async fn it_persists_a_project() {
    let pg = testcontainers::Postgres::default().start().await;
    let pool = pg.pool().await;
    sqlx::migrate!().run(&pool).await.unwrap();
    // ... test real sqlx queries
}
```

**No mocking of sqlx.** A mocked test that passes while the real migration breaks is worse than no test.

### 31. Property tests for parsers and normalization

```rust
#[proptest]
fn url_normalization_is_idempotent(url: String) {
    let once = normalize(&url);
    let twice = normalize(&once);
    prop_assert_eq!(once, twice);
}
```

Use `proptest` for: URL canonicalization, robots.txt parsing, cursor encoding, anywhere with unbounded inputs.

### 32. Fixture-based end-to-end test

A local nginx serves 20 hand-crafted HTML files (including pages with missing titles, duplicate titles, 404 response, redirect chains). E2E crawl → assert findings match expected set. Runs in CI.

### 33. Never skip tests

Failing test blocks merge. No `#[ignore]` without a linked issue and a deadline.

---

## Security

### 34. Secrets: env vars only

- Loaded at startup via `envy::from_env::<Config>()`.
- `.env.example` checked in with placeholder values.
- Real `.env`, `.env.production` in `.gitignore`.
- No secrets in Dockerfiles, compose files, or code.

### 35. SSRF defense is non-optional

Every URL fetched by `crawl-worker` goes through `is_safe_target(url)`:
1. Parse to `Url`.
2. Scheme must be `http` or `https`.
3. Resolve hostname → all returned IPs.
4. Every IP must be in public unicast ranges (no RFC1918, no loopback, no link-local, no AWS metadata IP).
5. Redirects re-checked (new URL passes through `is_safe_target` again).

Unit-tested against a curated attack list.

### 36. JWT verification — fail closed

- Required claims missing → 401.
- Signature invalid → 401.
- `exp` in past → 401.
- `nbf` in future → 401.
- Unknown `kid` → 401.
- Malformed token → 401.
- No path to a handler that runs without the middleware. Enforced by axum router composition (middleware applied at the router level, not per-route).

### 37. Dependency hygiene

- `cargo audit` in CI; fails on any RUSTSEC advisory of Medium+ severity.
- `cargo deny check advisories licenses bans sources` in CI.
- Lockfile committed.
- Dependency update PRs reviewed (not auto-merged).

---

## Code style

### 38. `rustfmt` enforced in CI

`rustfmt.toml` committed. No format-only PRs.

### 39. `clippy` must pass with `-D warnings`

Lints we keep strict:
- `clippy::pedantic` (selected: module_name_repetitions allowed).
- `clippy::unwrap_used` — deny except in tests.
- `clippy::expect_used` — warn.
- `clippy::panic` — deny in non-test code.

### 40. Comments

**Default: write no comment.** Well-named identifiers are the documentation.

Write a comment only when:
- The *why* is non-obvious (a hidden constraint, a workaround for a specific bug, a subtle invariant).
- Removing the comment would confuse the next reader.

Do not:
- Explain what the code does (the reader can see that).
- Reference the current task, fix, or caller ("for the X flow", "added for issue #42").
- Paste links to tickets.

### 41. Doc comments (`///`) on public API

Every `pub` item in `core/` has a doc comment with a one-line summary. Every `pub fn` explains its error conditions. `cargo doc` must build clean.

### 42. `#[must_use]` on types carrying invariants

`Result` is `#[must_use]` already. Add it to:
- Builders (forgetting `.build()` is silent otherwise).
- Guards returned from functions.

---

## What we deliberately do NOT do

- **No dependency injection container.** `Arc<AppState>` is enough.
- **No "clean architecture" onion with 5 layers.** 3 crates.
- **No Event Sourcing.** CQRS-lite, no event log.
- **No microservices.** Three binaries in one workspace.
- **No GraphQL.** REST fits the resource model.
- **No Kubernetes CRDs or operators.** Plain Deployments.
- **No service mesh in Phase 1.** Add only if we need mTLS or traffic policy.
- **No `unsafe`** without a code-review-blocking justification in a `// SAFETY:` comment.
- **No generics for flexibility "later"**. Concrete types now; generalize when the second caller appears.
- **No premature crates/modules.** Don't split a file before the file is too big to read.

---

## Review checklist (what reviewers look for)

Before approving a PR, check:

- [ ] New DB query includes `WHERE tenant_id = $1`.
- [ ] New public type has a doc comment.
- [ ] No `unwrap`/`expect` outside tests.
- [ ] External calls have timeouts.
- [ ] New endpoint has request validation and `utoipa` annotation.
- [ ] Error cases return `Problem`, not ad-hoc bodies.
- [ ] New dependency: approved license, audited, justified.
- [ ] Tests cover the happy path AND at least one failure case.
- [ ] No secrets or PII in logs.
- [ ] No new `#[allow(clippy::...)]` without a comment explaining why.
- [ ] If this introduces a new pattern, is it documented here?

---

## Change log

Amend when conventions change.

- _YYYY-MM-DD_ — initial standards for Phase 1.
