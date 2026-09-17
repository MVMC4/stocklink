import { useQuery } from '@tanstack/react-query';
import { api, type CartItem, type Warehouse, type Store } from './api';
import { useAuth } from './auth';

/** StockLink's MVP scope is one warehouse per account (mirrors the backend's
 * `resolve_single_store` for stores — see docs/STATUS.md). */
export function useMyWarehouse() {
  return useQuery({
    queryKey: ['my-warehouses'],
    queryFn: () => api.get<Warehouse[]>('/v1/onboarding/warehouses'),
    select: (rows) => rows[0] as Warehouse | undefined,
  });
}

export function useMyStore() {
  return useQuery({
    queryKey: ['my-stores'],
    queryFn: () => api.get<Store[]>('/v1/onboarding/stores'),
    select: (rows) => rows[0] as Store | undefined,
  });
}

/** The cart only exists for a store — a warehouse account has none, so
 *  identity's `resolve-single-store` lookup behind it correctly rejects one.
 *  Gating the query on the `store` role keeps a warehouse session from
 *  making a request that was never going to succeed for it.
 *
 *  Lives here, not in screens/Cart.tsx, so App.tsx's top-level cart-count
 *  badge can import it without pulling in the whole Cart screen — that
 *  static import next to the screen's own `React.lazy()` dynamic import
 *  would otherwise fold the screen into the main bundle and silently
 *  defeat the code-split. */
export function useCart() {
  const { session } = useAuth();
  return useQuery({
    queryKey: ['cart'],
    queryFn: () => api.get<CartItem[]>('/v1/cart/items'),
    enabled: !!session?.roles.includes('store'),
  });
}
