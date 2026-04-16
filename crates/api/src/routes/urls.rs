use axum::extract::{Path, Query, State};
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;
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
