use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use jsonwebtoken::{DecodingKey, Validation, decode};
use serde::{Deserialize, Serialize};
use sf_core::id::TenantId;

use crate::app_state::AppState;
use crate::error::ApiError;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub tenant_id: String,
    pub user_id: String,
    pub scopes: Vec<String>,
    pub exp: usize,
    pub iat: usize,
}

#[derive(Debug)]
pub struct AuthUser {
    pub tenant_id: TenantId,
    pub user_id: String,
    pub scopes: Vec<String>,
}

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let header = parts
            .headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| ApiError::unauthorized("missing Authorization header"))?;

        let token = header
            .strip_prefix("Bearer ")
            .ok_or_else(|| ApiError::unauthorized("invalid Bearer token format"))?;

        let key = DecodingKey::from_secret(state.jwt_secret.as_bytes());
        let mut validation = Validation::default();
        validation.set_required_spec_claims(&["exp", "iat"]);

        let data = decode::<Claims>(token, &key, &validation)
            .map_err(|e| ApiError::unauthorized(format!("invalid token: {e}")))?;

        Ok(AuthUser {
            tenant_id: TenantId::new(&data.claims.tenant_id),
            user_id: data.claims.user_id,
            scopes: data.claims.scopes,
        })
    }
}
