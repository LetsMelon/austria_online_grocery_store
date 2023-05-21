use anyhow::Result;
use sqlx::types::Uuid;
use sqlx::PgPool;

pub mod billa;
pub mod spar;

pub trait ExecuteCrawler {
    async fn execute(pool: &PgPool, crawl_id: Uuid) -> Result<()>;
}
