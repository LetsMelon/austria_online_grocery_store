use std::cell::Cell;
use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use reqwest::Client;
use serde_json::Value;
use sqlx::postgres::PgPoolOptions;
use sqlx::types::Uuid;
use sqlx::PgPool;
use strum::IntoEnumIterator;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;

use crate::stores::billa::{BillaUrl, Category, PagingInfo, Product};

mod stores;

async fn download_category(
    crawl_id: Uuid,
    client: Client,
    pool: &PgPool,
    category: Category,
) -> Result<Vec<(Product, Uuid)>> {
    let mut last_page = false;

    let mut billa_url = BillaUrl::new(category.clone(), 1);

    let mut products = Vec::new();

    while !last_page {
        println!("{:?}: {}", category, billa_url.page());

        let url = billa_url.as_url();
        let res = client.get(&url).send().await?;

        if res.status() == 200 {
            let text = res.text().await?;
            let body: Value = serde_json::from_str(&text)?;

            let document_id: (Uuid,) = sqlx::query_as(
                "insert into br_billa_raw (br_raw, br_url, br_bcw_crawl) values ( $1, $2, $3 ) RETURNING br_id",
            )
            .bind(text)
            .bind(url)
            .bind(crawl_id)
            .fetch_one(pool)
            .await?;

            let paging_info: PagingInfo = serde_json::from_value(body["pagingInfo"].clone())?;

            let products_raw = body["tiles"].clone();
            let arr = products_raw
                .as_array()
                .map(|items| {
                    items
                        .iter()
                        .map(|item| serde_json::from_value(item["data"].clone()))
                        .map(|item| item.ok())
                        .filter(|item| item.is_some())
                        .map(|item| (item.unwrap(), document_id.0))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();

            products.extend(arr);

            billa_url.next_page();

            last_page = paging_info.is_last_page;
        } else {
            sqlx::query("insert into br_billa_raw (br_url, br_err) values ( $1, $2 )")
                .bind(url)
                .bind(format!("{:?}", res.text().await?))
                .execute(pool)
                .await?;
        }
    }

    Ok(products)
}

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

            let products = download_category(crawl_id, client, &pool, category).await;

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

            let status = insert_products(&pool, category_map, category, products).await;

            drop(permit);

            status
        });
    }

    while let Some(res) = set.join_next().await {
        let _ = res;
    }
}

async fn insert_products(
    pool: &PgPool,
    category_map: Arc<HashMap<Category, Uuid>>,
    category: Category,
    products: Vec<(Product, Uuid)>,
) -> Result<()> {
    for (product, document_id) in products {
        // TODO get bpo_id as option
        let count: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM bpo_billa_product WHERE bpo_billa_id = $1")
                .bind(&product.billa_id)
                .fetch_one(pool)
                .await?;
        let count_already_in_db = count.0 != 0;

        let product_id = if count_already_in_db {
            let product_id: (Uuid,) =
                sqlx::query_as("SELECT bpo_id FROM bpo_billa_product WHERE bpo_billa_id = $1")
                    .bind(&product.billa_id)
                    .fetch_one(pool)
                    .await?;

            product_id.0
        } else {
            // TODO add category into db and link with it
            let product_id: (Uuid,) = sqlx::query_as("INSERT INTO bpo_billa_product (bpo_online_shop_url, bpo_billa_id, bpo_name, bpo_description, bpo_brand, bpo_badge, bpo_unit, bpo_price_factor, bpo_grammage, bpo_bc_category) VALUES ( $1, $2, $3, $4, $5, $6, $7, $8, $9, $10) RETURNING bpo_id")
                .bind(product.online_shop_url)
                .bind(product.billa_id)
                .bind(&product.name)
                .bind(product.description)
                .bind(product.brand)
                .bind(product.grammage_badge)
                .bind(product.grammage_unit)
                .bind(product.grammage_price_factor)
                .bind(product.grammage)
                .bind(category_map.get(&category).unwrap().clone())
                .fetch_one(pool).await?;

            product_id.0
        };

        sqlx::query("INSERT INTO bp_billa_price (bp_bpo_product, bp_br_raw, bp_normal, bp_unit) VALUES ($1, $2, $3, $4)")
            .bind(product_id)
            .bind(&document_id)
            .bind(product.price.normal)
            .bind(product.price.unit)
            .execute(pool).await?;
    }

    Ok(())
}
