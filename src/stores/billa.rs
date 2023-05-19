use std::hash::Hash;

use serde::{Deserialize, Deserializer};
use sqlx::types::Uuid;
use strum_macros::EnumIter;

#[derive(Debug, Clone, Copy, EnumIter, PartialEq, Eq, Hash)]
pub enum Category {
    Vegetables,
    Bread,
    Drinks,
    RefrigeratedGoods,
    Staple,
    Sweets,
    CareProducts,
    Household,
    Pet,
}

impl Category {
    fn id(&self) -> &'static str {
        match self {
            Category::Vegetables => "B2-1",
            Category::Bread => "B2-2",
            Category::Drinks => "B2-3",
            Category::RefrigeratedGoods => "B2-4",
            Category::Staple => "B2-6",
            Category::Sweets => "B2-7",
            Category::CareProducts => "B2-8",
            Category::Household => "B2-9",
            Category::Pet => "B2-A",
        }
    }
}

#[derive(Debug)]
pub struct BillaUrl {
    category: Category,
    page: usize,
    page_size: usize,
}

impl BillaUrl {
    pub fn new(category: Category, page: usize) -> Self {
        BillaUrl {
            category,
            page,
            page_size: 40,
        }
    }

    pub fn as_url(&self) -> String {
        format!("https://shop.billa.at/api/search/full?category={}&includeSort%5B%5D=rank&page={}&sort=rank&storeId=00-10&pageSize={}", self.category.id(), self.page, self.page_size)
    }

    pub fn next_page(&mut self) {
        self.page += 1;
    }

    pub fn page(&self) -> usize {
        self.page
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct PagingInfo {
    page: usize,
    #[serde(rename = "pageSize")]
    page_size: usize,
    #[serde(rename = "numResults")]
    numResults: usize,
    offset: usize,
    limit: usize,
    #[serde(rename = "isFirstPage")]
    is_first_page: bool,
    #[serde(rename = "isLastPage")]
    pub is_last_page: bool,
}

#[derive(Debug, serde::Deserialize, sqlx::FromRow)]
pub struct Product {
    #[sqlx(rename = "bpo_online_shop_url")]
    #[serde(rename = "canonicalPath")]
    pub online_shop_url: String,
    #[sqlx(rename = "bpo_billa_id")]
    #[serde(rename = "articleId")]
    pub billa_id: String,
    #[sqlx(rename = "bpo_name")]
    pub name: String,
    #[sqlx(rename = "bpo_description")]
    #[serde(deserialize_with = "deserialize_null_default")]
    pub description: String,
    #[sqlx(rename = "bpo_brand")]
    #[serde(deserialize_with = "deserialize_null_default")]
    pub brand: String,
    #[sqlx(rename = "bpo_badge")]
    #[serde(rename = "grammageBadge")]
    #[serde(deserialize_with = "deserialize_null_default")]
    pub grammage_badge: String,
    #[sqlx(rename = "bpo_unit")]
    #[serde(rename = "grammageUnit")]
    pub grammage_unit: String,
    #[sqlx(rename = "bpo_price_factor")]
    #[serde(rename = "grammagePriceFactor")]
    pub grammage_price_factor: f32,
    #[sqlx(rename = "bpo_grammage")]
    #[serde(rename = "grammage")]
    pub grammage: String,
    pub price: Price,
}

impl Hash for Product {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.billa_id.hash(state);
    }
}

#[derive(Debug, serde::Deserialize, sqlx::FromRow)]
pub struct Price {
    #[sqlx(rename = "bp_normal")]
    pub normal: f32,
    #[sqlx(rename = "bp_unit")]
    #[serde(deserialize_with = "deserialize_null_default")]
    pub unit: String,
}

fn deserialize_null_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    T: Default + Deserialize<'de>,
    D: Deserializer<'de>,
{
    let opt = Option::deserialize(deserializer)?;
    Ok(opt.unwrap_or_default())
}

#[derive(Debug, sqlx::FromRow)]
pub struct TableBillaRaw {
    #[sqlx(rename = "br_id")]
    pub id: Uuid,
    #[sqlx(rename = "br_raw")]
    pub raw: Option<String>,
    #[sqlx(rename = "br_url")]
    pub url: String,
}

#[derive(Debug, sqlx::FromRow)]
pub struct TableBillaPrice {
    #[sqlx(rename = "bp_id")]
    pub id: Uuid,
    #[sqlx(flatten)]
    pub price: Price,
}

#[derive(Debug, sqlx::FromRow)]
pub struct TableBillaProduct {
    #[sqlx(rename = "bpo_id")]
    pub id: Uuid,
    #[sqlx(flatten)]
    pub product: Product,
}
