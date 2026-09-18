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
  put: <T>(path: string, data?: unknown) =>
    request<T>(path, { method: 'PUT', body: data !== undefined ? JSON.stringify(data) : undefined }),
  patch: <T>(path: string, data?: unknown) =>
    request<T>(path, { method: 'PATCH', body: data !== undefined ? JSON.stringify(data) : undefined }),
  delete: <T>(path: string) => request<T>(path, { method: 'DELETE' }),
};

/** Uploads raw file bytes to a presigned (or, in dev, local-backend) upload
 *  url returned by `POST /v1/media/presign` — a plain PUT of the file body,
 *  never through the JSON `request()` wrapper above, and never carrying the
 *  session cookie (the s3 backend's presigned url isn't same-origin, and the
 *  dev local backend re-derives the account from the path it already signed
 *  server-side, not from a cookie). */
export async function uploadFile(uploadUrl: string, file: File): Promise<void> {
  const res = await fetch(uploadUrl, {
    method: 'PUT',
    credentials: 'include',
    headers: { 'Content-Type': file.type || 'application/octet-stream' },
    body: file,
  });
  if (!res.ok) throw new ApiError(res.status, 'UPLOAD_FAILED', 'Could not upload the file');
}

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

export const MAX_CATALOG_IMAGES = 5;

export interface CatalogImage {
  id: string;
  url: string;
  sort_order: number;
  is_thumbnail: boolean;
}

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
  thumbnail_url: string | null;
  images: CatalogImage[];
  warehouse_name: string;
  warehouse_region: string;
  warehouse_address: string | null;
}

/** Presigns, uploads and attaches one photo to a catalogue item — the three
 *  calls every "add a photo" button makes, in order. */
export async function uploadCatalogImage(
  warehouseId: string,
  itemId: string,
  file: File,
): Promise<CatalogItem> {
  const presigned = await api.post<{ media_asset_id: string; upload_url: string; public_url: string }>(
    '/v1/media/presign',
    { kind: 'catalog_item', content_type: file.type || 'application/octet-stream' },
  );
  await uploadFile(presigned.upload_url, file);
  return api.post<CatalogItem>(`/v1/warehouses/${warehouseId}/catalog/${itemId}/images`, {
    media_asset_id: presigned.media_asset_id,
    url: presigned.public_url,
  });
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
