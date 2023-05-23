#![feature(async_fn_in_trait)]
#![feature(return_position_impl_trait_in_trait)]

use sqlx::postgres::PgPoolOptions;
use sqlx::types::Uuid;
use stores::billa::BillaCrawl;
use stores::spar::SparCrawl;
use tracing::info;
use tracing_subscriber::fmt;
use tracing_subscriber::prelude::*;

use crate::stores::ExecuteCrawler;

mod stores;
mod utils;

#[tokio::main]
async fn main() {
    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(opentelemetry_otlp::new_exporter().tonic())
        .install_batch(opentelemetry::runtime::Tokio)
        .unwrap();
    let telemetry_layer = tracing_opentelemetry::layer().with_tracer(tracer);

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "austria_online_grocery_store=debug".into()),
        )
        .with(fmt::layer())
        .with(telemetry_layer)
        .init();

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect("postgres://postgres:postgres@localhost/postgres")
        .await
        .unwrap();
    info!("Created postgres pool");

    let crawl_id: (Uuid,) =
        sqlx::query_as("INSERT INTO bcw_billa_crawl DEFAULT VALUES RETURNING bcw_id")
            .fetch_one(&pool)
            .await
            .unwrap();

    info!("crawl id: {:?}", crawl_id.0);

    let (spar, billa) = tokio::join!(
        SparCrawl::execute(&pool, crawl_id.0),
        BillaCrawl::execute(&pool, crawl_id.0)
    );
    let _ = spar.unwrap();
    let _ = billa.unwrap();

    // SparCrawl::execute(&pool, crawl_id.0).await.unwrap();
}
