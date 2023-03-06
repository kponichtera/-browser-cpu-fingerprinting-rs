use std::error::Error;
use sqlx::{Pool, Postgres};
use sqlx::postgres::PgPoolOptions;
use crate::config::BackendConfig;

#[derive(Clone)]
pub struct BackendContext {
    pub connection_pool: Pool<Postgres>
}

pub async fn build_context(config: &BackendConfig) -> Result<BackendContext, Box<dyn Error>> {
    let pool = PgPoolOptions::new()
        .max_connections(config.database_connection_count)
        .connect(config.database_url.as_str())
        .await?;

    sqlx::migrate!()
        .run(&pool)
        .await?;

    Ok(BackendContext {
        connection_pool: pool
    })
}
