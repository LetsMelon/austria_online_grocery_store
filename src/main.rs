#![feature(async_fn_in_trait)]
#![feature(return_position_impl_trait_in_trait)]

use sqlx::postgres::PgPoolOptions;
use sqlx::types::Uuid;
use stores::billa::BillaCrawl;
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

    let (spar, billa) = tokio::join!(
        SparCrawl::execute(&pool, crawl_id.0),
        BillaCrawl::execute(&pool, crawl_id.0)
    );
    let _ = spar.unwrap();
    let _ = billa.unwrap();
}
