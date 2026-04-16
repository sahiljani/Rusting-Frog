use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;
use sf_core::id::ProjectId;
use uuid::Uuid;

use crate::app_state::AppState;
use crate::error::ApiError;
use crate::extractors::auth::AuthUser;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/projects", get(list_projects).post(create_project))
        .route("/projects/{id}", get(get_project).delete(delete_project))
}

#[derive(Debug, Deserialize)]
pub struct CreateProjectRequest {
    pub name: String,
    pub seed_url: String,
}

async fn list_projects(
    auth: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let rows = sqlx::query!(
        r#"
        SELECT id, name, seed_url, created_at, updated_at
        FROM projects
        WHERE tenant_id = $1
        ORDER BY created_at DESC
        "#,
        auth.tenant_id.as_str()
    )
    .fetch_all(&state.db)
    .await?;

    let projects: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|r| {
            serde_json::json!({
                "id": r.id,
                "name": r.name,
                "seed_url": r.seed_url,
                "created_at": r.created_at,
                "updated_at": r.updated_at,
            })
        })
        .collect();

    Ok(Json(serde_json::json!(projects)))
}

async fn create_project(
    auth: AuthUser,
    State(state): State<AppState>,
    Json(req): Json<CreateProjectRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let _: url::Url = req
        .seed_url
        .parse()
        .map_err(|_| ApiError::validation("invalid seed URL"))?;

    let id = ProjectId::new();
    let now = chrono::Utc::now();

    sqlx::query!(
        r#"
        INSERT INTO projects (id, tenant_id, name, seed_url, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $5)
        "#,
        id.as_uuid(),
        auth.tenant_id.as_str(),
        req.name,
        req.seed_url,
        now,
    )
    .execute(&state.db)
    .await?;

    Ok(Json(serde_json::json!({
        "id": id,
        "name": req.name,
        "seed_url": req.seed_url,
        "created_at": now,
        "updated_at": now,
    })))
}

async fn get_project(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let row = sqlx::query!(
        r#"
        SELECT id, name, seed_url, created_at, updated_at
        FROM projects
        WHERE id = $1 AND tenant_id = $2
        "#,
        &id,
        auth.tenant_id.as_str()
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::not_found("project not found"))?;

    Ok(Json(serde_json::json!({
        "id": row.id,
        "name": row.name,
        "seed_url": row.seed_url,
        "created_at": row.created_at,
        "updated_at": row.updated_at,
    })))
}

async fn delete_project(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let result = sqlx::query!(
        "DELETE FROM projects WHERE id = $1 AND tenant_id = $2",
        &id,
        auth.tenant_id.as_str()
    )
    .execute(&state.db)
    .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("project not found"));
    }

    Ok(Json(serde_json::json!({"deleted": true})))
}
