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
//! (http-headers, cookies, rendered-page, console-log, resources)
//! return the envelope with `rows: []` and `notes` explaining the
//! missing capture so the UI can degrade gracefully.

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
    // Accessibility
    ReportDef {
        key: "accessibility_all",
        title: "Accessibility: All",
        group: "Accessibility",
    },
    // Deferred — crawler must capture headers/cookies first
    ReportDef {
        key: "http_headers_all",
        title: "HTTP Headers: All",
        group: "HTTP Headers",
    },
    ReportDef {
        key: "cookies_all",
        title: "Cookies: All",
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
        "segments_overview" => Ok((
            vec!["segment", "urls"],
            vec![],
            Some("Segments feature not yet implemented".into()),
        )),
        "site_structure" => report_site_structure(state, crawl_id, limit).await,

        "redirects_all" => report_redirects_all(state, crawl_id, limit).await,
        "redirect_chains" => Ok((
            vec!["source", "chain", "final_status"],
            vec![],
            Some("Multi-hop chains require crawler to persist redirect chain; single-hop redirects available via 'redirects_all'".into()),
        )),
        "redirect_and_canonical_chains" => Ok((
            vec!["source", "chain", "final_status"],
            vec![],
            Some("Requires redirect-chain + canonical-chain persistence (future crawler work)".into()),
        )),
        "redirects_to_error" => report_redirects_to_error(state, crawl_id, limit).await,

        "canonicals_all" => report_canonicals_all(state, crawl_id, limit).await,
        "canonicals_missing" => report_finding(state, crawl_id, "canonicals_missing", limit).await,
        "canonicals_self_referencing" => report_canonicals_self(state, crawl_id, limit).await,
        "canonicals_non_indexable" => report_canonicals_non_indexable(state, crawl_id, limit).await,

        "pagination_non_200" => report_pagination_non_200(state, crawl_id, limit).await,
        "pagination_unlinked" => report_finding(state, crawl_id, "pagination_unlinked_pages", limit).await,

        "hreflang_missing" => report_finding(state, crawl_id, "hreflang_missing", limit).await,
        "hreflang_inconsistent" => report_finding(state, crawl_id, "hreflang_inconsistent_language_links", limit).await,
        "hreflang_non_canonical" => report_finding(state, crawl_id, "hreflang_non_canonical", limit).await,

        "insecure_content" => report_insecure_content(state, crawl_id, limit).await,
        "serp_summary" => report_serp_summary(state, crawl_id, limit).await,
        "orphan_pages" => report_orphan_pages(state, crawl_id, limit).await,

        "structured_data_all" => report_finding(state, crawl_id, "structured_data_all", limit).await,
        "structured_data_errors" => report_finding(state, crawl_id, "structured_data_parse_errors", limit).await,

        "javascript_all" => report_finding(state, crawl_id, "javascript_all", limit).await,
        "javascript_containing_js_content" => report_finding(state, crawl_id, "javascript_contains_js_content", limit).await,

        "pagespeed_all" => report_finding(state, crawl_id, "pagespeed_all", limit).await,

        "mobile_all" => report_finding(state, crawl_id, "mobile_all", limit).await,
        "accessibility_all" => report_finding(state, crawl_id, "accessibility_all", limit).await,

        "http_headers_all" => report_http_headers(state, crawl_id, limit).await,
        "cookies_all" => report_cookies(state, crawl_id, limit).await,

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
