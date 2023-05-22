use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use reqwest::Client;
use serde_json::Value;
use sqlx::types::Uuid;
use sqlx::{PgPool, Postgres, QueryBuilder};
use strum::IntoEnumIterator;
use strum_macros::EnumIter;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;
use tokio::time::Instant;

use super::ExecuteCrawler;

#[derive(Debug, Clone, Copy, EnumIter, PartialEq, Eq, Hash)]
pub enum Category {
    Vegan,
    Vegetables,
    RefrigeratedGoods,
    Meats,
    Pantry,
    Sweets,
    Bread,
    Drinks,
    FrozenGoods,
    Baby,
    Pet,
    Beauty,
    Household,
    KitchenUtensils,
}

impl Category {
    fn id(&self) -> &'static str {
        match self {
            Category::Vegan => "F17",
            Category::Vegetables => "F1",
            Category::RefrigeratedGoods => "F2",
            Category::Meats => "F3",
            Category::Pantry => "F4",
            Category::Sweets => "F5",
            Category::Bread => "F6",
            Category::Drinks => "F7",
            Category::FrozenGoods => "F8",
            Category::Baby => "F9",
            Category::Pet => "F10",
            Category::Beauty => "F11",
            Category::Household => "F12",
            Category::KitchenUtensils => "F13",
        }
    }
}

#[derive(Debug)]
pub struct SparUrl {
    category: Category,
    page: usize,
    page_size: usize,
}

impl SparUrl {
    pub fn new(category: Category, page: usize) -> Self {
        SparUrl {
            category,
            page,
            page_size: 80,
        }
    }

    pub fn as_url(&self) -> String {
        format!("https://search-spar.spar-ics.com/fact-finder/rest/v4/search/products_lmos_at?query=*&q=*&page={}&hitsPerPage={}&filter=category-path:{}", self.page, self.page_size, self.category.id())
    }

    pub fn next_page(&mut self) {
        self.page += 1;
    }
}

#[derive(Debug, serde::Deserialize, Clone)]
pub struct Product {
    description: String,
    #[serde(rename = "sales-unit")]
    sales_unit: String,
    pub title: String,
    #[serde(rename = "code-internal")]
    pub id_internal: String,
    price: f32,
    brand: Vec<String>,
    url: String,
    pub name: String,
    #[serde(rename = "product-number")]
    pub product_number: String,
    #[serde(rename = "price-per-unit")]
    price_per_unit: String,
}

#[derive(Debug, serde::Deserialize)]
struct Page {
    #[serde(rename = "currentPage")]
    current: usize,
    #[serde(rename = "pageCount")]
    count: usize,
}

#[derive(Debug)]
pub struct SparCrawl {}

impl ExecuteCrawler for SparCrawl {
    type Category = self::Category;
    type Product = self::Product;

    async fn get_or_add_categories(pool: &PgPool) -> Result<Arc<HashMap<Self::Category, Uuid>>> {
        let mut category_map = HashMap::new();

        for category in Self::Category::iter() {
            let category_string = format!("{:?}", category);

            let id: Option<(Uuid,)> =
                sqlx::query_as("select sc_id from sc_spar_category where sc_text = $1")
                    .bind(&category_string)
                    .fetch_optional(pool)
                    .await?;
            let id = match id {
                Some(id) => id.0,
                None => {
                    let id: (Uuid,) = sqlx::query_as(
                        "insert into sc_spar_category (sc_text) values ( $1 ) returning sc_id",
                    )
                    .bind(&category_string)
                    .fetch_one(pool)
                    .await
                    .unwrap();

                    id.0
                }
            };

            category_map.insert(category, id);
        }

        Ok(Arc::new(category_map))
    }

    async fn download_category(
        crawl_id: Uuid,
        client: Client,
        pool: &PgPool,
        category: Self::Category,
    ) -> Result<Vec<(Self::Product, Uuid)>> {
        let mut spar_url = SparUrl::new(category.clone(), 1);

        let mut products = Vec::new();

        loop {
            let url = spar_url.as_url();
            let res = client.get(&url).send().await?;

            if res.status() == 200 {
                let text = res.text().await?;

                let body: Value = serde_json::from_str(&text)?;

                let document_id: (Uuid,) = sqlx::query_as("insert into sr_spar_raw (sr_raw, sr_url, sr_cs_crawl_session) values ( $1, $2, $3 ) returning sr_id")
                    .bind(text)
                    .bind(url)
                    .bind(crawl_id)
                    .fetch_one(pool).await?;

                let hits_raw = body["hits"].clone();
                let arr = hits_raw
                    .as_array()
                    .map(|items| {
                        items
                            .iter()
                            .map(|item| serde_json::from_value(item["masterValues"].clone()))
                            .map(|item| item.ok())
                            .filter(|item| item.is_some())
                            .map(|item| (item.unwrap(), document_id.0))
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();

                products.extend(arr);

                let paging_info: Page = serde_json::from_value(body["paging"].clone())?;
                if paging_info.current >= paging_info.count {
                    break;
                }

                spar_url.next_page();
            }
        }

        Ok(products)
    }

    async fn insert_products(
        pool: &PgPool,
        category_map: Arc<HashMap<Self::Category, Uuid>>,
        category: Self::Category,
        products: Vec<(Self::Product, Uuid)>,
    ) -> Result<()> {
        let mut products_with_id = Vec::with_capacity(products.len());

        for (product, document_id) in products {
            let product_id: Option<(Uuid,)> =
                sqlx::query_as("select sp_id from sp_spar_product where sp_spar_id = $1")
                    .bind(&product.id_internal)
                    .fetch_optional(pool)
                    .await?;
            let product_id = match product_id {
                Some(product_id) => product_id.0,
                None => {
                    let brand_name = product.brand.join(";");

                    let product_id: (Uuid,) = sqlx::query_as("insert into sp_spar_product (sp_spar_id, sp_description, sp_online_shop_url, sp_name, sp_brand, sp_sc_category) values ( $1, $2, $3, $4, $5, $6 ) returning sp_id")
                        .bind(&product.id_internal)
                        .bind(&product.description)
                        .bind(&product.url)
                        .bind(&product.name)
                        .bind(brand_name)
                        .bind(category_map.get(&category).unwrap().clone())
                        .fetch_one(pool)
                        .await?;

                    product_id.0
                }
            };

            products_with_id.push((product, document_id, product_id));
        }

        let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new("insert into spr_spar_price (spr_price, spr_sales_unit, spr_price_unit, spr_sp_product, spr_sr_raw)");

        query_builder.push_values(
            products_with_id.iter().take(16),
            |mut b, (product, document_id, product_id)| {
                b.push_bind(&product.price);
                b.push_bind(&product.sales_unit);
                b.push_bind(&product.price_per_unit);
                b.push_bind(product_id);
                b.push_bind(document_id);
            },
        );

        let query = query_builder.build();
        query.execute(pool).await?;

        Ok(())
    }

    async fn execute(pool: &PgPool, crawl_id: Uuid) -> Result<()> {
        let client = Client::new();

        let category_map = Self::get_or_add_categories(&pool).await?;

        let semaphore = Arc::new(Semaphore::new(3));
        let mut set = JoinSet::new();

        for category in Self::Category::iter() {
            let semaphore = semaphore.clone();
            let client = client.clone();
            let pool = pool.clone();
            let crawl_id = crawl_id.clone();

            set.spawn(async move {
                let permit = semaphore.acquire().await.unwrap();

                println!("start with: {:?}", category);

                let products = Self::download_category(crawl_id, client, &pool, category).await;

                drop(permit);

                println!("finish with: {:?}", category);

                (products, category)
            });
        }

        let mut products_lists = Vec::new();

        while let Some(res) = set.join_next().await {
            match res {
                Ok((Ok(products), category)) => products_lists.push((category, products)),
                err => println!("err: {:?}", err),
            }
        }

        println!(
            "products: {}",
            products_lists
                .iter()
                .map(|(_, item)| item.len())
                .sum::<usize>()
        );
        println!("Start with inserting into db");

        let semaphore = Arc::new(Semaphore::new(20));
        let mut set = JoinSet::new();

        let now = Instant::now();

        for (category, products) in products_lists {
            let semaphore = semaphore.clone();
            let pool = pool.clone();
            let category_map = category_map.clone();

            set.spawn(async move {
                let permit = semaphore.acquire().await.unwrap();

                let status = Self::insert_products(&pool, category_map, category, products).await;

                drop(permit);

                status
            });
        }

        while let Some(res) = set.join_next().await {
            let _ = res;
        }

        let duration = now.elapsed();
        println!("took: {:?} ms", duration.as_millis());

        Ok(())
    }
}
