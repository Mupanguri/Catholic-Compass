use sqlx::postgres::PgPoolOptions;
use std::time::Duration;

pub type Pool = sqlx::Pool<sqlx::Postgres>;

pub async fn connect(database_url: &str) -> Result<Pool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(30))
        .connect(database_url)
        .await
}
