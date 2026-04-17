use axum::routing::get;
use axum::{Json, Router};
use sf_core::filter_key::FilterKey;
use sf_core::tab::TabKey;

use crate::app_state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new().route("/catalog/tabs", get(list_tabs))
}

async fn list_tabs() -> Json<serde_json::Value> {
    let tabs: Vec<serde_json::Value> = TabKey::all()
        .iter()
        .filter(|t| !matches!(t, TabKey::Undef))
        .map(|tab| {
            let filters: Vec<serde_json::Value> = FilterKey::for_tab(*tab)
                .iter()
                .filter(|f| !f.is_deprecated())
                .map(|f| {
                    serde_json::json!({
                        "key": f,
                        "display_name": f.display_name(),
                        "severity": f.severity(),
                        "filter_type": f.filter_key_type(),
                        "has_watermark": f.has_watermark(),
                    })
                })
                .collect();

            serde_json::json!({
                "key": tab,
                "display_name": tab.display_name(),
                "i18n_key": tab.i18n_key(),
                "has_dynamic_filters": tab.has_dynamic_filters(),
                "filters": filters,
            })
        })
        .collect();

    Json(serde_json::json!(tabs))
}
