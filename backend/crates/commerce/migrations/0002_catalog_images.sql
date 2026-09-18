-- Catalogue item photos: up to 5 per item (enforced in
-- CatalogService::attach_image, not here), one marked as the thumbnail.
-- `media_asset_id` is an opaque cross-service reference to the media
-- service's own `media_assets` table (see 0001_baseline.sql's note on why
-- cross-service ids are never foreign keys here) — `url` is stored
-- alongside it so listings render without a network call back to media.

create table catalog_item_images (
    id uuid primary key default gen_random_uuid(),
    catalog_item_id uuid not null references catalog_items(id) on delete cascade,
    media_asset_id uuid not null,
    url text not null,
    sort_order smallint not null default 0,
    is_thumbnail boolean not null default false,
    created_at timestamptz not null default now()
);
create index catalog_item_images_catalog_item_id_idx on catalog_item_images(catalog_item_id, sort_order);

-- At most one thumbnail per item — CatalogService::set_thumbnail always
-- clears the old one before setting the new one, in that order, so this
-- never has to reject a legitimate sequential write.
create unique index catalog_item_images_one_thumbnail_idx
    on catalog_item_images(catalog_item_id) where is_thumbnail;
