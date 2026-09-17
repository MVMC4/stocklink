-- Notifications service baseline schema. `account_id` is an opaque UUID
-- (identity owns the `accounts` table in its own database) — see
-- commerce's migration file for the full explanation of this pattern.

create extension if not exists pgcrypto;

create table notifications (
    id uuid primary key default gen_random_uuid(),
    account_id uuid not null,
    kind text not null,
    title text not null,
    body text not null,
    data jsonb not null default '{}'::jsonb,
    read_at timestamptz,
    created_at timestamptz not null default now()
);
create index notifications_account_id_idx on notifications(account_id, created_at desc);

create table devices (
    id uuid primary key default gen_random_uuid(),
    account_id uuid not null,
    provider text not null check (provider in ('FCM')),
    token text not null,
    active boolean not null default true,
    created_at timestamptz not null default now(),
    unique (provider, token)
);
create index devices_account_id_idx on devices(account_id);
