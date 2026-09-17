import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { ShoppingCart, Trash2 } from 'lucide-react';
import { api, type CatalogItem } from '../lib/api';
import { useCart } from '../lib/hooks';
import { useToast } from '../lib/toast';
import { CardSkeleton, EmptyState } from '../components/ui';

export function Cart({ onCheckedOut, onBrowse }: { onCheckedOut: () => void; onBrowse: () => void }) {
  const qc = useQueryClient();
  const { push } = useToast();
  const { data: cartItems, isLoading } = useCart();
  const { data: catalog } = useQuery({
    queryKey: ['browse-catalog'],
    queryFn: () => api.get<CatalogItem[]>('/v1/catalog'),
  });

  const remove = useMutation({
    mutationFn: (id: string) => api.delete(`/v1/cart/items/${id}`),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['cart'] }),
  });

  const checkout = useMutation({
    mutationFn: () => api.post('/v1/orders/checkout', {}),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['cart'] });
      qc.invalidateQueries({ queryKey: ['my-orders'] });
      push('Order placed');
      onCheckedOut();
    },
    onError: () => push('Checkout failed — stock may have changed', 'error'),
  });

  const rows = (cartItems ?? []).map((line) => {
    const item = catalog?.find((c) => c.id === line.catalog_item_id);
    const price = item ? (line.tier === 'unit' ? item.unit_price : line.tier === 'case' ? item.case_price! : item.pallet_price!) : 0;
    return { line, item, price, total: price * line.quantity };
  });
  const grandTotal = rows.reduce((sum, r) => sum + r.total, 0);

  return (
    <div className="view">
      <div className="glass card">
        <div className="card-head">
          <h2>Your cart</h2>
          <span>{rows.length} item(s)</span>
        </div>
        {isLoading ? (
          <CardSkeleton rows={2} />
        ) : !rows.length ? (
          <EmptyState
            icon={<ShoppingCart size={28} />}
            title="Your cart is empty"
            hint="Browse the marketplace to add items."
            action={{ label: 'Browse marketplace', onClick: onBrowse }}
          />
        ) : (
          <>
            {rows.map(({ line, item, price, total }) => (
              <div className="cart-line" key={line.id}>
                <div>
                  <div className="name">{item?.name ?? 'Item'}</div>
                  <div className="meta">
                    {line.quantity} × {line.tier} @ {item?.currency ?? 'BWP'} {price.toFixed(2)}
                  </div>
                </div>
                <div style={{ fontWeight: 600 }}>
                  {item?.currency ?? 'BWP'} {total.toFixed(2)}
                </div>
                <button className="btn ghost" onClick={() => remove.mutate(line.id)} aria-label="Remove">
                  <Trash2 size={14} />
                </button>
              </div>
            ))}
            <div className="cart-total">
              <span>Total</span>
              <span className="amount">BWP {grandTotal.toFixed(2)}</span>
            </div>
            <div className="form-actions">
              <button className="btn" disabled={checkout.isPending} onClick={() => checkout.mutate()}>
                {checkout.isPending ? 'Placing order…' : 'Checkout'}
              </button>
            </div>
          </>
        )}
      </div>
    </div>
  );
}
