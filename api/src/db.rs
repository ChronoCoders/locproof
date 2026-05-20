use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

/// Connect to Postgres using `DATABASE_URL` and verify the connection.
pub async fn create_pool() -> anyhow::Result<PgPool> {
    let url = std::env::var("DATABASE_URL")
        .map_err(|_| anyhow::anyhow!("DATABASE_URL not set"))?;
    let pool = PgPoolOptions::new()
        .max_connections(8)
        .connect(&url)
        .await?;
    Ok(pool)
}

/// Apply all migrations under `./migrations` to the connected database.
pub async fn run_migrations(pool: &PgPool) -> anyhow::Result<()> {
    sqlx::migrate!("../migrations").run(pool).await?;
    Ok(())
}
