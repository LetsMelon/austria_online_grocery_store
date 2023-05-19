use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use reqwest::Client;
use sqlx::postgres::PgPoolOptions;
use sqlx::types::Uuid;
use strum::IntoEnumIterator;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;

use crate::stores::billa::{BillaCrawl, Category};

mod stores;

#[tokio::main]
async fn main() {
    let client = Client::new();

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect("postgres://postgres:postgres@localhost/postgres")
        .await
        .unwrap();

    let mut category_map = HashMap::new();

    for category in Category::iter() {
        let category_string = format!("{:?}", category);

        let id: Result<Option<(Uuid,)>, sqlx::Error> =
            sqlx::query_as("SELECT bc_id FROM bc_billa_category WHERE bc_text = $1")
                .bind(&category_string)
                .fetch_optional(&pool)
                .await;
        let id = match id.unwrap() {
            Some(id) => id.0,
            None => {
                let id: (Uuid,) = sqlx::query_as(
                    "INSERT INTO bc_billa_category (bc_text) VALUES ( $1 ) RETURNING bc_id",
                )
                .bind(&category_string)
                .fetch_one(&pool)
                .await
                .unwrap();

                id.0
            }
        };

        category_map.insert(category, id);
    }

    let semaphore = Arc::new(Semaphore::new(3));
    let mut set = JoinSet::new();

    let crawl_id: (Uuid,) =
        sqlx::query_as("INSERT INTO bcw_billa_crawl DEFAULT VALUES RETURNING bcw_id")
            .fetch_one(&pool)
            .await
            .unwrap();

    println!("crawl uuid: {:?}", crawl_id.0);

    for category in Category::iter() {
        let semaphore = semaphore.clone();
        let client = client.clone();
        let pool = pool.clone();
        let crawl_id = crawl_id.0.clone();

        set.spawn(async move {
            let permit = semaphore.acquire().await.unwrap();

            let products = BillaCrawl::download_category(crawl_id, client, &pool, category).await;

            drop(permit);

            (products, category)
        });
    }

    let mut products_lists = Vec::new();

    while let Some(res) = set.join_next().await {
        match res {
            Ok((Ok(products), category)) => {
                products_lists.push((category, products));
            }
            err => println!("err: {:?}", err),
        }
    }

    println!("start product inserts");

    let semaphore = Arc::new(Semaphore::new(20));
    let mut set = JoinSet::new();
    let category_map = Arc::new(category_map);

    for (category, products) in products_lists {
        let semaphore = semaphore.clone();
        let pool = pool.clone();
        let category_map = category_map.clone();

        set.spawn(async move {
            let permit = semaphore.acquire().await.unwrap();

            let status = BillaCrawl::insert_products(&pool, category_map, category, products).await;

            drop(permit);

            status
        });
    }

    while let Some(res) = set.join_next().await {
        let _ = res;
    }
}
