use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use sf_core::crawl::CrawlStatus;
use sf_core::id::CrawlId;
use uuid::Uuid;

use crate::app_state::AppState;
use crate::error::ApiError;
use crate::extractors::auth::AuthUser;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/projects/{project_id}/crawls", post(start_crawl).get(list_crawls))
        .route("/crawls/{id}", get(get_crawl))
        .route("/crawls/{id}/pause", post(pause_crawl))
        .route("/crawls/{id}/resume", post(resume_crawl))
        .route("/crawls/{id}/stop", post(stop_crawl))
}

#[derive(Debug, Serialize)]
pub struct CrawlResponse {
    pub id: CrawlId,
    pub status: String,
    pub message: String,
}

async fn start_crawl(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(project_id): Path<Uuid>,
) -> Result<Json<CrawlResponse>, ApiError> {
    let project = sqlx::query!(
        "SELECT id, seed_url FROM projects WHERE id = $1 AND tenant_id = $2",
        &project_id,
        auth.tenant_id.as_str()
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::not_found("project not found"))?;

    let crawl_id = CrawlId::new();
    let now = chrono::Utc::now();
    let seed_urls = serde_json::json!([&project.seed_url]);

    sqlx::query!(
        r#"
        INSERT INTO crawls (id, project_id, tenant_id, status, seed_urls, urls_discovered, urls_crawled, created_at)
        VALUES ($1, $2, $3, 'queued', $4, 0, 0, $5)
        "#,
        crawl_id.as_uuid(),
        &project_id,
        auth.tenant_id.as_str(),
        seed_urls,
        now,
    )
    .execute(&state.db)
    .await?;

    Ok(Json(CrawlResponse {
        id: crawl_id,
        status: "queued".to_string(),
        message: "crawl queued".to_string(),
    }))
}

async fn get_crawl(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let row = sqlx::query!(
        r#"
        SELECT id, project_id, status, seed_urls,
               urls_discovered, urls_crawled,
               started_at, completed_at, created_at
        FROM crawls
        WHERE id = $1 AND tenant_id = $2
        "#,
        &id,
        auth.tenant_id.as_str()
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::not_found("crawl not found"))?;

    Ok(Json(serde_json::json!({
        "id": row.id,
        "project_id": row.project_id,
        "status": row.status,
        "seed_urls": row.seed_urls,
        "urls_discovered": row.urls_discovered,
        "urls_crawled": row.urls_crawled,
        "started_at": row.started_at,
        "completed_at": row.completed_at,
        "created_at": row.created_at,
    })))
}

async fn list_crawls(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(project_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let rows = sqlx::query!(
        r#"
        SELECT id, status, urls_discovered, urls_crawled,
               started_at, completed_at, created_at
        FROM crawls
        WHERE project_id = $1 AND tenant_id = $2
        ORDER BY created_at DESC
        "#,
        &project_id,
        auth.tenant_id.as_str()
    )
    .fetch_all(&state.db)
    .await?;

    let crawls: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|r| {
            serde_json::json!({
                "id": r.id,
                "status": r.status,
                "urls_discovered": r.urls_discovered,
                "urls_crawled": r.urls_crawled,
                "started_at": r.started_at,
                "completed_at": r.completed_at,
                "created_at": r.created_at,
            })
        })
        .collect();

    Ok(Json(serde_json::json!(crawls)))
}

async fn pause_crawl(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<CrawlResponse>, ApiError> {
    update_crawl_status(&state, &auth, &id, "paused").await
}

async fn resume_crawl(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<CrawlResponse>, ApiError> {
    update_crawl_status(&state, &auth, &id, "running").await
}

async fn stop_crawl(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<CrawlResponse>, ApiError> {
    update_crawl_status(&state, &auth, &id, "completed").await
}

async fn update_crawl_status(
    state: &AppState,
    auth: &AuthUser,
    id: &Uuid,
    status: &str,
) -> Result<Json<CrawlResponse>, ApiError> {
    let result = sqlx::query!(
        "UPDATE crawls SET status = $1 WHERE id = $2 AND tenant_id = $3",
        status,
        id,
        auth.tenant_id.as_str()
    )
    .execute(&state.db)
    .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("crawl not found"));
    }

    Ok(Json(CrawlResponse {
        id: CrawlId::from_uuid(*id),
        status: status.to_string(),
        message: format!("crawl status updated to {status}"),
    }))
}
