pub mod catalog;
pub mod crawls;
pub mod debug;
pub mod dev;
pub mod issues;
pub mod projects;
pub mod reports;
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
    let mut r = Router::new()
        .merge(projects::routes())
        .merge(crawls::routes())
        .merge(urls::routes())
        .merge(catalog::routes())
        .merge(reports::routes())
        .merge(issues::routes())
        .merge(debug::routes());
    if std::env::var("SF_DEV_MODE").ok().as_deref() == Some("1") {
        r = r.merge(dev::routes());
    }
    r
}
