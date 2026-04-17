//! `/v1/crawls/{id}/issues` — the SF "Issues" right-panel grid + detail copy.
//!
//! Groups findings by filter_key, joins in hand-authored issue metadata
//! (issue type, priority, description, how-to-fix), and returns both the
//! row list and a four-count summary (Issues / Warnings / Opportunities /
//! Info) matching SF's top chips.

use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};
use sf_core::filter_key::{FilterKey, IssueType};
use uuid::Uuid;

use crate::app_state::AppState;
use crate::error::ApiError;
use crate::extractors::auth::AuthUser;

pub fn routes() -> Router<AppState> {
    Router::new().route("/crawls/{crawl_id}/issues", get(list_issues))
}

async fn list_issues(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(crawl_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    verify_crawl_ownership(&state, &auth, &crawl_id).await?;

    // Per-filter counts for this crawl, plus total URL count for % math.
    let counts = sqlx::query!(
        r#"
        SELECT filter_key, COUNT(DISTINCT crawl_url_id) as "count!"
        FROM crawl_url_findings
        WHERE crawl_id = $1
        GROUP BY filter_key
        "#,
        &crawl_id,
    )
    .fetch_all(&state.db)
    .await?;

    let total_urls: i64 = sqlx::query_scalar!(
        r#"SELECT COUNT(*) as "c!" FROM crawl_urls WHERE crawl_id = $1"#,
        &crawl_id,
    )
    .fetch_one(&state.db)
    .await?;

    let mut items: Vec<serde_json::Value> = Vec::new();
    let mut summary_issues = 0i64;
    let mut summary_warnings = 0i64;
    let mut summary_opportunities = 0i64;
    let mut summary_info = 0i64;

    for row in counts {
        let Some(fk) = FilterKey::from_serde_key(&row.filter_key) else {
            // Unknown/legacy key — skip quietly.
            continue;
        };
        let itype = fk.issue_type();
        // Only show actionable rows in the Issues tab — stat/info counters
        // still contribute to the summary "Total" but not to the grid.
        match itype {
            IssueType::Issue => summary_issues += 1,
            IssueType::Warning => summary_warnings += 1,
            IssueType::Opportunity => summary_opportunities += 1,
            IssueType::Info => summary_info += 1,
        }
        if matches!(itype, IssueType::Info) {
            continue;
        }
        let percent = if total_urls > 0 {
            (row.count as f64 * 100.0) / total_urls as f64
        } else {
            0.0
        };
        items.push(serde_json::json!({
            "filter_key": fk.serde_key(),
            "issue_name": format!("{}: {}", fk.tab().display_name(), fk.display_name()),
            "issue_type": itype.as_str(),
            "priority": fk.priority().map(|p| p.as_str()),
            "urls": row.count,
            "percent_of_total": (percent * 10.0).round() / 10.0,
            "description": fk.description(),
            "how_to_fix": fk.how_to_fix(),
        }));
    }

    // Sort by type severity (Issue > Warning > Opportunity), then URL count desc.
    items.sort_by(|a, b| {
        let rank = |v: &serde_json::Value| match v["issue_type"].as_str().unwrap_or("") {
            "Issue" => 0,
            "Warning" => 1,
            "Opportunity" => 2,
            _ => 3,
        };
        rank(a).cmp(&rank(b)).then_with(|| {
            b["urls"]
                .as_i64()
                .unwrap_or(0)
                .cmp(&a["urls"].as_i64().unwrap_or(0))
        })
    });

    Ok(Json(serde_json::json!({
        "summary": {
            "issues": summary_issues,
            "warnings": summary_warnings,
            "opportunities": summary_opportunities,
            "info": summary_info,
            "total": summary_issues + summary_warnings + summary_opportunities,
        },
        "total_urls": total_urls,
        "items": items,
    })))
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
