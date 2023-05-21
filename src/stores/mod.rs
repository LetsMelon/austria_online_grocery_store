use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;

use anyhow::Result;
use reqwest::Client;
use sqlx::types::Uuid;
use sqlx::PgPool;
use strum::IntoEnumIterator;

pub mod billa;
pub mod spar;

pub trait ExecuteCrawler: Debug {
    type Category: Send + Sync + IntoEnumIterator + Debug;
    type Product: Send + Sync + Debug;

    async fn get_or_add_categories(pool: &PgPool) -> Result<Arc<HashMap<Self::Category, Uuid>>>;

    async fn download_category(
        crawl_id: Uuid,
        client: Client,
        pool: &PgPool,
        category: Self::Category,
    ) -> Result<Vec<(Self::Product, Uuid)>>;

    async fn insert_products(
        pool: &PgPool,
        category_map: Arc<HashMap<Self::Category, Uuid>>,
        category: Self::Category,
        products: Vec<(Self::Product, Uuid)>,
    ) -> Result<()>;

    // TODO implement generic execute function
    async fn execute(pool: &PgPool, crawl_id: Uuid) -> Result<()>;
}
