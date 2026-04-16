pub mod catalog;
pub mod crawls;
pub mod projects;
pub mod urls;

use axum::Router;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use crate::app_state::AppState;

pub fn router(state: AppState) -> Router {
    Router::new()
        .nest("/v1", api_routes())
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state)
}

fn api_routes() -> Router<AppState> {
    Router::new()
        .merge(projects::routes())
        .merge(crawls::routes())
        .merge(urls::routes())
        .merge(catalog::routes())
}
