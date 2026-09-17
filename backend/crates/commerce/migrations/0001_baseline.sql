-- Commerce service baseline schema. Owns: catalogue, cart, orders, bulk
-- consolidation, settlements, ledger, and the transactional outbox.
--
-- `warehouse_id`, `store_id` and ledger `account_id` columns below are
-- opaque UUIDs, not foreign keys — the tables they used to reference
-- (warehouses, stores, accounts) live in the identity service's own
-- database now. Ownership/existence is checked by calling identity's
-- `/internal/*` API (see `src/clients/identity_client.rs`), not by a
-- database constraint. This is the standard microservices tradeoff: each
-- service owns its own data outright, cross-service references are
-- validated over the network instead of by Postgres.

create extension if not exists pgcrypto;

create table catalog_items (
    id uuid primary key default gen_random_uuid(),
    warehouse_id uuid not null,
    sku text not null,
    name text not null,
    description text,
    currency text not null default 'BWP',
    unit_label text not null default 'unit',
    unit_price numeric(12, 2) not null check (unit_price >= 0),
    case_size int check (case_size > 0),
    case_price numeric(12, 2) check (case_price >= 0),
    pallet_size int check (pallet_size > 0),
    pallet_price numeric(12, 2) check (pallet_price >= 0),
    stock_qty_units numeric(14, 2) not null default 0 check (stock_qty_units >= 0),
    active boolean not null default true,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    unique (warehouse_id, sku)
);
create index catalog_items_warehouse_id_idx on catalog_items(warehouse_id);

create table cart_items (
    id uuid primary key default gen_random_uuid(),
    store_id uuid not null,
    catalog_item_id uuid not null references catalog_items(id) on delete cascade,
    tier text not null check (tier in ('unit', 'case', 'pallet')),
    quantity numeric(14, 2) not null check (quantity > 0),
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    unique (store_id, catalog_item_id, tier)
);
create index cart_items_store_id_idx on cart_items(store_id);

create table orders (
    id uuid primary key default gen_random_uuid(),
    store_id uuid not null,
    warehouse_id uuid not null,
    status text not null default 'pending'
        check (status in ('pending', 'confirmed', 'packed', 'in_transit', 'delivered', 'cancelled')),
    currency text not null default 'BWP',
    subtotal numeric(14, 2) not null default 0,
    total numeric(14, 2) not null default 0,
    idempotency_key text unique,
    placed_at timestamptz not null default now(),
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now()
);
create index orders_store_id_idx on orders(store_id);
create index orders_warehouse_id_idx on orders(warehouse_id);

create table order_items (
    id uuid primary key default gen_random_uuid(),
    order_id uuid not null references orders(id) on delete cascade,
    catalog_item_id uuid not null references catalog_items(id),
    tier text not null check (tier in ('unit', 'case', 'pallet')),
    quantity numeric(14, 2) not null check (quantity > 0),
    unit_price numeric(14, 2) not null check (unit_price >= 0),
    line_total numeric(14, 2) not null check (line_total >= 0)
);
create index order_items_order_id_idx on order_items(order_id);

create table order_events (
    id uuid primary key default gen_random_uuid(),
    order_id uuid not null references orders(id) on delete cascade,
    kind text not null,
    detail text,
    created_at timestamptz not null default now()
);
create index order_events_order_id_idx on order_events(order_id);

create table bulk_orders (
    id uuid primary key default gen_random_uuid(),
    warehouse_id uuid not null,
    catalog_item_id uuid not null references catalog_items(id),
    tier text not null check (tier in ('unit', 'case', 'pallet')),
    status text not null default 'open' check (status in ('open', 'confirmed', 'fulfilled', 'cancelled')),
    delivery_window_start timestamptz,
    delivery_window_end timestamptz,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now()
);
create index bulk_orders_warehouse_id_idx on bulk_orders(warehouse_id);

create table bulk_order_allocations (
    id uuid primary key default gen_random_uuid(),
    bulk_order_id uuid not null references bulk_orders(id) on delete cascade,
    order_id uuid not null references orders(id) on delete cascade,
    store_id uuid not null,
    quantity numeric(14, 2) not null check (quantity > 0),
    created_at timestamptz not null default now(),
    unique (bulk_order_id, order_id)
);
create index bulk_order_allocations_bulk_order_id_idx on bulk_order_allocations(bulk_order_id);

create table settlements (
    id uuid primary key default gen_random_uuid(),
    order_id uuid not null unique references orders(id) on delete cascade,
    status text not null default 'pending' check (status in ('pending', 'completed', 'failed')),
    total_amount numeric(14, 2) not null check (total_amount >= 0),
    currency text not null,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now()
);

create table ledger_journals (
    id uuid primary key default gen_random_uuid(),
    kind text not null,
    currency text not null,
    order_id uuid references orders(id),
    settlement_id uuid references settlements(id),
    memo text,
    created_at timestamptz not null default now()
);

create table ledger_entries (
    id uuid primary key default gen_random_uuid(),
    journal_id uuid not null references ledger_journals(id) on delete cascade,
    account_id uuid not null,
    direction text not null check (direction in ('debit', 'credit')),
    amount numeric(14, 2) not null check (amount > 0),
    currency text not null,
    memo text,
    created_at timestamptz not null default now()
);
create index ledger_entries_journal_id_idx on ledger_entries(journal_id);
create index ledger_entries_account_id_idx on ledger_entries(account_id);

-- Transactional outbox (see stocklink-shared::outbox — column names are
-- load-bearing).
create table domain_outbox (
    id uuid primary key default gen_random_uuid(),
    event_id uuid not null,
    idempotency_key text not null unique,
    aggregate_type text not null,
    aggregate_id uuid not null,
    event_type text not null,
    event_version int not null default 1,
    topic text not null,
    payload jsonb not null,
    occurred_at timestamptz not null,
    status text not null default 'pending' check (status in ('pending', 'processing', 'retry', 'published', 'dead')),
    attempts int not null default 0,
    available_at timestamptz not null default now(),
    locked_at timestamptz,
    locked_by text,
    last_error text,
    dead_lettered_at timestamptz,
    published_at timestamptz,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now()
);
create index domain_outbox_claim_idx on domain_outbox(status, available_at);
