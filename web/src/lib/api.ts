// Thin fetch wrapper for the StockLink gateway. Auth rides on the
// `sl_session` HttpOnly cookie the identity service sets on login
// (`credentials: 'include'`) — the browser attaches it automatically on
// every same-origin request through the gateway, so there is no token to
// manage here at all. `access_token` is still returned in login/refresh
// responses for non-browser clients; the web app ignores it.

const BASE = '/api';

export class ApiError extends Error {
  code: string;
  field?: string;
  status: number;
  constructor(status: number, code: string, message: string, field?: string) {
    super(message);
    this.status = status;
    this.code = code;
    this.field = field;
  }
}

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const res = await fetch(`${BASE}${path}`, {
    credentials: 'include',
    headers: { 'Content-Type': 'application/json', ...(init?.headers ?? {}) },
    ...init,
  });
  if (res.status === 204) return undefined as T;
  const isJson = res.headers.get('content-type')?.includes('application/json');
  const body = isJson ? await res.json().catch(() => null) : null;
  if (!res.ok) {
    const err = body?.error;
    throw new ApiError(res.status, err?.code ?? 'UNKNOWN', err?.message ?? res.statusText, err?.field);
  }
  return body as T;
}

export const api = {
  get: <T>(path: string) => request<T>(path),
  post: <T>(path: string, data?: unknown) =>
    request<T>(path, { method: 'POST', body: data !== undefined ? JSON.stringify(data) : undefined }),
  patch: <T>(path: string, data?: unknown) =>
    request<T>(path, { method: 'PATCH', body: data !== undefined ? JSON.stringify(data) : undefined }),
  delete: <T>(path: string) => request<T>(path, { method: 'DELETE' }),
};

// ── types (mirror backend/crates/*/src/schemas) ────────────────────────────

export type Role = 'warehouse' | 'store' | 'carrier' | 'admin';

export interface TokenPair {
  account_id: string;
  access_token: string;
  refresh_token: string;
  roles: Role[];
}

export interface Warehouse {
  id: string;
  name: string;
  region: string;
  address: string | null;
}

export interface Store {
  id: string;
  name: string;
  region: string;
  address: string | null;
}

export type Tier = 'unit' | 'case' | 'pallet';

export interface CatalogItem {
  id: string;
  warehouse_id: string;
  sku: string;
  name: string;
  description: string | null;
  currency: string;
  unit_label: string;
  unit_price: number;
  case_size: number | null;
  case_price: number | null;
  pallet_size: number | null;
  pallet_price: number | null;
  stock_qty_units: number;
  active: boolean;
}

export interface CartItem {
  id: string;
  catalog_item_id: string;
  tier: Tier;
  quantity: number;
}

export type OrderStatus = 'pending' | 'confirmed' | 'packed' | 'in_transit' | 'delivered' | 'cancelled';

export interface Order {
  id: string;
  store_id: string;
  warehouse_id: string;
  status: OrderStatus;
  currency: string;
  subtotal: number;
  total: number;
  placed_at: string;
}

export interface Notification {
  id: string;
  kind: string;
  title: string;
  body: string;
  read_at: string | null;
  created_at: string;
}
