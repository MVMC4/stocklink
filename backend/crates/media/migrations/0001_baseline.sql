-- Media service baseline schema. `account_id` is an opaque UUID (identity
-- owns the `accounts` table in its own database) — see commerce's
-- migration file for the full explanation of this pattern.

create extension if not exists pgcrypto;

create table media_assets (
    id uuid primary key default gen_random_uuid(),
    account_id uuid not null,
    kind text not null,
    object_key text not null,
    url text not null,
    content_type text not null,
    byte_size bigint not null check (byte_size >= 0),
    created_at timestamptz not null default now()
);
create index media_assets_account_id_idx on media_assets(account_id);
