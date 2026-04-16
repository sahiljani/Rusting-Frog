use anyhow::Context;

mod fetcher;
mod frontier;
mod parser;
mod pipeline;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .json()
        .init();

    let database_url =
        std::env::var("DATABASE_URL").context("DATABASE_URL must be set")?;
    let redis_url =
        std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());

    tracing::info!("crawl-worker starting");

    // TODO: connect to Postgres + Redis, poll for queued crawl jobs, run pipeline
    tracing::warn!("crawl-worker is a stub — pipeline not yet wired");

    Ok(())
}
