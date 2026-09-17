import { useMemo, useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { Package, Search, ShoppingCart, Store as StoreIcon } from 'lucide-react';
import { api, type CatalogItem, type Tier } from '../lib/api';
import { useToast } from '../lib/toast';
import { CardSkeleton, EmptyState } from '../components/ui';

/** Deterministic flat-colour chip per item (by SKU) — a visual category
 *  marker, same idea as a coloured folder label. No gradients: each chip is
 *  one solid colour from the --chip-* palette in styles.css. */
export function chipClass(key: string) {
  let h = 0;
  for (let i = 0; i < key.length; i++) h = (h * 31 + key.charCodeAt(i)) >>> 0;
  return `c${h % 4}`;
}

const TIERS: { key: Tier; label: string }[] = [
  { key: 'unit', label: 'Unit' },
  { key: 'case', label: 'Case' },
  { key: 'pallet', label: 'Pallet' },
];

export function Marketplace() {
  const [query, setQuery] = useState('');
  const { data: items, isLoading } = useQuery({
    queryKey: ['browse-catalog'],
    queryFn: () => api.get<CatalogItem[]>('/v1/catalog'),
  });

  const filtered = useMemo(
    () => (items ?? []).filter((i) => i.name.toLowerCase().includes(query.toLowerCase())),
    [items, query],
  );

  return (
    <div className="view">
      <div className="glass card">
        <div style={{ position: 'relative', maxWidth: 360 }}>
          <Search size={15} style={{ position: 'absolute', left: 12, top: 11, color: 'var(--faint)' }} />
          <input
            className="input"
            style={{ paddingLeft: 34 }}
            placeholder="Search products..."
            value={query}
            onChange={(e) => setQuery(e.target.value)}
          />
        </div>
      </div>

      {isLoading ? (
        <div className="grid product-grid">
          <div className="glass"><CardSkeleton rows={4} /></div>
          <div className="glass"><CardSkeleton rows={4} /></div>
          <div className="glass"><CardSkeleton rows={4} /></div>
        </div>
      ) : !filtered.length ? (
        <div className="glass card">
          <EmptyState icon={<StoreIcon size={28} />} title="No products found" hint="Try a different search." />
        </div>
      ) : (
        <div className="grid product-grid">
          {filtered.map((item) => (
            <ProductCard key={item.id} item={item} />
          ))}
        </div>
      )}
    </div>
  );
}

function ProductCard({ item }: { item: CatalogItem }) {
  const [tier, setTier] = useState<Tier>('unit');
  const [qty, setQty] = useState(1);
  const qc = useQueryClient();
  const { push } = useToast();

  const availableTiers = TIERS.filter((t) =>
    t.key === 'unit' ? true : t.key === 'case' ? !!item.case_price : !!item.pallet_price,
  );

  const price = tier === 'unit' ? item.unit_price : tier === 'case' ? item.case_price! : item.pallet_price!;

  const addToCart = useMutation({
    mutationFn: () => api.post('/v1/cart/items', { catalog_item_id: item.id, tier, quantity: qty }),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['cart'] });
      push(`Added ${qty} × ${item.name} to cart`);
    },
    onError: () => push('Could not add to cart', 'error'),
  });

  return (
    <div className="glass product-card">
      <div className={`icon-chip ${chipClass(item.sku)}`}>
        <Package size={16} />
      </div>
      <div className="name">{item.name}</div>
      <div className="meta">SKU {item.sku}</div>
      <div className="tiers">
        {availableTiers.map((t) => (
          <button key={t.key} type="button" className="tier-chip" aria-pressed={tier === t.key} onClick={() => setTier(t.key)}>
            {t.label}
          </button>
        ))}
      </div>
      <div className="price">
        {item.currency} {price.toFixed(2)} <small>/ {tier}</small>
      </div>
      <div className="qty-row">
        <input
          type="number"
          min={1}
          className="input"
          value={qty}
          onChange={(e) => setQty(Math.max(1, Number(e.target.value)))}
        />
        <button className="btn" style={{ flex: 1 }} disabled={addToCart.isPending} onClick={() => addToCart.mutate()}>
          <ShoppingCart size={14} /> Add
        </button>
      </div>
    </div>
  );
}
