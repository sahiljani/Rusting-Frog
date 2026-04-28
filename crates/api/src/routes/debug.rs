// Read-side endpoints for the per-crawl debug log files written by
// crawl-worker into the shared `debug-logs` volume. The worker writes a
// `<crawl_id>.log` JSON Lines file (rotated at 5MB → .1, .2); this
// module just tails it.

use std::collections::HashMap;
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;

use axum::extract::{Path, Query, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::app_state::AppState;
use crate::error::ApiError;

const DEFAULT_LOG_DIR: &str = "/var/log/sf-debug";
const MAX_LIMIT: usize = 1000;
const DEFAULT_LIMIT: usize = 500;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/debug/crawls", get(list_crawls))
        .route("/debug/crawls/{id}/log", get(read_log))
        .route("/debug/crawls/{id}/tail", get(tail_log))
        .route("/debug/crawls/{id}/summary", get(summary))
        .route("/debug/crawls/{id}/log.jsonl", get(download_log))
}

fn log_dir() -> PathBuf {
    std::env::var("SF_DEBUG_LOG_DIR")
        .unwrap_or_else(|_| DEFAULT_LOG_DIR.to_string())
        .into()
}

/// Map a crawl_id to the active log file path, validating it parses as a
/// UUID first so `?id=../../etc/passwd` can't escape the log directory.
fn active_log_path(crawl_id: &str) -> Result<PathBuf, ApiError> {
    let uuid: Uuid = crawl_id
        .parse()
        .map_err(|_| ApiError::validation("crawl_id must be a UUID"))?;
    Ok(log_dir().join(format!("{uuid}.log")))
}

#[derive(Debug, Serialize)]
struct CrawlSummary {
    crawl_id: String,
    seed_url: Option<String>,
    started_at: Option<String>,
    ended_at: Option<String>,
    status: Option<String>,
    log_bytes: u64,
    rotated_files: usize,
}

async fn list_crawls(State(state): State<AppState>) -> Result<Json<Vec<CrawlSummary>>, ApiError> {
    let dir = log_dir();
    let mut out: Vec<CrawlSummary> = Vec::new();

    let entries = match std::fs::read_dir(&dir) {
        Ok(e) => e,
        // If the volume hasn't been written to yet, return empty rather
        // than 500 — the page is functional, just shows "no crawls".
        Err(_) => return Ok(Json(out)),
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n,
            None => continue,
        };
        // Active file only — rotated `.1` / `.2` are siblings of the same crawl.
        let stem = match name.strip_suffix(".log") {
            Some(s) => s,
            None => continue,
        };
        let uuid = match stem.parse::<Uuid>() {
            Ok(u) => u,
            Err(_) => continue,
        };

        let log_bytes = entry.metadata().map(|m| m.len()).unwrap_or(0);
        let mut rotated = 0;
        for n in 1..=2 {
            if dir.join(format!("{uuid}.log.{n}")).exists() {
                rotated += 1;
            }
        }

        // Fetch metadata from the crawls row. Best-effort: if missing,
        // we still surface the log file (the operator may have the file
        // but not the row, e.g. after a DB wipe). Using the non-macro
        // form because this query isn't in the offline sqlx cache.
        type CrawlMeta = (
            serde_json::Value,
            String,
            Option<chrono::DateTime<chrono::Utc>>,
            Option<chrono::DateTime<chrono::Utc>>,
        );
        let row: Option<CrawlMeta> = sqlx::query_as::<_, CrawlMeta>(
            "SELECT seed_urls, status, started_at, completed_at FROM crawls WHERE id = $1",
        )
        .bind(uuid)
        .fetch_optional(&state.db)
        .await
        .ok()
        .flatten();

        let (seed_url, status, started_at, ended_at) = match row {
            Some((seed_urls, st, started, ended)) => {
                let seed: Option<String> = serde_json::from_value::<Vec<String>>(seed_urls)
                    .ok()
                    .and_then(|v| v.into_iter().next());
                (
                    seed,
                    Some(st),
                    started.map(|t| t.to_rfc3339()),
                    ended.map(|t| t.to_rfc3339()),
                )
            }
            None => (None, None, None, None),
        };

        out.push(CrawlSummary {
            crawl_id: uuid.to_string(),
            seed_url,
            started_at,
            ended_at,
            status,
            log_bytes,
            rotated_files: rotated,
        });
    }

    // Newest first by mtime equivalent — sort by started_at descending,
    // putting unknowns last.
    out.sort_by(|a, b| b.started_at.cmp(&a.started_at));
    out.truncate(100);
    Ok(Json(out))
}

#[derive(Debug, Deserialize)]
struct LogParams {
    cursor: Option<u64>,
    limit: Option<usize>,
    /// `asc` (oldest first) or `desc` (newest first; default for the UI).
    order: Option<String>,
    /// Reserved: when true, after exhausting the active file pages into
    /// `.1` and `.2`. Not yet implemented — caller can read rotated
    /// content directly via `/log.jsonl` for now.
    #[allow(dead_code)]
    include_rotated: Option<bool>,
}

#[derive(Debug, Serialize)]
struct LogResponse {
    lines: Vec<Value>,
    next_cursor: Option<u64>,
    file_size_bytes: u64,
    is_running: bool,
}

async fn read_log(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(params): Query<LogParams>,
) -> Result<Json<LogResponse>, ApiError> {
    let path = active_log_path(&id)?;
    let cursor = params.cursor.unwrap_or(0);
    let limit = params.limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT);
    let order = params.order.as_deref().unwrap_or("desc");

    let (file_size, lines, next_cursor) = read_lines(&path, cursor, limit, order)?;

    let is_running = is_crawl_running(&state, &id).await;

    Ok(Json(LogResponse {
        lines,
        next_cursor,
        file_size_bytes: file_size,
        is_running,
    }))
}

#[derive(Debug, Deserialize)]
struct TailParams {
    since: Option<u64>,
}

#[derive(Debug, Serialize)]
struct TailResponse {
    lines: Vec<Value>,
    next_since: u64,
    is_running: bool,
}

async fn tail_log(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(params): Query<TailParams>,
) -> Result<Json<TailResponse>, ApiError> {
    let path = active_log_path(&id)?;
    let since = params.since.unwrap_or(0);
    let mut lines = Vec::new();
    let mut next_since = since;

    if path.exists() {
        let mut f = std::fs::File::open(&path).map_err(io_err)?;
        let size = f.metadata().map_err(io_err)?.len();
        if since < size {
            f.seek(SeekFrom::Start(since)).map_err(io_err)?;
            let mut buf = String::new();
            f.read_to_string(&mut buf).map_err(io_err)?;
            for raw in buf.split_terminator('\n') {
                if let Ok(v) = serde_json::from_str::<Value>(raw) {
                    lines.push(v);
                }
            }
            next_since = size;
        } else {
            next_since = size;
        }
    }

    let is_running = is_crawl_running(&state, &id).await;
    Ok(Json(TailResponse {
        lines,
        next_since,
        is_running,
    }))
}

#[derive(Debug, Default, Serialize)]
struct PhaseStats {
    count: u64,
    total_ms: u64,
    avg_ms: u64,
    p95_ms: u64,
}

#[derive(Debug, Default, Serialize)]
struct Totals {
    wall_ms: u64,
    urls_crawled: i64,
    bytes_downloaded: u64,
    peak_worker_rss_bytes: u64,
    peak_host_mem_used_bytes: u64,
    pg_db_growth_bytes: i64,
}

#[derive(Debug, Serialize)]
struct SummaryResponse {
    phases: HashMap<String, PhaseStats>,
    totals: Totals,
    is_running: bool,
}

async fn summary(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<SummaryResponse>, ApiError> {
    let path = active_log_path(&id)?;

    // Bucketed durations per phase.
    let mut buckets: HashMap<String, Vec<u64>> = HashMap::new();
    let mut totals = Totals::default();
    let mut first_pg_db: Option<i64> = None;
    let mut last_pg_db: Option<i64> = None;
    let mut crawl_end_ms: Option<u64> = None;

    if path.exists() {
        let body = std::fs::read_to_string(&path).map_err(io_err)?;
        for raw in body.split_terminator('\n') {
            let v: Value = match serde_json::from_str(raw) {
                Ok(v) => v,
                Err(_) => continue,
            };
            match v.get("kind").and_then(|k| k.as_str()) {
                Some("phase") => {
                    let phase = v.get("phase").and_then(|p| p.as_str()).unwrap_or("?");
                    let ms = v.get("ms").and_then(|m| m.as_u64()).unwrap_or(0);
                    if phase == "crawl_end" {
                        crawl_end_ms = Some(ms);
                    }
                    if phase == "fetch" {
                        if let Some(b) = v
                            .get("meta")
                            .and_then(|m| m.get("bytes"))
                            .and_then(|b| b.as_u64())
                        {
                            totals.bytes_downloaded += b;
                        }
                    }
                    buckets.entry(phase.to_string()).or_default().push(ms);
                }
                Some("sample") => {
                    let rss = v
                        .get("worker_rss_bytes")
                        .and_then(|x| x.as_u64())
                        .unwrap_or(0);
                    let host = v
                        .get("host_mem_used_bytes")
                        .and_then(|x| x.as_u64())
                        .unwrap_or(0);
                    if rss > totals.peak_worker_rss_bytes {
                        totals.peak_worker_rss_bytes = rss;
                    }
                    if host > totals.peak_host_mem_used_bytes {
                        totals.peak_host_mem_used_bytes = host;
                    }
                    let urls = v.get("urls_done").and_then(|x| x.as_i64()).unwrap_or(0);
                    if urls > totals.urls_crawled {
                        totals.urls_crawled = urls;
                    }
                    if let Some(pg) = v.get("pg_db_bytes").and_then(|x| x.as_i64()) {
                        if first_pg_db.is_none() {
                            first_pg_db = Some(pg);
                        }
                        last_pg_db = Some(pg);
                    }
                }
                _ => {}
            }
        }
    }

    if let (Some(a), Some(b)) = (first_pg_db, last_pg_db) {
        totals.pg_db_growth_bytes = b - a;
    }
    totals.wall_ms = crawl_end_ms.unwrap_or(0);

    let phases = buckets
        .into_iter()
        .map(|(k, mut v)| {
            v.sort_unstable();
            let count = v.len() as u64;
            let total_ms: u64 = v.iter().sum();
            let avg_ms = if count > 0 { total_ms / count } else { 0 };
            let p95_idx = ((v.len() as f64) * 0.95).ceil() as usize;
            let p95_ms = if v.is_empty() {
                0
            } else {
                v[p95_idx.min(v.len() - 1)]
            };
            (
                k,
                PhaseStats {
                    count,
                    total_ms,
                    avg_ms,
                    p95_ms,
                },
            )
        })
        .collect();

    let is_running = is_crawl_running(&state, &id).await;
    Ok(Json(SummaryResponse {
        phases,
        totals,
        is_running,
    }))
}

async fn download_log(Path(id): Path<String>) -> Result<Response, ApiError> {
    let path = active_log_path(&id)?;
    if !path.exists() {
        return Err(ApiError::not_found("debug log not found for crawl"));
    }
    let body = std::fs::read_to_string(&path).map_err(io_err)?;
    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "application/x-ndjson"),
            (
                header::CONTENT_DISPOSITION,
                "inline; filename=\"debug.jsonl\"",
            ),
        ],
        body,
    )
        .into_response())
}

async fn is_crawl_running(state: &AppState, id: &str) -> bool {
    let Ok(uuid) = id.parse::<Uuid>() else {
        return false;
    };
    let status: Option<String> =
        sqlx::query_scalar::<_, String>("SELECT status FROM crawls WHERE id = $1")
            .bind(uuid)
            .fetch_optional(&state.db)
            .await
            .ok()
            .flatten();
    matches!(
        status.as_deref(),
        Some("running") | Some("queued") | Some("starting")
    )
}

fn io_err(e: std::io::Error) -> ApiError {
    tracing::warn!(error = %e, "debug log io error");
    ApiError::internal(format!("log read failed: {e}"))
}

/// Read a window of lines from `path` starting at byte offset `cursor`.
/// `order=desc` returns the newest `limit` lines BEFORE the cursor (and
/// `next_cursor` points further back); `asc` returns `limit` lines AFTER
/// the cursor. Returns (file_size, lines, next_cursor).
fn read_lines(
    path: &std::path::Path,
    cursor: u64,
    limit: usize,
    order: &str,
) -> Result<(u64, Vec<Value>, Option<u64>), ApiError> {
    if !path.exists() {
        return Ok((0, vec![], None));
    }
    let mut f = std::fs::File::open(path).map_err(io_err)?;
    let size = f.metadata().map_err(io_err)?.len();

    if order == "asc" {
        if cursor >= size {
            return Ok((size, vec![], None));
        }
        f.seek(SeekFrom::Start(cursor)).map_err(io_err)?;
        let mut buf = String::new();
        f.read_to_string(&mut buf).map_err(io_err)?;
        let mut consumed: u64 = 0;
        let mut out = Vec::with_capacity(limit);
        for raw in buf.split_inclusive('\n') {
            if out.len() >= limit {
                break;
            }
            consumed += raw.len() as u64;
            let trimmed = raw.trim_end_matches('\n');
            if let Ok(v) = serde_json::from_str::<Value>(trimmed) {
                out.push(v);
            }
        }
        let new_cursor = cursor + consumed;
        let next = if new_cursor < size {
            Some(new_cursor)
        } else {
            None
        };
        return Ok((size, out, next));
    }

    // desc: read the whole file and slice from the back. At 5MB ceiling
    // this is cheap; not worth doing chunked reverse-read just yet.
    let mut buf = String::new();
    std::fs::File::open(path)
        .and_then(|mut f| f.read_to_string(&mut buf))
        .map_err(io_err)?;
    let mut all: Vec<&str> = buf.split_terminator('\n').collect();
    let total_lines = all.len();
    // cursor in desc mode = how many lines from the end we've already seen.
    let skip = cursor as usize;
    if skip >= total_lines {
        return Ok((size, vec![], None));
    }
    let end = total_lines - skip;
    let start = end.saturating_sub(limit);
    let slice: Vec<Value> = all
        .drain(start..end)
        .filter_map(|raw| serde_json::from_str::<Value>(raw).ok())
        .rev()
        .collect();
    let consumed = (end - start) as u64;
    let next_cursor = if start > 0 {
        Some(cursor + consumed)
    } else {
        None
    };
    Ok((size, slice, next_cursor))
}
