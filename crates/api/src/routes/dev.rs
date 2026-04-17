//! Development-only helper routes.
//!
//! These routes are only registered when the API is started with
//! `SF_DEV_MODE=1`. They exist purely to make the local React UI
//! self-serve a JWT for dev crawls without forcing the user to
//! shell out to `node scripts/mint_jwt.js`.
//!
//! In production these endpoints are never registered, so calling
//! them returns 404. The real identity flow is always Laravel →
//! signed JWT → this API.

use axum::routing::get;
use axum::{Json, Router};
use jsonwebtoken::{EncodingKey, Header, encode};
use serde_json::json;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::app_state::AppState;
use crate::extractors::auth::Claims;

pub fn routes() -> Router<AppState> {
    Router::new().route("/dev/token", get(mint_dev_token))
}

async fn mint_dev_token(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> Json<serde_json::Value> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as usize)
        .unwrap_or(0);
    let claims = Claims {
        tenant_id: "dev-tenant".to_string(),
        user_id: "dev-user".to_string(),
        scopes: vec![
            "crawl:read".to_string(),
            "crawl:write".to_string(),
            "export:read".to_string(),
            "config:write".to_string(),
        ],
        iat: now,
        exp: now + 3600,
    };
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(state.jwt_secret.as_bytes()),
    )
    .expect("HS256 encode never fails for valid inputs");
    Json(json!({ "token": token, "expires_in": 3600 }))
}
