use axum::extract::{Path, Query, State};
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;
use url::Url;
use uuid::Uuid;

use crate::app_state::AppState;
use crate::error::ApiError;
use crate::extractors::auth::AuthUser;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/crawls/{crawl_id}/urls", get(list_urls))
        .route("/crawls/{crawl_id}/urls/{url_id}", get(get_url_detail))
        .route("/crawls/{crawl_id}/urls/{url_id}/inlinks", get(get_inlinks))
        .route("/crawls/{crawl_id}/urls/{url_id}/outlinks", get(get_outlinks))
        .route("/crawls/{crawl_id}/urls/{url_id}/images", get(get_images))
        .route("/crawls/{crawl_id}/urls/{url_id}/resources", get(get_resources))
        .route("/crawls/{crawl_id}/urls/{url_id}/serp", get(get_serp))
        .route("/crawls/{crawl_id}/urls/{url_id}/headers", get(get_headers))
        .route("/crawls/{crawl_id}/urls/{url_id}/cookies", get(get_cookies))
        .route("/crawls/{crawl_id}/urls/{url_id}/source", get(get_source))
        .route("/crawls/{crawl_id}/urls/{url_id}/duplicates", get(get_duplicates))
        .route("/crawls/{crawl_id}/urls/{url_id}/structured-data", get(get_structured_data))
        .route("/crawls/{crawl_id}/overview", get(get_overview))
}

#[derive(Debug, Deserialize)]
pub struct UrlsQuery {
    pub tab: Option<String>,
    pub filter: Option<String>,
    pub q: Option<String>,
    pub cursor: Option<String>,
    pub limit: Option<i64>,
}

async fn list_urls(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(crawl_id): Path<Uuid>,
    Query(query): Query<UrlsQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    verify_crawl_ownership(&state, &auth, &crawl_id).await?;

    let limit = query.limit.unwrap_or(50).min(200);

    if let Some(ref filter_key) = query.filter {
        let rows = sqlx::query!(
            r#"
            SELECT u.id, u.url, u.status_code, u.content_type,
                   u.is_internal, u.depth, u.title, u.title_length,
                   u.h1_first, u.h1_count, u.word_count,
                   u.response_time_ms, u.content_length, u.crawled_at
            FROM crawl_urls u
            JOIN crawl_url_findings f ON f.crawl_url_id = u.id
            WHERE u.crawl_id = $1 AND f.filter_key = $2
            ORDER BY u.url
            LIMIT $3
            "#,
            &crawl_id,
            filter_key,
            limit,
        )
        .fetch_all(&state.db)
        .await?;

        let urls: Vec<serde_json::Value> = rows
            .into_iter()
            .map(|r| {
                serde_json::json!({
                    "id": r.id,
                    "url": r.url,
                    "status_code": r.status_code,
                    "content_type": r.content_type,
                    "is_internal": r.is_internal,
                    "depth": r.depth,
                    "title": r.title,
                    "title_length": r.title_length,
                    "h1_first": r.h1_first,
                    "h1_count": r.h1_count,
                    "word_count": r.word_count,
                    "response_time_ms": r.response_time_ms,
                    "content_length": r.content_length,
                    "crawled_at": r.crawled_at,
                })
            })
            .collect();

        return Ok(Json(serde_json::json!({
            "data": urls,
            "next_cursor": serde_json::Value::Null,
        })));
    }

    let rows = sqlx::query!(
        r#"
        SELECT id, url, status_code, content_type,
               is_internal, depth, title, title_length,
               h1_first, h1_count, word_count,
               response_time_ms, content_length, crawled_at
        FROM crawl_urls
        WHERE crawl_id = $1
        ORDER BY url
        LIMIT $2
        "#,
        &crawl_id,
        limit,
    )
    .fetch_all(&state.db)
    .await?;

    let urls: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|r| {
            serde_json::json!({
                "id": r.id,
                "url": r.url,
                "status_code": r.status_code,
                "content_type": r.content_type,
                "is_internal": r.is_internal,
                "depth": r.depth,
                "title": r.title,
                "title_length": r.title_length,
                "h1_first": r.h1_first,
                "h1_count": r.h1_count,
                "word_count": r.word_count,
                "response_time_ms": r.response_time_ms,
                "content_length": r.content_length,
                "crawled_at": r.crawled_at,
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "data": urls,
        "next_cursor": serde_json::Value::Null,
    })))
}

async fn get_url_detail(
    auth: AuthUser,
    State(state): State<AppState>,
    Path((crawl_id, url_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    verify_crawl_ownership(&state, &auth, &crawl_id).await?;

    let row = sqlx::query!(
        r#"
        SELECT id, url, status_code, content_type,
               is_internal, depth, title, title_length, title_pixel_width,
               meta_description, meta_description_length,
               h1_first, h1_count, h2_first, h2_count,
               word_count, response_time_ms, content_length,
               redirect_url, canonical_url, meta_robots, crawled_at
        FROM crawl_urls
        WHERE id = $1 AND crawl_id = $2
        "#,
        &url_id,
        &crawl_id,
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::not_found("URL not found"))?;

    let findings = sqlx::query!(
        "SELECT filter_key FROM crawl_url_findings WHERE crawl_url_id = $1",
        &url_id,
    )
    .fetch_all(&state.db)
    .await?;

    let filter_keys: Vec<String> = findings.into_iter().map(|f| f.filter_key).collect();

    Ok(Json(serde_json::json!({
        "id": row.id,
        "url": row.url,
        "status_code": row.status_code,
        "content_type": row.content_type,
        "is_internal": row.is_internal,
        "depth": row.depth,
        "title": row.title,
        "title_length": row.title_length,
        "title_pixel_width": row.title_pixel_width,
        "meta_description": row.meta_description,
        "meta_description_length": row.meta_description_length,
        "h1_first": row.h1_first,
        "h1_count": row.h1_count,
        "h2_first": row.h2_first,
        "h2_count": row.h2_count,
        "word_count": row.word_count,
        "response_time_ms": row.response_time_ms,
        "content_length": row.content_length,
        "redirect_url": row.redirect_url,
        "canonical_url": row.canonical_url,
        "meta_robots": row.meta_robots,
        "crawled_at": row.crawled_at,
        "findings": filter_keys,
    })))
}

async fn get_inlinks(
    auth: AuthUser,
    State(state): State<AppState>,
    Path((crawl_id, url_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    verify_crawl_ownership(&state, &auth, &crawl_id).await?;

    let rows = sqlx::query!(
        r#"
        SELECT l.source_url_id, u.url as source_url, l.anchor_text, l.link_type
        FROM crawl_links l
        JOIN crawl_urls u ON u.id = l.source_url_id
        WHERE l.target_url_id = $1 AND l.crawl_id = $2
        "#,
        &url_id,
        &crawl_id,
    )
    .fetch_all(&state.db)
    .await?;

    let links: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|r| {
            serde_json::json!({
                "source_url_id": r.source_url_id,
                "source_url": r.source_url,
                "anchor_text": r.anchor_text,
                "link_type": r.link_type,
            })
        })
        .collect();

    Ok(Json(serde_json::json!(links)))
}

async fn get_outlinks(
    auth: AuthUser,
    State(state): State<AppState>,
    Path((crawl_id, url_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    verify_crawl_ownership(&state, &auth, &crawl_id).await?;

    let rows = sqlx::query!(
        r#"
        SELECT l.target_url_id, u.url as target_url, l.anchor_text, l.link_type
        FROM crawl_links l
        JOIN crawl_urls u ON u.id = l.target_url_id
        WHERE l.source_url_id = $1 AND l.crawl_id = $2
        "#,
        &url_id,
        &crawl_id,
    )
    .fetch_all(&state.db)
    .await?;

    let links: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|r| {
            serde_json::json!({
                "target_url_id": r.target_url_id,
                "target_url": r.target_url,
                "anchor_text": r.anchor_text,
                "link_type": r.link_type,
            })
        })
        .collect();

    Ok(Json(serde_json::json!(links)))
}

/// Feeds Screaming Frog's "Image Details" detail sub-tab — every image
/// referenced from this URL (linked via <img src>, etc). We approximate
/// "referenced images" as outlinks whose target was classified as an image
/// by the content-type sniffer. Columns match what SF shows.
async fn get_images(
    auth: AuthUser,
    State(state): State<AppState>,
    Path((crawl_id, url_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    verify_crawl_ownership(&state, &auth, &crawl_id).await?;
    let rows = sqlx::query!(
        r#"
        SELECT u.id, u.url, u.status_code, u.content_type,
               u.content_length, u.response_time_ms, l.anchor_text
        FROM crawl_links l
        JOIN crawl_urls u ON u.id = l.target_url_id
        WHERE l.source_url_id = $1
          AND l.crawl_id = $2
          AND u.content_type = 'image'
        ORDER BY u.url
        "#,
        &url_id,
        &crawl_id,
    )
    .fetch_all(&state.db)
    .await?;

    let images: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|r| {
            serde_json::json!({
                "id": r.id,
                "url": r.url,
                "status_code": r.status_code,
                "content_type": r.content_type,
                "size_bytes": r.content_length,
                "response_time_ms": r.response_time_ms,
                "alt_text": r.anchor_text,
            })
        })
        .collect();

    Ok(Json(serde_json::json!(images)))
}

// Feeds the "Resources" per-URL detail tab. Returns every script /
// stylesheet / image URL the page loaded, whether or not we actually
// fetched the asset ourselves — SF's Resources tab is a pure inventory of
// what the HTML referenced. Grouped into per-type buckets so the UI can
// render a tabbed table without client-side filtering.
async fn get_resources(
    auth: AuthUser,
    State(state): State<AppState>,
    Path((crawl_id, url_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    verify_crawl_ownership(&state, &auth, &crawl_id).await?;

    let page_url: Option<String> = sqlx::query_scalar!(
        "SELECT url FROM crawl_urls WHERE id = $1 AND crawl_id = $2",
        &url_id,
        &crawl_id,
    )
    .fetch_optional(&state.db)
    .await?;
    let page_url = page_url.ok_or_else(|| ApiError::not_found("url not found"))?;

    let rows = sqlx::query!(
        r#"
        SELECT url, resource_type
        FROM crawl_url_resources
        WHERE source_url_id = $1 AND crawl_id = $2
        ORDER BY resource_type, url
        "#,
        &url_id,
        &crawl_id,
    )
    .fetch_all(&state.db)
    .await?;

    let mut counts_by_type: std::collections::BTreeMap<String, i64> =
        std::collections::BTreeMap::new();
    let mut resources: Vec<serde_json::Value> = Vec::with_capacity(rows.len());
    for r in rows {
        *counts_by_type.entry(r.resource_type.clone()).or_insert(0) += 1;
        resources.push(serde_json::json!({
            "url": r.url,
            "type": r.resource_type,
        }));
    }

    Ok(Json(serde_json::json!({
        "url": page_url,
        "count": resources.len(),
        "counts_by_type": counts_by_type,
        "resources": resources,
    })))
}

/// Feeds Screaming Frog's "SERP Snippet" detail sub-tab. Returns the raw
/// fields plus SF's derived measurements (length, pixel width, display
/// truncation) for title + description so the UI can render a live preview
/// matching the Google SERP layout.
async fn get_serp(
    auth: AuthUser,
    State(state): State<AppState>,
    Path((crawl_id, url_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    verify_crawl_ownership(&state, &auth, &crawl_id).await?;
    let row = sqlx::query!(
        r#"
        SELECT url, title, title_length, title_pixel_width,
               meta_description, meta_description_length
        FROM crawl_urls
        WHERE id = $1 AND crawl_id = $2
        "#,
        &url_id,
        &crawl_id,
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::not_found("URL not found"))?;

    // SERP layout thresholds taken from Screaming Frog defaults.
    const TITLE_MAX_PIXELS: i32 = 567;
    const DESC_MAX_PIXELS: i32 = 1020;
    const TITLE_MAX_CHARS: i32 = 55;
    const DESC_MAX_CHARS: i32 = 150;

    let title_px = row.title_pixel_width.unwrap_or(0);
    let title_len = row.title_length.unwrap_or(0);
    let desc_len = row.meta_description_length.unwrap_or(0);
    // We don't persist desc pixel width yet; approximate as chars × 7 as a
    // lightweight proxy. This matches SF's eyeballed average for
    // proportional fonts closely enough for UI truncation previews.
    let desc_px = desc_len * 7;

    let title_remaining_chars = TITLE_MAX_CHARS - title_len;
    let title_remaining_px = TITLE_MAX_PIXELS - title_px;
    let desc_remaining_chars = DESC_MAX_CHARS - desc_len;
    let desc_remaining_px = DESC_MAX_PIXELS - desc_px;

    let breadcrumb: Vec<String> = Url::parse(&row.url)
        .ok()
        .map(|u| {
            let host = u.host_str().unwrap_or("").to_string();
            let segs: Vec<String> = u
                .path_segments()
                .map(|it| it.filter(|s| !s.is_empty()).map(String::from).collect())
                .unwrap_or_default();
            let mut out = vec![host];
            out.extend(segs);
            out
        })
        .unwrap_or_default();

    Ok(Json(serde_json::json!({
        "url": row.url,
        "breadcrumb": breadcrumb,
        "title": {
            "value": row.title,
            "length_chars": title_len,
            "length_pixels": title_px,
            "max_chars": TITLE_MAX_CHARS,
            "max_pixels": TITLE_MAX_PIXELS,
            "remaining_chars": title_remaining_chars,
            "remaining_pixels": title_remaining_px,
            "truncated": title_px > TITLE_MAX_PIXELS,
        },
        "description": {
            "value": row.meta_description,
            "length_chars": desc_len,
            "length_pixels_approx": desc_px,
            "max_chars": DESC_MAX_CHARS,
            "max_pixels": DESC_MAX_PIXELS,
            "remaining_chars": desc_remaining_chars,
            "remaining_pixels": desc_remaining_px,
            "truncated": desc_px > DESC_MAX_PIXELS,
        }
    })))
}

/// Feeds Screaming Frog's "HTTP Headers" detail sub-tab. Returns the raw
/// response headers (in arrival order) plus the post-redirect URL.
async fn get_headers(
    auth: AuthUser,
    State(state): State<AppState>,
    Path((crawl_id, url_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    verify_crawl_ownership(&state, &auth, &crawl_id).await?;
    let row = sqlx::query!(
        r#"
        SELECT url, final_url, status_code, response_headers
        FROM crawl_urls
        WHERE id = $1 AND crawl_id = $2
        "#,
        &url_id,
        &crawl_id,
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::not_found("URL not found"))?;

    let headers: Vec<serde_json::Value> = row
        .response_headers
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|pair| {
            let arr = pair.as_array()?;
            Some(serde_json::json!({
                "name": arr.first().and_then(|v| v.as_str()).unwrap_or(""),
                "value": arr.get(1).and_then(|v| v.as_str()).unwrap_or(""),
            }))
        })
        .collect();

    Ok(Json(serde_json::json!({
        "url": row.url,
        "final_url": row.final_url,
        "status_code": row.status_code,
        "header_count": headers.len(),
        "headers": headers,
    })))
}

/// Feeds Screaming Frog's "Cookies" detail sub-tab. Parses every
/// `Set-Cookie` response header into name / value / domain / path /
/// expires / max_age / secure / http_only / same_site so the UI can
/// render one row per cookie.
async fn get_cookies(
    auth: AuthUser,
    State(state): State<AppState>,
    Path((crawl_id, url_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    verify_crawl_ownership(&state, &auth, &crawl_id).await?;
    let row = sqlx::query!(
        r#"
        SELECT url, response_headers
        FROM crawl_urls
        WHERE id = $1 AND crawl_id = $2
        "#,
        &url_id,
        &crawl_id,
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::not_found("URL not found"))?;

    let cookies: Vec<serde_json::Value> = row
        .response_headers
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|pair| {
            let arr = pair.as_array()?;
            let name = arr.first()?.as_str()?;
            if !name.eq_ignore_ascii_case("set-cookie") {
                return None;
            }
            let raw = arr.get(1)?.as_str()?;
            Some(parse_set_cookie(raw))
        })
        .collect();

    Ok(Json(serde_json::json!({
        "url": row.url,
        "count": cookies.len(),
        "cookies": cookies,
    })))
}

/// Cheap hand-rolled `Set-Cookie` parser that returns everything SF's
/// Cookies detail tab shows. Accepts the raw header value (what comes
/// after "Set-Cookie:" with no further unfolding) and produces a JSON
/// object. Unknown attributes are silently dropped.
fn parse_set_cookie(raw: &str) -> serde_json::Value {
    let mut parts = raw.split(';').map(str::trim);
    let Some(first) = parts.next() else {
        return serde_json::json!({ "raw": raw });
    };
    let (name, value) = first.split_once('=').unwrap_or((first, ""));

    let mut domain: Option<String> = None;
    let mut path: Option<String> = None;
    let mut expires: Option<String> = None;
    let mut max_age: Option<i64> = None;
    let mut secure = false;
    let mut http_only = false;
    let mut same_site: Option<String> = None;

    for attr in parts {
        if attr.is_empty() { continue; }
        let (k, v) = attr.split_once('=').map(|(k, v)| (k.trim(), v.trim())).unwrap_or((attr, ""));
        match k.to_ascii_lowercase().as_str() {
            "domain"   => domain = Some(v.to_string()),
            "path"     => path = Some(v.to_string()),
            "expires"  => expires = Some(v.to_string()),
            "max-age"  => max_age = v.parse().ok(),
            "secure"   => secure = true,
            "httponly" => http_only = true,
            "samesite" => same_site = Some(v.to_string()),
            _ => {}
        }
    }

    serde_json::json!({
        "name": name,
        "value": value,
        "domain": domain,
        "path": path,
        "expires": expires,
        "max_age": max_age,
        "secure": secure,
        "http_only": http_only,
        "same_site": same_site,
        "raw": raw,
    })
}

/// Feeds Screaming Frog's "View Source" detail tab. Returns the raw
/// HTML body the crawler stored verbatim. For non-HTML responses (images,
/// binary, redirects with empty bodies) `html` is null.
async fn get_source(
    auth: AuthUser,
    State(state): State<AppState>,
    Path((crawl_id, url_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    verify_crawl_ownership(&state, &auth, &crawl_id).await?;
    let row = sqlx::query!(
        r#"
        SELECT url, content_type, content_length, raw_html, content_hash
        FROM crawl_urls
        WHERE id = $1 AND crawl_id = $2
        "#,
        &url_id,
        &crawl_id,
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::not_found("URL not found"))?;

    let html_len = row.raw_html.as_deref().map(|s| s.len()).unwrap_or(0);
    let hash_hex = row
        .content_hash
        .as_deref()
        .map(|b| b.iter().map(|x| format!("{x:02x}")).collect::<String>());

    Ok(Json(serde_json::json!({
        "url": row.url,
        "content_type": row.content_type,
        "content_length": row.content_length,
        "html_length": html_len,
        "content_hash": hash_hex,
        "html": row.raw_html,
    })))
}

/// Feeds Screaming Frog's "Duplicate Details" detail tab. Two URLs are
/// exact duplicates when their normalised content hashes match. Returns
/// the current URL's hash plus every other crawled URL in the same
/// crawl that shares it (exact — not near-duplicate).
async fn get_duplicates(
    auth: AuthUser,
    State(state): State<AppState>,
    Path((crawl_id, url_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    verify_crawl_ownership(&state, &auth, &crawl_id).await?;
    let row = sqlx::query!(
        r#"
        SELECT url, content_hash
        FROM crawl_urls
        WHERE id = $1 AND crawl_id = $2
        "#,
        &url_id,
        &crawl_id,
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::not_found("URL not found"))?;

    let Some(hash) = row.content_hash.as_deref() else {
        return Ok(Json(serde_json::json!({
            "url": row.url,
            "content_hash": null,
            "match_type": "exact",
            "count": 0,
            "duplicates": [],
        })));
    };

    let dupes = sqlx::query!(
        r#"
        SELECT id, url
        FROM crawl_urls
        WHERE crawl_id = $1 AND content_hash = $2 AND id <> $3
        ORDER BY url ASC
        "#,
        &crawl_id,
        hash,
        &url_id,
    )
    .fetch_all(&state.db)
    .await?;

    let hash_hex: String = hash.iter().map(|x| format!("{x:02x}")).collect();
    let items: Vec<serde_json::Value> = dupes
        .into_iter()
        .map(|r| serde_json::json!({ "url_id": r.id, "url": r.url, "similarity": 1.0 }))
        .collect();

    Ok(Json(serde_json::json!({
        "url": row.url,
        "content_hash": hash_hex,
        "match_type": "exact",
        "count": items.len(),
        "duplicates": items,
    })))
}

/// Feeds Screaming Frog's "Structured Data" detail tab. Returns every
/// extracted structured-data block for the URL. Only JSON-LD is wired
/// for now — Microdata / RDFa remain empty arrays until we pick a
/// parser (tracked in the master reference doc under Batch D).
async fn get_structured_data(
    auth: AuthUser,
    State(state): State<AppState>,
    Path((crawl_id, url_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    verify_crawl_ownership(&state, &auth, &crawl_id).await?;
    let row = sqlx::query!(
        r#"
        SELECT url, structured_data
        FROM crawl_urls
        WHERE id = $1 AND crawl_id = $2
        "#,
        &url_id,
        &crawl_id,
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::not_found("URL not found"))?;

    let items = row
        .structured_data
        .as_array()
        .cloned()
        .unwrap_or_default();
    let mut by_type: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    for it in &items {
        if let Some(t) = it.get("type").and_then(|v| v.as_str()) {
            *by_type.entry(t.to_string()).or_default() += 1;
        }
    }

    Ok(Json(serde_json::json!({
        "url": row.url,
        "count": items.len(),
        "counts_by_type": by_type,
        "items": items,
    })))
}

async fn get_overview(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(crawl_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    verify_crawl_ownership(&state, &auth, &crawl_id).await?;

    let rows = sqlx::query!(
        r#"
        SELECT filter_key, COUNT(*) as "count!"
        FROM crawl_url_findings
        WHERE crawl_id = $1
        GROUP BY filter_key
        ORDER BY filter_key
        "#,
        &crawl_id,
    )
    .fetch_all(&state.db)
    .await?;

    let counts: serde_json::Map<String, serde_json::Value> = rows
        .into_iter()
        .map(|r| (r.filter_key, serde_json::json!(r.count)))
        .collect();

    Ok(Json(serde_json::json!(counts)))
}

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
