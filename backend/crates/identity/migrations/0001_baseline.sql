-- Identity service baseline schema. Owns: accounts, contacts, roles, OTP
-- challenges, refresh-token sessions, warehouse/store/carrier onboarding,
-- and the admin audit log. Every FK here stays inside this one database —
-- no other service's tables are visible to this one (see docs/STATUS.md's
-- microservices entry for how cross-service references work instead).

create extension if not exists pgcrypto;

create table accounts (
    id uuid primary key default gen_random_uuid(),
    status text not null default 'active' check (status in ('active', 'suspended', 'deleted')),
    display_name text not null,
    preferred_lang text not null default 'en',
    primary_contact_id uuid,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    deleted_at timestamptz
);

create table account_contacts (
    id uuid primary key default gen_random_uuid(),
    account_id uuid not null references accounts(id) on delete cascade,
    channel text not null check (channel in ('email', 'phone')),
    identifier text not null check (btrim(identifier) <> ''),
    verified_at timestamptz,
    is_primary boolean not null default false,
    created_at timestamptz not null default now(),
    unique (channel, identifier)
);
create index account_contacts_account_id_idx on account_contacts(account_id);

alter table accounts
    add constraint accounts_primary_contact_fkey
    foreign key (primary_contact_id) references account_contacts(id) on delete set null;

create table account_roles (
    account_id uuid not null references accounts(id) on delete cascade,
    role text not null check (role in ('warehouse', 'store', 'carrier', 'admin')),
    created_at timestamptz not null default now(),
    primary key (account_id, role)
);

create table otp_challenges (
    id uuid primary key default gen_random_uuid(),
    channel text not null check (channel in ('email', 'phone')),
    identifier text not null,
    code_hash text not null,
    attempts int not null default 0,
    max_attempts int not null default 5,
    expires_at timestamptz not null,
    consumed_at timestamptz,
    created_at timestamptz not null default now()
);
create index otp_challenges_lookup_idx on otp_challenges(channel, identifier, created_at desc);

create table sessions (
    id uuid primary key default gen_random_uuid(),
    account_id uuid not null references accounts(id) on delete cascade,
    secret_hash text not null,
    user_agent text,
    ip text,
    created_at timestamptz not null default now(),
    last_used_at timestamptz,
    revoked_at timestamptz,
    expires_at timestamptz not null
);
create index sessions_account_id_idx on sessions(account_id);

create table warehouses (
    id uuid primary key default gen_random_uuid(),
    account_id uuid not null references accounts(id) on delete cascade,
    name text not null,
    region text not null,
    address text,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now()
);
create index warehouses_account_id_idx on warehouses(account_id);

create table stores (
    id uuid primary key default gen_random_uuid(),
    account_id uuid not null references accounts(id) on delete cascade,
    name text not null,
    region text not null,
    address text,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now()
);
create index stores_account_id_idx on stores(account_id);

create table carriers (
    id uuid primary key default gen_random_uuid(),
    account_id uuid not null references accounts(id) on delete cascade,
    name text not null,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now()
);
create index carriers_account_id_idx on carriers(account_id);

create table admin_audit_log (
    id uuid primary key default gen_random_uuid(),
    actor_account_id uuid not null references accounts(id),
    action text not null,
    target_type text,
    target_id uuid,
    metadata jsonb not null default '{}'::jsonb,
    created_at timestamptz not null default now()
);
create index admin_audit_log_actor_idx on admin_audit_log(actor_account_id);
