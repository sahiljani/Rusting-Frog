//! Reports — crawl-level analysis endpoints mirroring Screaming Frog's
//! "Reports" top-menu (All Redirects, Canonicals, Hreflang, Insecure
//! Content, SERP Summary, Orphan Pages, Structured Data, etc.).
//!
//! All reports read from existing DB tables and return a uniform envelope:
//!   { key, title, count, columns, rows }
//!
//! With `?format=csv` the endpoint returns `text/csv` instead.
//!
//! Endpoints:
//!   GET /v1/crawls/:id/reports                 → catalog of available reports
//!   GET /v1/crawls/:id/reports/:report_key     → run one report
//!
//! Reports that require crawler data we don't yet persist
//! (rendered-page, console-log, resources) return the envelope with
//! `rows: []` and `notes` explaining the missing capture so the UI
//! can degrade gracefully. HTTP headers and cookies are captured
//! (see `0002_headers_cookies.sql`) and those reports now return
//! real rows.

use axum::extract::{Path, Query, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::app_state::AppState;
use crate::error::ApiError;
use crate::extractors::auth::AuthUser;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/crawls/{crawl_id}/reports", get(list_reports))
        .route("/crawls/{crawl_id}/reports/{report_key}", get(run_report))
}

#[derive(Debug, Deserialize)]
pub struct ReportQuery {
    pub format: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Debug, Clone, Copy, Serialize)]
struct ReportDef {
    key: &'static str,
    title: &'static str,
    group: &'static str,
}

const REPORTS: &[ReportDef] = &[
    // Top-level overviews
    ReportDef {
        key: "crawl_overview",
        title: "Crawl Overview",
        group: "Overview",
    },
    ReportDef {
        key: "issues_overview",
        title: "Issues Overview",
        group: "Overview",
    },
    ReportDef {
        key: "segments_overview",
        title: "Segments Overview",
        group: "Overview",
    },
    ReportDef {
        key: "site_structure",
        title: "Site Structure",
        group: "Overview",
    },
    ReportDef {
        key: "crawl_path",
        title: "Crawl Path",
        group: "Overview",
    },
    // Redirects
    ReportDef {
        key: "redirects_all",
        title: "All Redirects",
        group: "Redirects",
    },
    ReportDef {
        key: "redirect_chains",
        title: "Redirect Chains",
        group: "Redirects",
    },
    ReportDef {
        key: "redirect_and_canonical_chains",
        title: "Redirect & Canonical Chains",
        group: "Redirects",
    },
    ReportDef {
        key: "redirects_to_error",
        title: "Redirects to Error",
        group: "Redirects",
    },
    // Canonicals
    ReportDef {
        key: "canonicals_missing",
        title: "Canonicals: Missing",
        group: "Canonicals",
    },
    ReportDef {
        key: "canonicals_all",
        title: "Canonicals: All",
        group: "Canonicals",
    },
    ReportDef {
        key: "canonicals_self_referencing",
        title: "Canonicals: Self Referencing",
        group: "Canonicals",
    },
    ReportDef {
        key: "canonicals_non_indexable",
        title: "Canonicals: Non-Indexable",
        group: "Canonicals",
    },
    ReportDef {
        key: "canonical_chains",
        title: "Canonical Chains",
        group: "Canonicals",
    },
    // Pagination
    ReportDef {
        key: "pagination_non_200",
        title: "Pagination: Non-200",
        group: "Pagination",
    },
    ReportDef {
        key: "pagination_unlinked",
        title: "Pagination: Unlinked",
        group: "Pagination",
    },
    // Hreflang
    ReportDef {
        key: "hreflang_missing",
        title: "Hreflang: Missing",
        group: "Hreflang",
    },
    ReportDef {
        key: "hreflang_inconsistent",
        title: "Hreflang: Inconsistent",
        group: "Hreflang",
    },
    ReportDef {
        key: "hreflang_non_canonical",
        title: "Hreflang: Non-Canonical",
        group: "Hreflang",
    },
    // Misc
    ReportDef {
        key: "insecure_content",
        title: "Insecure Content",
        group: "Security",
    },
    ReportDef {
        key: "serp_summary",
        title: "SERP Summary",
        group: "SERP",
    },
    ReportDef {
        key: "orphan_pages",
        title: "Orphan Pages",
        group: "Links",
    },
    // Structured data
    ReportDef {
        key: "structured_data_all",
        title: "Structured Data: All",
        group: "Structured Data",
    },
    ReportDef {
        key: "structured_data_errors",
        title: "Structured Data: Errors",
        group: "Structured Data",
    },
    // JavaScript
    ReportDef {
        key: "javascript_all",
        title: "JavaScript: All",
        group: "JavaScript",
    },
    ReportDef {
        key: "javascript_containing_js_content",
        title: "JavaScript: JS Content",
        group: "JavaScript",
    },
    // PageSpeed
    ReportDef {
        key: "pagespeed_all",
        title: "PageSpeed: All",
        group: "PageSpeed",
    },
    // Mobile
    ReportDef {
        key: "mobile_all",
        title: "Mobile: All",
        group: "Mobile",
    },
    ReportDef {
        key: "mobile_summary",
        title: "Mobile: Summary",
        group: "Mobile",
    },
    // Accessibility
    ReportDef {
        key: "accessibility_all",
        title: "Accessibility: All",
        group: "Accessibility",
    },
    ReportDef {
        key: "accessibility_summary",
        title: "Accessibility: Summary",
        group: "Accessibility",
    },
    // HTTP headers & cookies — backed by crawl_urls.response_headers
    ReportDef {
        key: "http_headers_all",
        title: "HTTP Headers: All",
        group: "HTTP Headers",
    },
    ReportDef {
        key: "http_headers_summary",
        title: "HTTP Headers: Summary",
        group: "HTTP Headers",
    },
    ReportDef {
        key: "cookies_all",
        title: "Cookies: All",
        group: "Cookies",
    },
    ReportDef {
        key: "cookies_summary",
        title: "Cookies: Summary",
        group: "Cookies",
    },
];

async fn list_reports(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(crawl_id): Path<Uuid>,
) -> Result<Json<Value>, ApiError> {
    verify_crawl_ownership(&state, &auth, &crawl_id).await?;
    let catalog: Vec<Value> = REPORTS
        .iter()
        .map(|r| json!({ "key": r.key, "title": r.title, "group": r.group }))
        .collect();
    Ok(Json(json!({ "reports": catalog })))
}

async fn run_report(
    auth: AuthUser,
    State(state): State<AppState>,
    Path((crawl_id, report_key)): Path<(Uuid, String)>,
    Query(q): Query<ReportQuery>,
) -> Result<Response, ApiError> {
    verify_crawl_ownership(&state, &auth, &crawl_id).await?;
    let limit = q.limit.unwrap_or(5000).min(20_000);

    let def = REPORTS
        .iter()
        .find(|r| r.key == report_key)
        .ok_or_else(|| ApiError::not_found(format!("unknown report '{}'", report_key)))?;

    let (columns, rows, notes) = build_report(&state, &crawl_id, def.key, limit).await?;

    let csv = matches!(q.format.as_deref(), Some("csv"));
    if csv {
        let body = to_csv(&columns, &rows);
        let filename = format!("{}.csv", def.key);
        return Ok((
            StatusCode::OK,
            [
                (header::CONTENT_TYPE, "text/csv; charset=utf-8".to_string()),
                (
                    header::CONTENT_DISPOSITION,
                    format!("attachment; filename=\"{}\"", filename),
                ),
            ],
            body,
        )
            .into_response());
    }

    let envelope = json!({
        "key": def.key,
        "title": def.title,
        "group": def.group,
        "count": rows.len(),
        "columns": columns,
        "rows": rows,
        "notes": notes,
    });
    Ok((StatusCode::OK, Json(envelope)).into_response())
}

type ReportOutput = (Vec<&'static str>, Vec<Value>, Option<String>);

async fn build_report(
    state: &AppState,
    crawl_id: &Uuid,
    key: &str,
    limit: i64,
) -> Result<ReportOutput, ApiError> {
    match key {
        "crawl_overview" => report_crawl_overview(state, crawl_id).await,
        "issues_overview" => report_issues_overview(state, crawl_id).await,
        "segments_overview" => report_segments_overview(state, crawl_id, limit).await,
        "site_structure" => report_site_structure(state, crawl_id, limit).await,
        "crawl_path" => report_crawl_path(state, crawl_id, limit).await,

        "redirects_all" => report_redirects_all(state, crawl_id, limit).await,
        "redirect_chains" => report_redirect_chains(state, crawl_id, limit).await,
        "redirect_and_canonical_chains" => {
            report_redirect_and_canonical_chains(state, crawl_id, limit).await
        }
        "redirects_to_error" => report_redirects_to_error(state, crawl_id, limit).await,

        "canonicals_all" => report_canonicals_all(state, crawl_id, limit).await,
        "canonicals_missing" => report_finding(state, crawl_id, "canonicals_missing", limit).await,
        "canonicals_self_referencing" => report_canonicals_self(state, crawl_id, limit).await,
        "canonicals_non_indexable" => report_canonicals_non_indexable(state, crawl_id, limit).await,
        "canonical_chains" => report_canonical_chains(state, crawl_id, limit).await,

        "pagination_non_200" => report_pagination_non_200(state, crawl_id, limit).await,
        "pagination_unlinked" => {
            report_finding(
                state,
                crawl_id,
                "pagination_unlinked_pagination_urls",
                limit,
            )
            .await
        }

        "hreflang_missing" => report_finding(state, crawl_id, "hreflang_missing", limit).await,
        "hreflang_inconsistent" => {
            report_finding(
                state,
                crawl_id,
                "hreflang_inconsistent_language_return_links",
                limit,
            )
            .await
        }
        "hreflang_non_canonical" => {
            report_finding(
                state,
                crawl_id,
                "hreflang_non_canonical_return_links",
                limit,
            )
            .await
        }

        "insecure_content" => report_insecure_content(state, crawl_id, limit).await,
        "serp_summary" => report_serp_summary(state, crawl_id, limit).await,
        "orphan_pages" => report_orphan_pages(state, crawl_id, limit).await,

        "structured_data_all" => {
            report_finding(state, crawl_id, "structured_data_all", limit).await
        }
        "structured_data_errors" => {
            report_finding(state, crawl_id, "structured_data_parse_errors", limit).await
        }

        "javascript_all" => report_finding(state, crawl_id, "javascript_all", limit).await,
        "javascript_containing_js_content" => {
            report_finding(state, crawl_id, "javascript_contains_js_content", limit).await
        }

        "pagespeed_all" => report_finding(state, crawl_id, "pagespeed_all", limit).await,

        "mobile_all" => report_finding(state, crawl_id, "mobile_all", limit).await,
        "mobile_summary" => report_category_summary(state, crawl_id, "mobile", limit).await,
        "accessibility_all" => report_finding(state, crawl_id, "accessibility_all", limit).await,
        "accessibility_summary" => {
            report_category_summary(state, crawl_id, "accessibility", limit).await
        }

        "http_headers_all" => report_http_headers(state, crawl_id, limit).await,
        "http_headers_summary" => report_http_headers_summary(state, crawl_id, limit).await,
        "cookies_all" => report_cookies(state, crawl_id, limit).await,
        "cookies_summary" => report_cookies_summary(state, crawl_id, limit).await,

        other => Err(ApiError::not_found(format!("unknown report '{}'", other))),
    }
}

// ---------- helpers ----------

async fn verify_crawl_ownership(
    state: &AppState,
    auth: &AuthUser,
    crawl_id: &Uuid,
) -> Result<(), ApiError> {
    let exists = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM crawls WHERE id = $1 AND tenant_id = $2",
        crawl_id,
        auth.tenant_id.as_str()
    )
    .fetch_one(&state.db)
    .await?;
    if exists.unwrap_or(0) == 0 {
        return Err(ApiError::not_found("crawl not found"));
    }
    Ok(())
}

fn to_csv(columns: &[&'static str], rows: &[Value]) -> String {
    let mut out = String::new();
    // header
    let headers: Vec<String> = columns.iter().map(|c| csv_escape(c)).collect();
    out.push_str(&headers.join(","));
    out.push('\n');
    for r in rows {
        let line: Vec<String> = columns
            .iter()
            .map(|c| match r.get(*c) {
                Some(Value::Null) | None => String::new(),
                Some(Value::String(s)) => csv_escape(s),
                Some(Value::Bool(b)) => b.to_string(),
                Some(Value::Number(n)) => n.to_string(),
                Some(v) => csv_escape(&v.to_string()),
            })
            .collect();
        out.push_str(&line.join(","));
        out.push('\n');
    }
    out
}

fn csv_escape(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        let escaped = s.replace('"', "\"\"");
        format!("\"{}\"", escaped)
    } else {
        s.to_string()
    }
}

// ---------- individual report implementations ----------

async fn report_crawl_overview(
    state: &AppState,
    crawl_id: &Uuid,
) -> Result<ReportOutput, ApiError> {
    let rows = sqlx::query!(
        r#"
        SELECT filter_key, COUNT(*) as "count!"
        FROM crawl_url_findings
        WHERE crawl_id = $1
        GROUP BY filter_key
        ORDER BY filter_key
        "#,
        crawl_id,
    )
    .fetch_all(&state.db)
    .await?;

    let data: Vec<Value> = rows
        .into_iter()
        .map(|r| json!({ "filter_key": r.filter_key, "count": r.count }))
        .collect();
    Ok((vec!["filter_key", "count"], data, None))
}

async fn report_issues_overview(
    state: &AppState,
    crawl_id: &Uuid,
) -> Result<ReportOutput, ApiError> {
    let rows = sqlx::query!(
        r#"
        SELECT filter_key, COUNT(*) as "count!"
        FROM crawl_url_findings
        WHERE crawl_id = $1
        GROUP BY filter_key
        ORDER BY filter_key
        "#,
        crawl_id,
    )
    .fetch_all(&state.db)
    .await?;

    // Infer severity client-side using the filter_key catalog; here we just
    // return the raw per-filter counts grouped by a coarse severity derived
    // from the key shape (errors/warnings/opportunities/info). The UI can
    // enrich this against /v1/catalog/tabs for the authoritative mapping.
    let data: Vec<Value> = rows
        .into_iter()
        .map(|r| {
            let sev = if r.filter_key.contains("missing")
                || r.filter_key.contains("broken")
                || r.filter_key.contains("error")
                || r.filter_key.contains("duplicate")
            {
                "Issue"
            } else if r.filter_key.contains("over_")
                || r.filter_key.contains("below_")
                || r.filter_key.contains("non_")
            {
                "Warning"
            } else {
                "Info"
            };
            json!({ "filter_key": r.filter_key, "count": r.count, "severity": sev })
        })
        .collect();
    Ok((vec!["filter_key", "count", "severity"], data, None))
}

async fn report_site_structure(
    state: &AppState,
    crawl_id: &Uuid,
    _limit: i64,
) -> Result<ReportOutput, ApiError> {
    let rows = sqlx::query!(
        r#"
        SELECT depth, COUNT(*) as "count!"
        FROM crawl_urls
        WHERE crawl_id = $1 AND is_internal = true
        GROUP BY depth
        ORDER BY depth
        "#,
        crawl_id,
    )
    .fetch_all(&state.db)
    .await?;
    let data: Vec<Value> = rows
        .into_iter()
        .map(|r| json!({ "depth": r.depth, "count": r.count }))
        .collect();
    Ok((vec!["depth", "count"], data, None))
}

/// Default segmentation: group internal URLs by (host, first path segment).
/// SF's full Segments feature is regex-rule-driven; until we add the
/// configuration UI for custom rules, this "host + top-level path"
/// bucketing tracks the coarse site structure most users want first
/// (e.g. `/blog/*`, `/products/*`, `/support/*` under a single host).
async fn report_segments_overview(
    state: &AppState,
    crawl_id: &Uuid,
    limit: i64,
) -> Result<ReportOutput, ApiError> {
    // Bucket URLs in Rust so we don't need a new sqlx::query! offline
    // cache entry — and so the host/first-seg parsing stays identical
    // to what the `url` crate does elsewhere in the codebase.
    let rows = sqlx::query!(
        r#"
        SELECT url, status_code
        FROM crawl_urls
        WHERE crawl_id = $1 AND is_internal = true
        "#,
        crawl_id,
    )
    .fetch_all(&state.db)
    .await?;

    use std::collections::BTreeMap;
    #[derive(Default)]
    struct Bucket {
        urls: u64,
        ok_urls: u64,
        redirects: u64,
        errors: u64,
    }
    let mut agg: BTreeMap<(String, String), Bucket> = BTreeMap::new();
    for r in rows {
        let parsed = match url::Url::parse(&r.url) {
            Ok(u) => u,
            Err(_) => continue,
        };
        let host = parsed.host_str().unwrap_or("").to_string();
        let first_seg = parsed
            .path_segments()
            .and_then(|mut it| it.find(|s| !s.is_empty()))
            .unwrap_or("")
            .to_string();
        let entry = agg.entry((host, first_seg)).or_default();
        entry.urls += 1;
        match r.status_code {
            Some(c) if (200..300).contains(&c) => entry.ok_urls += 1,
            Some(c) if (300..400).contains(&c) => entry.redirects += 1,
            Some(c) if c >= 400 => entry.errors += 1,
            _ => {}
        }
    }

    let mut data: Vec<Value> = agg
        .into_iter()
        .map(|((host, first_seg), b)| {
            let segment = if first_seg.is_empty() {
                format!("{}/", host)
            } else {
                format!("{}/{}", host, first_seg)
            };
            json!({
                "segment": segment,
                "host": host,
                "path_prefix": first_seg,
                "urls": b.urls,
                "ok_urls": b.ok_urls,
                "redirects": b.redirects,
                "errors": b.errors,
            })
        })
        .collect();
    data.sort_by(|a, b| {
        let ao = a.get("urls").and_then(|v| v.as_u64()).unwrap_or(0);
        let bo = b.get("urls").and_then(|v| v.as_u64()).unwrap_or(0);
        bo.cmp(&ao)
    });
    data.truncate(limit as usize);
    Ok((
        vec![
            "segment",
            "host",
            "path_prefix",
            "urls",
            "ok_urls",
            "redirects",
            "errors",
        ],
        data,
        None,
    ))
}

async fn report_redirects_all(
    state: &AppState,
    crawl_id: &Uuid,
    limit: i64,
) -> Result<ReportOutput, ApiError> {
    let rows = sqlx::query!(
        r#"
        SELECT url, status_code, redirect_url
        FROM crawl_urls
        WHERE crawl_id = $1 AND status_code BETWEEN 300 AND 399
        ORDER BY url
        LIMIT $2
        "#,
        crawl_id,
        limit,
    )
    .fetch_all(&state.db)
    .await?;
    let data: Vec<Value> = rows
        .into_iter()
        .map(|r| {
            json!({
                "url": r.url,
                "status_code": r.status_code,
                "redirect_url": r.redirect_url,
            })
        })
        .collect();
    Ok((vec!["url", "status_code", "redirect_url"], data, None))
}

async fn report_redirects_to_error(
    state: &AppState,
    crawl_id: &Uuid,
    limit: i64,
) -> Result<ReportOutput, ApiError> {
    // A redirect whose redirect_url resolves to a URL with non-2xx status in this crawl.
    let rows = sqlx::query!(
        r#"
        SELECT src.url AS "src_url!",
               src.status_code AS src_status,
               src.redirect_url AS "redirect_url!",
               dst.status_code AS dst_status
        FROM crawl_urls src
        JOIN crawl_urls dst
          ON dst.crawl_id = src.crawl_id AND dst.url = src.redirect_url
        WHERE src.crawl_id = $1
          AND src.status_code BETWEEN 300 AND 399
          AND (dst.status_code IS NULL OR dst.status_code < 200 OR dst.status_code >= 300)
        ORDER BY src.url
        LIMIT $2
        "#,
        crawl_id,
        limit,
    )
    .fetch_all(&state.db)
    .await?;
    let data: Vec<Value> = rows
        .into_iter()
        .map(|r| {
            json!({
                "source": r.src_url,
                "source_status": r.src_status,
                "redirect_url": r.redirect_url,
                "final_status": r.dst_status,
            })
        })
        .collect();
    Ok((
        vec!["source", "source_status", "redirect_url", "final_status"],
        data,
        None,
    ))
}

async fn report_canonicals_all(
    state: &AppState,
    crawl_id: &Uuid,
    limit: i64,
) -> Result<ReportOutput, ApiError> {
    let rows = sqlx::query!(
        r#"
        SELECT url, canonical_url
        FROM crawl_urls
        WHERE crawl_id = $1 AND canonical_url IS NOT NULL
        ORDER BY url
        LIMIT $2
        "#,
        crawl_id,
        limit,
    )
    .fetch_all(&state.db)
    .await?;
    let data: Vec<Value> = rows
        .into_iter()
        .map(|r| json!({ "url": r.url, "canonical_url": r.canonical_url }))
        .collect();
    Ok((vec!["url", "canonical_url"], data, None))
}

async fn report_canonicals_self(
    state: &AppState,
    crawl_id: &Uuid,
    limit: i64,
) -> Result<ReportOutput, ApiError> {
    let rows = sqlx::query!(
        r#"
        SELECT url, canonical_url
        FROM crawl_urls
        WHERE crawl_id = $1
          AND canonical_url IS NOT NULL
          AND canonical_url = url
        ORDER BY url
        LIMIT $2
        "#,
        crawl_id,
        limit,
    )
    .fetch_all(&state.db)
    .await?;
    let data: Vec<Value> = rows
        .into_iter()
        .map(|r| json!({ "url": r.url, "canonical_url": r.canonical_url }))
        .collect();
    Ok((vec!["url", "canonical_url"], data, None))
}

async fn report_canonicals_non_indexable(
    state: &AppState,
    crawl_id: &Uuid,
    limit: i64,
) -> Result<ReportOutput, ApiError> {
    // canonical points to a URL with non-2xx status or meta_robots containing noindex
    let rows = sqlx::query!(
        r#"
        SELECT src.url  AS "src_url!",
               src.canonical_url AS "canonical_url!",
               dst.status_code   AS dst_status,
               dst.meta_robots   AS dst_robots
        FROM crawl_urls src
        LEFT JOIN crawl_urls dst
          ON dst.crawl_id = src.crawl_id AND dst.url = src.canonical_url
        WHERE src.crawl_id = $1
          AND src.canonical_url IS NOT NULL
          AND (
                dst.status_code IS NULL
             OR dst.status_code < 200
             OR dst.status_code >= 300
             OR (dst.meta_robots IS NOT NULL AND dst.meta_robots ILIKE '%noindex%')
          )
        ORDER BY src.url
        LIMIT $2
        "#,
        crawl_id,
        limit,
    )
    .fetch_all(&state.db)
    .await?;
    let data: Vec<Value> = rows
        .into_iter()
        .map(|r| {
            json!({
                "url": r.src_url,
                "canonical_url": r.canonical_url,
                "canonical_status": r.dst_status,
                "canonical_robots": r.dst_robots,
            })
        })
        .collect();
    Ok((
        vec![
            "url",
            "canonical_url",
            "canonical_status",
            "canonical_robots",
        ],
        data,
        None,
    ))
}

async fn report_pagination_non_200(
    state: &AppState,
    crawl_id: &Uuid,
    limit: i64,
) -> Result<ReportOutput, ApiError> {
    // URLs matching any pagination filter but with non-2xx code
    let rows = sqlx::query!(
        r#"
        SELECT DISTINCT u.url, u.status_code
        FROM crawl_urls u
        JOIN crawl_url_findings f ON f.crawl_url_id = u.id
        WHERE u.crawl_id = $1
          AND f.filter_key LIKE 'pagination%'
          AND (u.status_code IS NULL OR u.status_code < 200 OR u.status_code >= 300)
        ORDER BY u.url
        LIMIT $2
        "#,
        crawl_id,
        limit,
    )
    .fetch_all(&state.db)
    .await?;
    let data: Vec<Value> = rows
        .into_iter()
        .map(|r| json!({ "url": r.url, "status_code": r.status_code }))
        .collect();
    Ok((vec!["url", "status_code"], data, None))
}

async fn report_insecure_content(
    state: &AppState,
    crawl_id: &Uuid,
    limit: i64,
) -> Result<ReportOutput, ApiError> {
    // HTTPS parent page with HTTP outlinks/images/scripts.
    // We inspect crawl_links edges where source is https:// and target is http://
    let rows = sqlx::query!(
        r#"
        SELECT src.url AS "source_url!", dst.url AS "insecure_url!"
        FROM crawl_links l
        JOIN crawl_urls src ON src.id = l.source_url_id
        JOIN crawl_urls dst ON dst.id = l.target_url_id
        WHERE l.crawl_id = $1
          AND src.url ILIKE 'https://%'
          AND dst.url ILIKE 'http://%'
        ORDER BY src.url, dst.url
        LIMIT $2
        "#,
        crawl_id,
        limit,
    )
    .fetch_all(&state.db)
    .await?;
    let data: Vec<Value> = rows
        .into_iter()
        .map(|r| json!({ "source_url": r.source_url, "insecure_url": r.insecure_url }))
        .collect();
    Ok((vec!["source_url", "insecure_url"], data, None))
}

async fn report_serp_summary(
    state: &AppState,
    crawl_id: &Uuid,
    limit: i64,
) -> Result<ReportOutput, ApiError> {
    let rows = sqlx::query!(
        r#"
        SELECT url, title, title_length, title_pixel_width,
               meta_description, meta_description_length
        FROM crawl_urls
        WHERE crawl_id = $1
          AND is_internal = true
          AND content_type = 'html'
          AND status_code BETWEEN 200 AND 299
        ORDER BY url
        LIMIT $2
        "#,
        crawl_id,
        limit,
    )
    .fetch_all(&state.db)
    .await?;
    let data: Vec<Value> = rows
        .into_iter()
        .map(|r| {
            json!({
                "url": r.url,
                "title": r.title,
                "title_length": r.title_length,
                "title_pixel_width": r.title_pixel_width,
                "meta_description": r.meta_description,
                "meta_description_length": r.meta_description_length,
            })
        })
        .collect();
    Ok((
        vec![
            "url",
            "title",
            "title_length",
            "title_pixel_width",
            "meta_description",
            "meta_description_length",
        ],
        data,
        None,
    ))
}

async fn report_orphan_pages(
    state: &AppState,
    crawl_id: &Uuid,
    limit: i64,
) -> Result<ReportOutput, ApiError> {
    // Orphans = internal crawl_urls that have no inlinks (link target) AND are not the seed (depth=0).
    let rows = sqlx::query!(
        r#"
        SELECT u.url, u.depth, u.status_code
        FROM crawl_urls u
        LEFT JOIN crawl_links l
               ON l.target_url_id = u.id AND l.crawl_id = u.crawl_id
        WHERE u.crawl_id = $1
          AND u.is_internal = true
          AND u.depth > 0
          AND l.id IS NULL
        ORDER BY u.url
        LIMIT $2
        "#,
        crawl_id,
        limit,
    )
    .fetch_all(&state.db)
    .await?;
    let data: Vec<Value> = rows
        .into_iter()
        .map(|r| json!({ "url": r.url, "depth": r.depth, "status_code": r.status_code }))
        .collect();
    Ok((vec!["url", "depth", "status_code"], data, None))
}

/// Generic filter-key-backed report: list all URLs with a given finding.
async fn report_finding(
    state: &AppState,
    crawl_id: &Uuid,
    filter_key: &str,
    limit: i64,
) -> Result<ReportOutput, ApiError> {
    let rows = sqlx::query!(
        r#"
        SELECT u.url, u.status_code, u.title, u.title_length
        FROM crawl_urls u
        JOIN crawl_url_findings f ON f.crawl_url_id = u.id
        WHERE u.crawl_id = $1 AND f.filter_key = $2
        ORDER BY u.url
        LIMIT $3
        "#,
        crawl_id,
        filter_key,
        limit,
    )
    .fetch_all(&state.db)
    .await?;
    let data: Vec<Value> = rows
        .into_iter()
        .map(|r| {
            json!({
                "url": r.url,
                "status_code": r.status_code,
                "title": r.title,
                "title_length": r.title_length,
            })
        })
        .collect();
    Ok((
        vec!["url", "status_code", "title", "title_length"],
        data,
        None,
    ))
}

/// All HTTP response headers across the crawl, flattened to one row per
/// (url, header_name, header_value) tuple.
async fn report_http_headers(
    state: &AppState,
    crawl_id: &Uuid,
    limit: i64,
) -> Result<ReportOutput, ApiError> {
    let rows = sqlx::query!(
        r#"
        SELECT url, response_headers
        FROM crawl_urls
        WHERE crawl_id = $1 AND jsonb_array_length(response_headers) > 0
        ORDER BY url
        LIMIT $2
        "#,
        crawl_id,
        limit,
    )
    .fetch_all(&state.db)
    .await?;

    let mut data: Vec<Value> = Vec::new();
    for r in rows {
        if let Some(arr) = r.response_headers.as_array() {
            for pair in arr {
                let Some(p) = pair.as_array() else { continue };
                let name = p.first().and_then(|v| v.as_str()).unwrap_or("");
                let value = p.get(1).and_then(|v| v.as_str()).unwrap_or("");
                data.push(json!({ "url": r.url, "name": name, "value": value }));
            }
        }
    }

    Ok((vec!["url", "name", "value"], data, None))
}

/// Every Set-Cookie seen during the crawl, parsed into SF's Cookies
/// detail columns.
async fn report_cookies(
    state: &AppState,
    crawl_id: &Uuid,
    limit: i64,
) -> Result<ReportOutput, ApiError> {
    let rows = sqlx::query!(
        r#"
        SELECT url, response_headers
        FROM crawl_urls
        WHERE crawl_id = $1 AND jsonb_array_length(response_headers) > 0
        ORDER BY url
        LIMIT $2
        "#,
        crawl_id,
        limit,
    )
    .fetch_all(&state.db)
    .await?;

    let mut data: Vec<Value> = Vec::new();
    for r in rows {
        let Some(arr) = r.response_headers.as_array() else {
            continue;
        };
        for pair in arr {
            let Some(p) = pair.as_array() else { continue };
            let name = p.first().and_then(|v| v.as_str()).unwrap_or("");
            if !name.eq_ignore_ascii_case("set-cookie") {
                continue;
            }
            let raw = p.get(1).and_then(|v| v.as_str()).unwrap_or("");
            let parsed = parse_set_cookie_report(raw);
            data.push(json!({
                "url": r.url,
                "name":      parsed.0,
                "value":     parsed.1,
                "domain":    parsed.2,
                "path":      parsed.3,
                "expires":   parsed.4,
                "secure":    parsed.5,
                "http_only": parsed.6,
                "same_site": parsed.7,
            }));
        }
    }

    Ok((
        vec![
            "url",
            "name",
            "value",
            "domain",
            "path",
            "expires",
            "secure",
            "http_only",
            "same_site",
        ],
        data,
        None,
    ))
}

/// BFS from the seed URL (depth=0) to each discovered internal URL. We follow
/// crawl_links edges and surface the shortest path found by depth. Per SF's
/// Crawl Path report this helps trace how a target URL was discovered.
async fn report_crawl_path(
    state: &AppState,
    crawl_id: &Uuid,
    limit: i64,
) -> Result<ReportOutput, ApiError> {
    let rows = sqlx::query!(
        r#"
        WITH RECURSIVE seeds AS (
            SELECT id, url, depth
            FROM crawl_urls
            WHERE crawl_id = $1 AND depth = 0
        ),
        walk AS (
            SELECT s.id AS url_id,
                   s.url AS url,
                   0::int AS steps,
                   ARRAY[s.url]::text[] AS path
            FROM seeds s
            UNION ALL
            SELECT u.id, u.url, w.steps + 1, w.path || u.url
            FROM walk w
            JOIN crawl_links l ON l.source_url_id = w.url_id AND l.crawl_id = $1
            JOIN crawl_urls u ON u.id = l.target_url_id
            WHERE w.steps < 10
              AND NOT (u.url = ANY(w.path))
        ),
        shortest AS (
            SELECT DISTINCT ON (url_id) url_id, url, steps, path
            FROM walk
            ORDER BY url_id, steps ASC
        )
        SELECT url, steps, path
        FROM shortest
        ORDER BY steps, url
        LIMIT $2
        "#,
        crawl_id,
        limit,
    )
    .fetch_all(&state.db)
    .await?;

    let data: Vec<Value> = rows
        .into_iter()
        .map(|r| {
            json!({
                "url": r.url,
                "steps": r.steps,
                "path": r.path.unwrap_or_default().join(" → "),
            })
        })
        .collect();
    Ok((vec!["url", "steps", "path"], data, None))
}

/// Walk canonical_url references to a terminal node (self-canonical or
/// unresolved). Detects chains of length ≥ 2 and surfaces the full chain.
async fn report_canonical_chains(
    state: &AppState,
    crawl_id: &Uuid,
    limit: i64,
) -> Result<ReportOutput, ApiError> {
    let rows = sqlx::query!(
        r#"
        WITH RECURSIVE chain AS (
            SELECT u.url AS source,
                   u.canonical_url AS next,
                   ARRAY[u.url]::text[] AS path,
                   1::int AS hops
            FROM crawl_urls u
            WHERE u.crawl_id = $1
              AND u.canonical_url IS NOT NULL
              AND u.canonical_url <> u.url
            UNION ALL
            SELECT c.source,
                   n.canonical_url,
                   c.path || n.url,
                   c.hops + 1
            FROM chain c
            JOIN crawl_urls n
              ON n.crawl_id = $1 AND n.url = c.next
            WHERE c.hops < 10
              AND n.canonical_url IS NOT NULL
              AND n.canonical_url <> n.url
              AND NOT (n.url = ANY(c.path))
        ),
        terminated AS (
            SELECT DISTINCT ON (source) source, path, next AS terminal, hops
            FROM chain
            ORDER BY source, hops DESC
        )
        SELECT source, path, terminal, hops
        FROM terminated
        WHERE hops >= 2
        ORDER BY hops DESC, source
        LIMIT $2
        "#,
        crawl_id,
        limit,
    )
    .fetch_all(&state.db)
    .await?;

    let data: Vec<Value> = rows
        .into_iter()
        .map(|r| {
            let mut chain = r.path.unwrap_or_default();
            if let Some(t) = r.terminal.as_ref() {
                chain.push(t.clone());
            }
            json!({
                "source": r.source,
                "hops": r.hops,
                "terminal": r.terminal,
                "chain": chain.join(" → "),
            })
        })
        .collect();
    Ok((vec!["source", "hops", "terminal", "chain"], data, None))
}

/// Multi-hop redirect chains (3xx → 3xx → … → terminal).
async fn report_redirect_chains(
    state: &AppState,
    crawl_id: &Uuid,
    limit: i64,
) -> Result<ReportOutput, ApiError> {
    let rows = sqlx::query!(
        r#"
        WITH RECURSIVE chain AS (
            SELECT u.url AS source,
                   u.redirect_url AS next,
                   ARRAY[u.url]::text[] AS path,
                   1::int AS hops,
                   u.status_code AS start_status
            FROM crawl_urls u
            WHERE u.crawl_id = $1
              AND u.status_code BETWEEN 300 AND 399
              AND u.redirect_url IS NOT NULL
            UNION ALL
            SELECT c.source,
                   n.redirect_url,
                   c.path || n.url,
                   c.hops + 1,
                   c.start_status
            FROM chain c
            JOIN crawl_urls n
              ON n.crawl_id = $1 AND n.url = c.next
            WHERE c.hops < 10
              AND n.status_code BETWEEN 300 AND 399
              AND n.redirect_url IS NOT NULL
              AND NOT (n.url = ANY(c.path))
        ),
        terminated AS (
            SELECT DISTINCT ON (source) source, path, next AS terminal, hops, start_status
            FROM chain
            ORDER BY source, hops DESC
        )
        SELECT t.source,
               t.path,
               t.terminal,
               t.hops,
               t.start_status,
               dst.status_code AS terminal_status
        FROM terminated t
        LEFT JOIN crawl_urls dst ON dst.crawl_id = $1 AND dst.url = t.terminal
        WHERE t.hops >= 2
        ORDER BY t.hops DESC, t.source
        LIMIT $2
        "#,
        crawl_id,
        limit,
    )
    .fetch_all(&state.db)
    .await?;

    let data: Vec<Value> = rows
        .into_iter()
        .map(|r| {
            let mut chain = r.path.unwrap_or_default();
            if let Some(t) = r.terminal.as_ref() {
                chain.push(t.clone());
            }
            json!({
                "source": r.source,
                "hops": r.hops,
                "start_status": r.start_status,
                "terminal": r.terminal,
                "terminal_status": r.terminal_status,
                "chain": chain.join(" → "),
            })
        })
        .collect();
    Ok((
        vec![
            "source",
            "hops",
            "start_status",
            "terminal",
            "terminal_status",
            "chain",
        ],
        data,
        None,
    ))
}

/// Walk the combined redirect OR canonical chain — SF's "Redirect &
/// Canonical Chains" report. A node is followed via redirect_url if the
/// current URL is a 3xx, otherwise via canonical_url when present and
/// different from self.
async fn report_redirect_and_canonical_chains(
    state: &AppState,
    crawl_id: &Uuid,
    limit: i64,
) -> Result<ReportOutput, ApiError> {
    let rows = sqlx::query!(
        r#"
        WITH RECURSIVE chain AS (
            SELECT u.url AS source,
                   CASE
                     WHEN u.status_code BETWEEN 300 AND 399 THEN u.redirect_url
                     WHEN u.canonical_url IS NOT NULL AND u.canonical_url <> u.url THEN u.canonical_url
                     ELSE NULL
                   END AS next,
                   ARRAY[u.url]::text[] AS path,
                   1::int AS hops,
                   CASE
                     WHEN u.status_code BETWEEN 300 AND 399 THEN 'redirect'
                     WHEN u.canonical_url IS NOT NULL AND u.canonical_url <> u.url THEN 'canonical'
                     ELSE 'none'
                   END::text AS kind
            FROM crawl_urls u
            WHERE u.crawl_id = $1
              AND (
                (u.status_code BETWEEN 300 AND 399 AND u.redirect_url IS NOT NULL)
                OR (u.canonical_url IS NOT NULL AND u.canonical_url <> u.url)
              )
            UNION ALL
            SELECT c.source,
                   CASE
                     WHEN n.status_code BETWEEN 300 AND 399 THEN n.redirect_url
                     WHEN n.canonical_url IS NOT NULL AND n.canonical_url <> n.url THEN n.canonical_url
                     ELSE NULL
                   END,
                   c.path || n.url,
                   c.hops + 1,
                   c.kind
            FROM chain c
            JOIN crawl_urls n
              ON n.crawl_id = $1 AND n.url = c.next
            WHERE c.hops < 10
              AND c.next IS NOT NULL
              AND NOT (n.url = ANY(c.path))
              AND (
                (n.status_code BETWEEN 300 AND 399 AND n.redirect_url IS NOT NULL)
                OR (n.canonical_url IS NOT NULL AND n.canonical_url <> n.url)
              )
        ),
        terminated AS (
            SELECT DISTINCT ON (source) source, path, next AS terminal, hops
            FROM chain
            ORDER BY source, hops DESC
        )
        SELECT source, path, terminal, hops
        FROM terminated
        WHERE hops >= 2
        ORDER BY hops DESC, source
        LIMIT $2
        "#,
        crawl_id,
        limit,
    )
    .fetch_all(&state.db)
    .await?;

    let data: Vec<Value> = rows
        .into_iter()
        .map(|r| {
            let mut chain = r.path.unwrap_or_default();
            if let Some(t) = r.terminal.as_ref() {
                chain.push(t.clone());
            }
            json!({
                "source": r.source,
                "hops": r.hops,
                "terminal": r.terminal,
                "chain": chain.join(" → "),
            })
        })
        .collect();
    Ok((vec!["source", "hops", "terminal", "chain"], data, None))
}

/// Aggregate finding counts under a filter-key prefix (e.g. "mobile",
/// "accessibility"). Mirrors SF's category "Summary" report which groups
/// violations by rule.
async fn report_category_summary(
    state: &AppState,
    crawl_id: &Uuid,
    prefix: &str,
    limit: i64,
) -> Result<ReportOutput, ApiError> {
    let pattern = format!("{}_%", prefix);
    let rows = sqlx::query!(
        r#"
        SELECT filter_key, COUNT(*) AS "count!",
               COUNT(DISTINCT crawl_url_id) AS "urls!"
        FROM crawl_url_findings
        WHERE crawl_id = $1 AND filter_key LIKE $2
        GROUP BY filter_key
        ORDER BY COUNT(*) DESC, filter_key
        LIMIT $3
        "#,
        crawl_id,
        pattern,
        limit,
    )
    .fetch_all(&state.db)
    .await?;

    let data: Vec<Value> = rows
        .into_iter()
        .map(|r| json!({ "filter_key": r.filter_key, "count": r.count, "urls": r.urls }))
        .collect();
    Ok((vec!["filter_key", "count", "urls"], data, None))
}

/// Counts each response-header name across the crawl — SF's HTTP Header
/// Summary. Name matching is case-insensitive to fold the usual header
/// capitalization variants together.
async fn report_http_headers_summary(
    state: &AppState,
    crawl_id: &Uuid,
    limit: i64,
) -> Result<ReportOutput, ApiError> {
    let rows = sqlx::query!(
        r#"
        SELECT response_headers
        FROM crawl_urls
        WHERE crawl_id = $1 AND jsonb_array_length(response_headers) > 0
        "#,
        crawl_id,
    )
    .fetch_all(&state.db)
    .await?;

    use std::collections::BTreeMap;
    let mut counts: BTreeMap<String, (u64, u64)> = BTreeMap::new();
    for r in rows {
        let Some(arr) = r.response_headers.as_array() else {
            continue;
        };
        let mut seen_on_url: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        for pair in arr {
            let Some(p) = pair.as_array() else { continue };
            let name = p
                .first()
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            if name.is_empty() {
                continue;
            }
            let entry = counts.entry(name.clone()).or_insert((0, 0));
            entry.0 += 1;
            if seen_on_url.insert(name) {
                entry.1 += 1;
            }
        }
    }

    let mut data: Vec<Value> = counts
        .into_iter()
        .map(|(name, (occurrences, urls))| {
            json!({ "header": name, "occurrences": occurrences, "urls": urls })
        })
        .collect();
    data.sort_by(|a, b| {
        let ao = a.get("occurrences").and_then(|v| v.as_u64()).unwrap_or(0);
        let bo = b.get("occurrences").and_then(|v| v.as_u64()).unwrap_or(0);
        bo.cmp(&ao)
    });
    data.truncate(limit as usize);
    Ok((vec!["header", "occurrences", "urls"], data, None))
}

/// Distinct cookies seen (grouped by name + domain) with per-cookie issue
/// flags (Secure / HttpOnly / SameSite=None). Matches SF's Cookie Summary.
async fn report_cookies_summary(
    state: &AppState,
    crawl_id: &Uuid,
    limit: i64,
) -> Result<ReportOutput, ApiError> {
    let rows = sqlx::query!(
        r#"
        SELECT response_headers
        FROM crawl_urls
        WHERE crawl_id = $1 AND jsonb_array_length(response_headers) > 0
        "#,
        crawl_id,
    )
    .fetch_all(&state.db)
    .await?;

    #[derive(Default)]
    struct Agg {
        urls: u64,
        secure_count: u64,
        http_only_count: u64,
        samesite_none_count: u64,
    }
    use std::collections::BTreeMap;
    let mut summary: BTreeMap<(String, String), Agg> = BTreeMap::new();
    for r in rows {
        let Some(arr) = r.response_headers.as_array() else {
            continue;
        };
        for pair in arr {
            let Some(p) = pair.as_array() else { continue };
            let name = p.first().and_then(|v| v.as_str()).unwrap_or("");
            if !name.eq_ignore_ascii_case("set-cookie") {
                continue;
            }
            let raw = p.get(1).and_then(|v| v.as_str()).unwrap_or("");
            let parsed = parse_set_cookie_report(raw);
            let cookie_name = parsed.0;
            let domain = parsed.2.unwrap_or_default();
            let entry = summary.entry((cookie_name, domain)).or_default();
            entry.urls += 1;
            if parsed.5 {
                entry.secure_count += 1;
            }
            if parsed.6 {
                entry.http_only_count += 1;
            }
            if matches!(parsed.7.as_deref(), Some(s) if s.eq_ignore_ascii_case("None")) {
                entry.samesite_none_count += 1;
            }
        }
    }

    let mut data: Vec<Value> = summary
        .into_iter()
        .map(|((name, domain), agg)| {
            json!({
                "name": name,
                "domain": domain,
                "occurrences": agg.urls,
                "secure": agg.secure_count,
                "http_only": agg.http_only_count,
                "samesite_none": agg.samesite_none_count,
            })
        })
        .collect();
    data.sort_by(|a, b| {
        let ao = a.get("occurrences").and_then(|v| v.as_u64()).unwrap_or(0);
        let bo = b.get("occurrences").and_then(|v| v.as_u64()).unwrap_or(0);
        bo.cmp(&ao)
    });
    data.truncate(limit as usize);
    Ok((
        vec![
            "name",
            "domain",
            "occurrences",
            "secure",
            "http_only",
            "samesite_none",
        ],
        data,
        None,
    ))
}

/// Cheap Set-Cookie parser — duplicated here to keep `reports` standalone.
/// Returns (name, value, domain, path, expires, secure, http_only, same_site).
type ParsedCookie = (
    String,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    bool,
    bool,
    Option<String>,
);
fn parse_set_cookie_report(raw: &str) -> ParsedCookie {
    let mut parts = raw.split(';').map(str::trim);
    let first = parts.next().unwrap_or("");
    let (name, value) = first.split_once('=').unwrap_or((first, ""));
    let (mut domain, mut path, mut expires, mut same_site) = (None, None, None, None);
    let (mut secure, mut http_only) = (false, false);
    for attr in parts {
        if attr.is_empty() {
            continue;
        }
        let (k, v) = attr
            .split_once('=')
            .map(|(k, v)| (k.trim(), v.trim()))
            .unwrap_or((attr, ""));
        match k.to_ascii_lowercase().as_str() {
            "domain" => domain = Some(v.to_string()),
            "path" => path = Some(v.to_string()),
            "expires" => expires = Some(v.to_string()),
            "secure" => secure = true,
            "httponly" => http_only = true,
            "samesite" => same_site = Some(v.to_string()),
            _ => {}
        }
    }
    (
        name.to_string(),
        value.to_string(),
        domain,
        path,
        expires,
        secure,
        http_only,
        same_site,
    )
}
