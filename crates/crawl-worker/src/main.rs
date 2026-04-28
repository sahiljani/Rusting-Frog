use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Context;
use sf_core::config::CrawlConfig;
use sf_core::id::CrawlId;
use sqlx::postgres::PgPoolOptions;
use url::Url;

mod debug_log;
mod fetcher;
mod frontier;
mod parser;
mod pipeline;
mod robots;
mod sampler;
mod sitemap;

use debug_log::{DebugLogger, DEFAULT_DEBUG_LOG_DIR};
use sampler::Counters;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let database_url = std::env::var("DATABASE_URL").context("DATABASE_URL must be set")?;

    let db = PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await
        .context("failed to connect to Postgres")?;

    tracing::info!("crawl-worker started, polling for jobs");

    let debug_log_dir: PathBuf = std::env::var("SF_DEBUG_LOG_DIR")
        .unwrap_or_else(|_| DEFAULT_DEBUG_LOG_DIR.to_string())
        .into();

    loop {
        match pick_next_job(&db).await {
            Ok(Some(job)) => {
                tracing::info!(crawl_id = %job.crawl_id, seed = %job.seed_url, "picked up crawl job");

                let seed_url: Url = match job.seed_url.parse() {
                    Ok(u) => u,
                    Err(e) => {
                        tracing::error!(crawl_id = %job.crawl_id, error = %e, "invalid seed URL, marking failed");
                        let _ = sqlx::query!(
                            "UPDATE crawls SET status = 'failed' WHERE id = $1",
                            job.crawl_id.as_uuid()
                        )
                        .execute(&db)
                        .await;
                        continue;
                    }
                };

                let config = job.config.unwrap_or_default();

                let logger = DebugLogger::new(&debug_log_dir, &job.crawl_id.to_string());
                let counters = Arc::new(Counters::default());

                let mut pipeline = match pipeline::CrawlPipeline::new(
                    db.clone(),
                    job.crawl_id,
                    job.tenant_id,
                    seed_url,
                    config,
                    logger.clone(),
                    counters.clone(),
                ) {
                    Ok(p) => p,
                    Err(e) => {
                        tracing::error!(crawl_id = %job.crawl_id, error = %e, "failed to init pipeline");
                        let _ = sqlx::query!(
                            "UPDATE crawls SET status = 'failed' WHERE id = $1",
                            job.crawl_id.as_uuid()
                        )
                        .execute(&db)
                        .await;
                        continue;
                    }
                };

                // Sampler lives for the life of this crawl. Drop = stop.
                let _sampler =
                    sampler::spawn(logger.clone(), db.clone(), counters.clone(), debug_log_dir.clone());

                if let Err(e) = pipeline.run().await {
                    tracing::error!(crawl_id = %job.crawl_id, error = %e, "crawl failed");
                    logger.log("error", &format!("crawl failed: {e}"), serde_json::Value::Null);
                    let _ = sqlx::query!(
                        "UPDATE crawls SET status = 'failed' WHERE id = $1",
                        job.crawl_id.as_uuid()
                    )
                    .execute(&db)
                    .await;
                }
            }
            Ok(None) => {
                // No jobs, wait before polling again
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            }
            Err(e) => {
                tracing::error!(error = %e, "error polling for jobs");
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            }
        }
    }
}

struct CrawlJob {
    crawl_id: CrawlId,
    tenant_id: String,
    seed_url: String,
    config: Option<CrawlConfig>,
}

async fn pick_next_job(db: &sqlx::PgPool) -> anyhow::Result<Option<CrawlJob>> {
    // Atomically claim a queued crawl by setting status to 'starting'
    let row = sqlx::query!(
        r#"
        UPDATE crawls
        SET status = 'starting'
        WHERE id = (
            SELECT id FROM crawls
            WHERE status = 'queued'
            ORDER BY created_at ASC
            LIMIT 1
            FOR UPDATE SKIP LOCKED
        )
        RETURNING id, tenant_id, seed_urls, project_id
        "#,
    )
    .fetch_optional(db)
    .await?;

    let row = match row {
        Some(r) => r,
        None => return Ok(None),
    };

    // Extract first seed URL from JSONB array
    let seed_urls: Vec<String> = serde_json::from_value(row.seed_urls)?;

    let seed_url = seed_urls
        .into_iter()
        .next()
        .context("crawl has no seed URLs")?;

    // Load project config
    let project = sqlx::query!("SELECT config FROM projects WHERE id = $1", row.project_id)
        .fetch_optional(db)
        .await?;

    let config: Option<CrawlConfig> = project.and_then(|p| serde_json::from_value(p.config).ok());

    Ok(Some(CrawlJob {
        crawl_id: CrawlId::from_uuid(row.id),
        tenant_id: row.tenant_id,
        seed_url,
        config,
    }))
}
