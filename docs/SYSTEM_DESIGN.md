# SEO Crawler — System Design (Phase 1)

**Status:** Phase 1 scope only. The long-term vision (49 endpoints, 33 tabs, integrations, rendering) lives in `C:\Users\User\.claude\plans\ticklish-meandering-hollerith.md`. This document describes what we're actually *building first*.

**Phase 1 mission:** end-to-end crawl → React grid renders real findings. Prove the architecture works. Minimal feature set, production-grade bones.

---

## 1. Context

One paragraph, one picture. Who talks to what.

```
┌──────────┐                      ┌─────────────────────┐
│  React   │ ──── HTTPS + WS ───► │  Laravel web app    │
│  screen  │                      │  (existing system)  │
└──────────┘                      └──────────┬──────────┘
                                             │ internal HTTPS (JWT, shared secret)
                                             │ + webhooks back
                                             ▼
                                  ┌──────────────────────┐
                                  │  Rust SEO API        │ ─── HTTP fetch ──► target websites
                                  │  (this system)       │
                                  └──────────────────────┘
```

**System purpose.** The Rust SEO API crawls a seed URL, extracts SEO-relevant fields per page, runs evaluators to classify findings, and serves the results to the Laravel app's React screen.

**Users.** Marketers using the Laravel app. They never see the Rust API directly.

**Boundaries.**
- **Laravel owns:** user accounts, orgs, billing, session, UI shell. Laravel issues short-lived JWTs identifying `tenant_id` + `user_id` + scopes.
- **Rust API owns:** crawling, parsing, evaluation, persistence of crawl data, serving queries over that data.

**Out of Phase 1 scope:** headless Chromium rendering, GSC/GA4/PSI integrations, AI providers, visualisations, scheduler, exports, Compare mode, SERP mode.

---

## 2. Containers

Two Rust processes and two stateful stores. No object storage in Phase 1.

```
┌─────────────────────────────────────────────────────────────────┐
│                    Rust SEO API boundary                        │
│                                                                 │
│    ┌──────────────┐              ┌──────────────────┐           │
│    │     api      │ ─ enqueue ─► │  crawl-worker    │           │
│    │   (axum)     │              │  (tokio, reqwest)│           │
│    │              │ ◄─ pub/sub ─ │                  │           │
│    │  HTTP + WS   │              │  fetch → parse → │           │
│    │              │              │  evaluate → write│           │
│    └────┬─────────┘              └───────┬──────────┘           │
│         │                                │                      │
│         └──────────┐         ┌───────────┘                      │
│                    ▼         ▼                                  │
│              ┌──────────────┐   ┌─────────────┐                 │
│              │  PostgreSQL  │   │   Redis     │                 │
│              │              │   │             │                 │
│              │ Projects,    │   │ Job queue + │                 │
│              │ Crawls, URLs,│   │ Pub/Sub for │                 │
│              │ Links        │   │ live events │                 │
│              └──────────────┘   └─────────────┘                 │
└─────────────────────────────────────────────────────────────────┘
```

### Container table

| Container | Responsibility | Tech | Scales by | Stateful? |
|---|---|---|---|---|
| `api` | HTTP/WS surface, verify JWT, enqueue crawl jobs, serve grid queries, fan out live events to WS clients | axum 0.7, tokio, sqlx | Horizontal (stateless) | No |
| `crawl-worker` | Pull jobs, fetch URLs (static HTTP), parse HTML, run evaluators, write rows, publish progress | tokio, reqwest, scraper | Horizontal (one or more workers) | No |
| PostgreSQL | Projects, crawls, URL rows (with ~20 extracted columns), link edges | PG 15 | Vertical; read replica in Phase 2 | Yes |
| Redis | Durable job queue (Streams or a queue crate like `apalis`) + ephemeral pub/sub channel per crawl | Redis 7 | Vertical | Semi (queue durable, pub/sub ephemeral) |

### Decisions captured at container level

- **Two binaries, not one.** Workers do CPU-heavy parsing; API must stay latency-predictable. Splitting them means a stuck crawl can't starve the HTTP threadpool.
- **No render-worker in Phase 1.** Static HTTP only. Chromium adds ~200 MB RAM per worker, a CDP client dependency, and a whole class of bugs. Defer until Phase 2.
- **No S3 in Phase 1.** We don't store raw HTML or screenshots. If `store_raw_html=true` is needed later, S3 is added then.
- **Redis does two jobs.** Queue + pub/sub. Acceptable because both are Redis-native; splitting adds an extra dependency without payoff.
- **Findings computed at read time.** Phase 1 has no `crawl_url_findings` table; the grid query uses WHERE clauses on indexed columns. Works cleanly up to ~100k URLs; we'll add a materialized findings table in Phase 2 when filter-set grows.

---

## 3. Components

### 3.1 `api` — HTTP/WS server

```
api process
├── main.rs               ← bootstrap: config, pool, router, listen
├── state.rs              ← AppState (PgPool, RedisClient, JwtVerifier)
├── middleware/
│   ├── jwt.rs            ← extract + verify JWT, inject TenantId into request extensions
│   ├── trace.rs          ← tracing span per request, trace_id correlation
│   └── error.rs          ← convert Result<_, ApiError> → Problem response
├── routes/
│   ├── health.rs         ← GET /v1/health
│   ├── catalog.rs        ← GET /v1/catalog/tabs (static response)
│   ├── projects.rs       ← POST/GET /v1/projects
│   ├── crawls.rs         ← POST /v1/projects/:id/crawls, GET /v1/crawls/:id
│   ├── urls.rs           ← GET /v1/crawls/:id/urls, /urls/:url_id
│   ├── overview.rs       ← GET /v1/crawls/:id/overview
│   └── ws.rs             ← WS /v1/crawls/:id/live
├── repository/
│   ├── project_repo.rs   ← sqlx queries for projects table
│   ├── crawl_repo.rs     ← sqlx queries for crawls table
│   └── url_repo.rs       ← grid query + URL detail
└── error.rs              ← ApiError enum, Problem serialization
```

**Key responsibilities:**
- Verify JWT on every request; reject with 401 if signature/expiry invalid.
- Extract `tenant_id` from JWT claims and inject into every DB query.
- Translate HTTP request → repository call → HTTP response. No business logic beyond validation.
- For `POST /crawls`, enqueue a job into Redis; return `{crawl_id, status: "queued"}` immediately.
- For `WS /live`, subscribe to `crawl:{id}:events` Redis channel and forward events to the client.

**What `api` does NOT do:**
- Fetch URLs.
- Parse HTML.
- Run evaluators.
- Write to S3 (doesn't exist in Phase 1).

### 3.2 `crawl-worker` — the pipeline

```
crawl-worker process
├── main.rs               ← bootstrap + job_consumer loop
├── job_consumer.rs       ← pulls {crawl_id} from Redis queue
├── config_loader.rs      ← loads Phase1CrawlConfig from PG for given crawl
├── frontier.rs           ← BFS queue; persisted to PG to survive restarts
├── scope_filter.rs       ← decides include/exclude (same host, depth, limits)
├── robots.rs             ← robots.txt parse + cache per host
├── rate_limiter.rs       ← per-host token bucket (hardcoded 5 req/s Phase 1)
├── fetcher.rs            ← reqwest GET with timeout + retry
├── parser.rs             ← scraper DOM parse, extracts ~20 fields → ExtractedUrl
├── link_extractor.rs     ← <a> tags → normalized URLs + anchor/rel metadata
├── evaluator/
│   ├── mod.rs            ← Evaluator trait + registry
│   ├── internal.rs       ← groups URLs by content-type + status
│   ├── response_codes.rs ← groups by status band
│   └── page_titles.rs    ← Missing/Duplicate/Over60/Below30
├── writer.rs             ← batched INSERT into crawl_urls, crawl_links
├── progress_publisher.rs ← XADD to Redis Stream + PUBLISH to pub/sub
└── finalizer.rs          ← on drain: update crawls.status=completed, publish final event
```

**Pipeline flow (simplified):**

```
job_consumer
     │   pulls crawl_id
     ▼
config_loader → seed_url, max_depth, max_urls
     │
     ▼
frontier ◄─────────────────────────┐
     │   pops next URL             │ link_extractor pushes discovered links back
     ▼                             │
scope_filter → drop if out-of-scope│
     │                             │
     ▼                             │
robots.txt check (per host cache)  │
     │                             │
     ▼                             │
rate_limiter (token bucket)        │
     │                             │
     ▼                             │
fetcher → raw HTML + headers       │
     │                             │
     ▼                             │
parser → ExtractedUrl              │
     │                             │
     ├── link_extractor ───────────┘
     ▼
evaluator → Vec<Finding>
     │
     ▼
writer → batch INSERT to PG
     │
     ▼
progress_publisher → Redis pub/sub
```

**Why stages:** each stage runs as its own tokio task connected by bounded `mpsc` channels. If the writer is slow, the channel fills, parser blocks, fetcher throttles. Automatic backpressure.

**Concurrency plan:**
- 1 job_consumer task (one crawl at a time per worker).
- 1 frontier task (single source of truth for URL queue state).
- N fetcher tasks (default 5, configurable via `max_concurrency`).
- 1 parser task (CPU-bound; parallelize via rayon within if needed).
- 1 evaluator task.
- 1 writer task (batches every 500 ms or 100 rows, whichever first).

### 3.3 The evaluator engine (Phase 1 subset)

Trait:

```rust
pub trait Evaluator: Send + Sync {
    fn tab(&self) -> TabKey;
    fn filter_keys(&self) -> &[FilterKey];
    fn evaluate(&self, url: &ExtractedUrl, ctx: &EvalContext) -> SmallVec<[Finding; 4]>;
}
```

Phase 1 implementations: 3 evaluators covering ~14 filters.

| Evaluator | Tab | Filters |
|---|---|---|
| `InternalEvaluator` | Internal | `InternalAll`, `InternalHtml`, `InternalNonHtml` |
| `ResponseCodeEvaluator` | ResponseCode | `RespAll`, `Resp2xx`, `Resp3xx`, `Resp4xx`, `Resp5xx`, `RespBlockedByRobots` |
| `PageTitlesEvaluator` | PageTitles | `PageTitlesAll`, `PageTitlesMissing`, `PageTitlesDuplicate`, `PageTitlesOver60Chars`, `PageTitlesBelow30Chars` |

`PageTitlesDuplicate` is computed in a second pass after all URLs written (simplest correct approach). Near-duplicate and post-crawl analysis are Phase 2+.

---

## 4. Data Model

Four tables. All have `tenant_id TEXT NOT NULL` as the first indexed column.

### 4.1 `projects`

```sql
CREATE TABLE projects (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id    TEXT NOT NULL,
    name         TEXT NOT NULL,
    seed_url     TEXT NOT NULL,
    config       JSONB NOT NULL DEFAULT '{}',  -- Phase1CrawlConfig
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_projects_tenant ON projects (tenant_id, created_at DESC);
```

### 4.2 `crawls`

```sql
CREATE TABLE crawls (
    id             UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id      TEXT NOT NULL,
    project_id     UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    status         TEXT NOT NULL,  -- queued|running|completed|failed
    started_at     TIMESTAMPTZ,
    finished_at    TIMESTAMPTZ,
    url_count      INTEGER NOT NULL DEFAULT 0,
    error          TEXT,
    config_snapshot JSONB NOT NULL,  -- frozen copy of project.config at start
    created_at     TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_crawls_tenant_project ON crawls (tenant_id, project_id, created_at DESC);
CREATE INDEX idx_crawls_status ON crawls (status) WHERE status IN ('queued','running');
```

### 4.3 `crawl_urls`

The wide row. Every extracted field is its own column so grid filters are indexable.

```sql
CREATE TABLE crawl_urls (
    id                  BIGSERIAL PRIMARY KEY,
    tenant_id           TEXT NOT NULL,
    crawl_id            UUID NOT NULL REFERENCES crawls(id) ON DELETE CASCADE,
    url                 TEXT NOT NULL,
    url_hash            BYTEA NOT NULL,       -- sha256 of normalized URL
    depth               INT NOT NULL,
    -- HTTP
    response_code       SMALLINT,
    content_type        TEXT,
    size_bytes          INT,
    response_time_ms    INT,
    -- Extracted fields
    title               TEXT,
    title_length        INT,
    h1_1                TEXT,
    h1_1_length         INT,
    h1_count            INT,
    meta_description    TEXT,
    meta_desc_length    INT,
    canonical           TEXT,
    meta_robots         TEXT,
    word_count          INT,
    indexability        SMALLINT,  -- 0=Indexable, 1=Non-Indexable
    indexability_reason TEXT,
    -- Bookkeeping
    fetched_at          TIMESTAMPTZ,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX idx_urls_crawl_hash   ON crawl_urls (crawl_id, url_hash);
CREATE INDEX idx_urls_tenant_crawl        ON crawl_urls (tenant_id, crawl_id);
CREATE INDEX idx_urls_response_code       ON crawl_urls (crawl_id, response_code);
CREATE INDEX idx_urls_title_len           ON crawl_urls (crawl_id, title_length);
CREATE INDEX idx_urls_title_missing       ON crawl_urls (crawl_id) WHERE title IS NULL;
```

### 4.4 `crawl_links`

Edge table; used by both inlinks and outlinks queries.

```sql
CREATE TABLE crawl_links (
    id             BIGSERIAL PRIMARY KEY,
    tenant_id      TEXT NOT NULL,
    crawl_id       UUID NOT NULL REFERENCES crawls(id) ON DELETE CASCADE,
    source_url_id  BIGINT NOT NULL REFERENCES crawl_urls(id) ON DELETE CASCADE,
    target_url     TEXT NOT NULL,
    target_url_id  BIGINT REFERENCES crawl_urls(id) ON DELETE SET NULL,
    anchor_text    TEXT,
    rel            TEXT,
    follow         BOOL NOT NULL DEFAULT TRUE,
    link_order     INT NOT NULL
);
CREATE INDEX idx_links_out ON crawl_links (crawl_id, source_url_id);
CREATE INDEX idx_links_in  ON crawl_links (crawl_id, target_url_id) WHERE target_url_id IS NOT NULL;
```

### 4.5 Tenant isolation

- App layer: every sqlx query includes `WHERE tenant_id = $1` with the JWT-extracted tenant.
- DB layer (Phase 2): enable RLS with `USING (tenant_id = current_setting('app.tenant_id'))`.

Phase 1 ships with app-layer enforcement only. RLS is Phase 2 (it requires per-transaction `SET LOCAL`, a minor middleware add).

---

## 5. Key Sequences

Three flows. Mermaid-style prose.

### 5.1 Start a crawl

```
React → Laravel:  POST /api/seo/crawls {project_id, ...}
Laravel:          mint short-lived JWT with tenant_id, user_id, scopes
Laravel → Rust:   POST /v1/projects/:id/crawls  (Authorization: Bearer <jwt>)
Rust api:
  1. Verify JWT sig+exp, extract tenant_id
  2. Load project by (tenant_id, project_id); 404 if missing
  3. Insert row into crawls (status=queued, config_snapshot=project.config)
  4. XADD to Redis stream "crawl_jobs" {crawl_id}
  5. Return 202 {crawl_id, status:"queued"}
Laravel → React:  {crawl_id}
React:            open WS /v1/crawls/:id/live?token=<jwt>

(meanwhile, in crawl-worker)
crawl-worker:
  1. XREAD from "crawl_jobs" → gets crawl_id
  2. UPDATE crawls SET status='running', started_at=NOW()
  3. Load config_snapshot from crawls row
  4. Seed frontier with seed_url at depth 0
  5. PUBLISH crawl:{id}:events {"type":"started"}
  6. Begin pipeline loop
```

### 5.2 Fetch + parse + evaluate one URL

```
frontier.pop() → Url{url, depth}
scope_filter:
  - same host? in depth limit? not already seen? not in exclude regex?
  - if no → discard
robots.txt lookup (cached per host, 1h TTL):
  - if disallowed → record as blocked, continue
rate_limiter.acquire(host)
fetcher.get(url):
  - reqwest GET with 20s timeout, max 10 redirects
  - capture: status, headers, body, response_time
parser.parse(body):
  - scraper::Html::parse_document
  - extract 20+ fields → ExtractedUrl
link_extractor.extract(document):
  - every <a href>, classify internal/external, normalize target URL
  - push internal links back to frontier (scope-filtered)
evaluator_engine.evaluate(extracted_url, ctx):
  - each Evaluator returns 0..N Findings
  - flatten into Vec<Finding>
writer.write(extracted_url, findings, outlinks):
  - batched INSERT into crawl_urls (RETURNING id)
  - batched INSERT into crawl_links with source_url_id
progress_publisher:
  - PUBLISH crawl:{id}:events {"type":"url_done","url":..., "code":200}
  - increment counters in Redis
```

### 5.3 User browses results

```
React:   GET /v1/crawls/:id/overview  (JWT)
api:
  - verify JWT, extract tenant_id
  - SELECT COUNT(*) FROM crawl_urls WHERE tenant_id=$1 AND crawl_id=$2
  - SELECT filter counts per tab (one query, GROUP BY)
  - return JSON {total_urls, by_tab:{...}}

React:   GET /v1/crawls/:id/urls?tab=PAGE_TITLES&filter=MISSING&limit=100
api:
  - verify JWT
  - decode cursor (if any)
  - SELECT id, url, title, title_length, ... FROM crawl_urls
    WHERE tenant_id=$1 AND crawl_id=$2 AND title IS NULL
    ORDER BY id LIMIT 100
  - encode next_cursor from last id
  - return {rows:[...], next_cursor:"..."}

React:   GET /v1/crawls/:id/urls/:url_id
api:
  - single-row fetch; compute which filters matched this URL on-the-fly
  - return {url detail}
```

---

## 6. Non-Functional Targets

Commit to numbers.

| Target | Value | How we measure |
|---|---|---|
| API p95 latency (grid query, ≤100k URLs) | < 200 ms | k6 load test on fixture DB |
| API p99 latency (grid query) | < 500 ms | same |
| Crawl throughput per worker | ≥ 50 URLs/sec (static HTTP, 1 KB pages, same-host) | benchmark |
| Max crawl size (Phase 1) | 10,000 URLs | e2e test, verify no OOM |
| Concurrent crawls per tenant | 1 (Phase 1) | app-enforced |
| WS event delivery latency | < 1 sec from url_done to client | instrumented |
| Startup time | < 5 sec from `cargo run` to ready | manual |
| JWT verification overhead | < 1 ms per request | tracing histogram |

RPO: crawls in progress may be lost on worker crash; users restart. Phase 2 adds resume-from-frontier.
RTO: single-worker restart ≈ 10 sec. No HA requirement in Phase 1.

---

## 7. Failure Modes

| Failure | Detection | Mitigation (Phase 1) | Blast radius |
|---|---|---|---|
| crawl-worker process dies mid-crawl | Redis job visibility timeout expires | Job re-delivered; crawl restarts from beginning (no resume in P1) | That one crawl restarts; bounded by max_urls |
| Fetcher hits hostile site / infinite redirect | max_redirects=10, body_size=50MB cap | Drop URL, mark as errored, continue | One URL |
| Target site rate-limits us (429) | Response code | Per-host backoff, move URL to tail of frontier | That host slows |
| Target site is slow (60s TTFB) | 20s timeout | Abandon, mark response_time_ms=null, code=null | One URL |
| Postgres connection lost | sqlx error | Worker exits with non-zero; supervisor restarts | Job replayed |
| Redis unavailable | Connect error | api returns 503; worker blocks on XREAD with backoff | No new crawls accepted |
| JWT signature wrong (clock skew / rotated key) | verification fails | 401 with `Problem type=/errors/invalid-token` | One request |
| SSRF attempt (seed URL = 169.254.169.254) | resolver + IP-range check | Reject at POST /crawls with 400 | Prevented |
| HTML parse OOM on huge page | body_size_limit=50MB | reqwest caps the body before parse | One URL |
| Grid query on massive crawl | cursor pagination, indexed WHERE | p95 stays bounded | n/a |

---

## 8. Security

### 8.1 Trust boundaries

1. **Laravel → Rust API** — trusted. JWT-verified. Rust believes `tenant_id` claim.
2. **Rust API → Postgres / Redis** — trusted. Internal network only.
3. **crawl-worker → target websites** — **UNTRUSTED**. HTML is hostile. HTTP is hostile. DNS is hostile.

### 8.2 JWT verification

- Algorithm: HS256 with shared secret.
- Required claims: `tenant_id` (string), `user_id` (string), `iat`, `exp`, `scopes` (array).
- `kid` header supported for rotation; keyring in app config.
- Max token TTL: 15 minutes.
- Clock skew tolerance: 30 seconds.
- Missing/invalid → 401 `Problem type=/errors/invalid-token`.

### 8.3 Tenant isolation

- Every sqlx query includes `WHERE tenant_id = $1` bound from JWT claim.
- Code review rule: any new query without `tenant_id` filter is a blocker.
- Integration test: run two parallel crawls with different tenant IDs; verify zero cross-tenant row visibility.

### 8.4 SSRF protection in crawler

**Block before connect:**
- IPv4: `10.0.0.0/8`, `172.16.0.0/12`, `192.168.0.0/16`, `127.0.0.0/8`, `169.254.0.0/16`, `100.64.0.0/10`.
- IPv6: `::1`, `fc00::/7`, `fe80::/10`, `::ffff:0:0/96` (IPv4-mapped).
- DNS rebinding: resolve hostname, validate all returned IPs; re-validate after each redirect.
- Protocol whitelist: `http`, `https` only. No `file://`, `ftp://`, `gopher://`.

### 8.5 Secrets

- Shared JWT secret, Postgres DSN, Redis URL: env vars injected at container start.
- Never logged. `tracing` filter strips `Authorization` header and any field named `*_secret`.
- `.env.*` files ignored by git; dev uses `.env.example`.

### 8.6 Dependencies

- `cargo audit` runs in CI; fail on any RUSTSEC advisory ≥ Medium.
- `cargo deny` enforces license policy (no GPL, no unknown licenses).
- Lockfile committed.

---

## 9. Open Questions

Unresolved items the plan knowingly defers.

1. **How do we handle a crawl that exceeds `max_urls`?** Options: hard stop, soft-stop with warning, continue without writing new rows. → Decide before implementing frontier.
2. **Is there a sensible default user-agent string?** `SeoCrawler/0.1 (+https://yourdomain/bot-info)` probably — need to confirm with product owner.
3. **Do we bill per-crawl or per-URL?** → Laravel's concern, but Rust may need to emit usage webhooks. Defer.
4. **Cursor format.** Base64 of `{last_id: i64}` is simplest; does it need to include filter hash for cache invalidation? → Decide when we build the grid endpoint.
5. **Do we retry failed HTTP fetches?** SF retries 5xx once. Phase 1: no retry, write the failure row. Revisit after first real crawl.
6. **Duplicate titles across multiple tenants — do we group them?** → No. Always scope to single crawl.

---

## 10. Change log

- _YYYY-MM-DD_ — initial Phase 1 design.

Amend this file as the design evolves. When code and doc diverge, update doc or delete the misleading section.
