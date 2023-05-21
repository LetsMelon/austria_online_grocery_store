#![feature(async_fn_in_trait)]

use sqlx::postgres::PgPoolOptions;
use sqlx::types::Uuid;
use stores::spar::SparCrawl;

use crate::stores::ExecuteCrawler;

mod stores;

#[tokio::main]
async fn main() {
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect("postgres://postgres:postgres@localhost/postgres")
        .await
        .unwrap();

    let crawl_id: (Uuid,) =
        sqlx::query_as("INSERT INTO bcw_billa_crawl DEFAULT VALUES RETURNING bcw_id")
            .fetch_one(&pool)
            .await
            .unwrap();

    println!("crawl id: {:?}", crawl_id.0);

    SparCrawl::execute(&pool, crawl_id.0).await.unwrap();
}
