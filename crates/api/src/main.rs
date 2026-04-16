use anyhow::Context;
use sqlx::postgres::PgPoolOptions;
use tracing_subscriber::EnvFilter;

mod app_state;
mod error;
mod extractors;
mod middleware;
mod routes;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .json()
        .init();

    let database_url =
        std::env::var("DATABASE_URL").context("DATABASE_URL must be set")?;
    let jwt_secret =
        std::env::var("JWT_SECRET").context("JWT_SECRET must be set")?;
    let redis_url =
        std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());

    let db = PgPoolOptions::new()
        .max_connections(20)
        .connect(&database_url)
        .await
        .context("failed to connect to Postgres")?;

    sqlx::migrate!("../migrations/sql")
        .run(&db)
        .await
        .context("failed to run migrations")?;

    let redis = redis::Client::open(redis_url.as_str())
        .context("failed to parse Redis URL")?;

    let state = app_state::AppState {
        db,
        redis,
        jwt_secret,
    };

    let app = routes::router(state);

    let addr = std::env::var("LISTEN_ADDR").unwrap_or_else(|_| "0.0.0.0:3000".to_string());
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("listening on {addr}");
    axum::serve(listener, app).await?;

    Ok(())
}
