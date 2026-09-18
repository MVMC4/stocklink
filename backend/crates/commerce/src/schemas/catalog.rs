use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct PublishCatalogItemReq {
    #[validate(length(min = 1, max = 64))]
    pub sku: String,
    #[validate(length(min = 1, max = 200))]
    pub name: String,
    pub description: Option<String>,
    #[validate(length(equal = 3))]
    pub currency: String,
    #[validate(length(min = 1, max = 32))]
    pub unit_label: String,
    /// Server-validated for non-negativity in `CatalogService::publish_item`
    /// (rust_decimal doesn't implement `validator`'s numeric-range trait).
    pub unit_price: Decimal,
    pub case_size: Option<i32>,
    pub case_price: Option<Decimal>,
    pub pallet_size: Option<i32>,
    pub pallet_price: Option<Decimal>,
    pub stock_qty_units: Decimal,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CatalogImageRes {
    pub id: Uuid,
    pub url: String,
    pub sort_order: i16,
    pub is_thumbnail: bool,
}

impl From<crate::models::catalog::CatalogItemImage> for CatalogImageRes {
    fn from(img: crate::models::catalog::CatalogItemImage) -> Self {
        Self {
            id: img.id,
            url: img.url,
            sort_order: img.sort_order,
            is_thumbnail: img.is_thumbnail,
        }
    }
}

/// Attaches an already-uploaded media asset (see the media service's
/// `POST /v1/media/presign`) to a catalogue item as a photo. Commerce trusts
/// the caller-supplied `url` rather than calling back to media to confirm
/// it — see `docs/STATUS.md`'s catalogue-images entry for why that's an
/// accepted gap for now, not an oversight.
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct AttachImageReq {
    pub media_asset_id: Uuid,
    #[validate(length(min = 1, max = 2048))]
    pub url: String,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct ReorderImagesReq {
    #[validate(length(min = 1, max = 5))]
    pub image_ids: Vec<Uuid>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CatalogItemRes {
    pub id: Uuid,
    pub warehouse_id: Uuid,
    pub sku: String,
    pub name: String,
    pub description: Option<String>,
    pub currency: String,
    pub unit_label: String,
    pub unit_price: Decimal,
    pub case_size: Option<i32>,
    pub case_price: Option<Decimal>,
    pub pallet_size: Option<i32>,
    pub pallet_price: Option<Decimal>,
    pub stock_qty_units: Decimal,
    pub active: bool,
    /// The thumbnail image's url if one is set, else the lowest-`sort_order`
    /// image, else `None` — the single url a list/grid view needs without
    /// also shipping the full `images` array to every card.
    pub thumbnail_url: Option<String>,
    pub images: Vec<CatalogImageRes>,
    /// Denormalized from identity's own `warehouses` table (see
    /// `IdentityClient::get_warehouse`) purely so a store browsing the
    /// marketplace can see where a listing ships from without a second
    /// round trip — commerce still treats identity as the source of truth,
    /// this is a read-time join over the network, not a stored copy.
    pub warehouse_name: String,
    pub warehouse_region: String,
    pub warehouse_address: Option<String>,
}

impl CatalogItemRes {
    pub fn from_item_and_images(
        item: crate::models::catalog::CatalogItem,
        images: Vec<crate::models::catalog::CatalogItemImage>,
    ) -> Self {
        let thumbnail_url = images
            .iter()
            .find(|i| i.is_thumbnail)
            .or_else(|| images.first())
            .map(|i| i.url.clone());
        Self {
            id: item.id,
            warehouse_id: item.warehouse_id,
            sku: item.sku,
            name: item.name,
            description: item.description,
            currency: item.currency,
            unit_label: item.unit_label,
            unit_price: item.unit_price,
            case_size: item.case_size,
            case_price: item.case_price,
            pallet_size: item.pallet_size,
            pallet_price: item.pallet_price,
            stock_qty_units: item.stock_qty_units,
            active: item.active,
            thumbnail_url,
            images: images.into_iter().map(Into::into).collect(),
            warehouse_name: String::new(),
            warehouse_region: String::new(),
            warehouse_address: None,
        }
    }

    pub fn with_warehouse(mut self, name: &str, region: &str, address: Option<&str>) -> Self {
        self.warehouse_name = name.to_string();
        self.warehouse_region = region.to_string();
        self.warehouse_address = address.map(str::to_string);
        self
    }
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct AddToCartReq {
    pub catalog_item_id: Uuid,
    #[validate(length(min = 1))]
    pub tier: String,
    /// Server-validated as `> 0` in `CatalogService::add_to_cart`.
    pub quantity: Decimal,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CartItemRes {
    pub id: Uuid,
    pub catalog_item_id: Uuid,
    pub tier: String,
    pub quantity: Decimal,
}

impl From<crate::models::catalog::CartItem> for CartItemRes {
    fn from(item: crate::models::catalog::CartItem) -> Self {
        Self {
            id: item.id,
            catalog_item_id: item.catalog_item_id,
            tier: item.tier,
            quantity: item.quantity,
        }
    }
}
