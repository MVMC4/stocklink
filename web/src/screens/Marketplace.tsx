import { useMemo, useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { MapPin, Package, Search, ShoppingCart, Store as StoreIcon, X } from 'lucide-react';
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
  const [viewing, setViewing] = useState<CatalogItem | null>(null);
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
            <ProductCard key={item.id} item={item} onView={() => setViewing(item)} />
          ))}
        </div>
      )}

      {viewing && <ListingDetail item={viewing} onClose={() => setViewing(null)} />}
    </div>
  );
}

/** Tier selection, quantity and the add-to-cart mutation — shared by the
 *  grid card and the listing detail view so the two never drift. */
function useAddToCart(item: CatalogItem) {
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

  return { tier, setTier, qty, setQty, availableTiers, price, addToCart };
}

function ProductCard({ item, onView }: { item: CatalogItem; onView: () => void }) {
  const { tier, setTier, qty, setQty, availableTiers, price, addToCart } = useAddToCart(item);

  return (
    <div className="glass product-card">
      <button
        type="button"
        onClick={onView}
        style={{ all: 'unset', cursor: 'pointer', display: 'block' }}
        aria-label={`View ${item.name}`}
      >
        {item.thumbnail_url ? (
          <img
            src={item.thumbnail_url}
            alt=""
            style={{
              width: '100%',
              aspectRatio: '4 / 3',
              objectFit: 'cover',
              borderRadius: 'var(--radius-sm)',
              border: 'var(--bw) solid var(--ink)',
              display: 'block',
              marginBottom: 4,
            }}
          />
        ) : (
          <div className={`icon-chip ${chipClass(item.sku)}`}>
            <Package size={16} />
          </div>
        )}
        <div className="name">{item.name}</div>
        <div className="meta">SKU {item.sku}</div>
      </button>
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

/** Full listing view: every photo, the description, all price tiers and
 *  where it ships from. `warehouse_address`/`warehouse_region` are the real
 *  fields identity has today — there's no warehouse/store geocoordinate yet,
 *  so this deliberately doesn't show a fabricated "X km away" distance (see
 *  docs/STATUS.md's catalogue-images entry for what a real one would need). */
function ListingDetail({ item, onClose }: { item: CatalogItem; onClose: () => void }) {
  const images = [...item.images].sort((a, b) => a.sort_order - b.sort_order);
  const [active, setActive] = useState(0);
  const mainImage = images[active]?.url ?? item.thumbnail_url;
  const { tier, setTier, qty, setQty, availableTiers, price, addToCart } = useAddToCart(item);

  return (
    <div className="palette-scrim" onClick={onClose}>
      <div
        className="glass pop palette"
        style={{ padding: 0, width: 'min(760px, calc(100vw - 32px))', maxHeight: '86vh', overflow: 'auto' }}
        onClick={(e) => e.stopPropagation()}
      >
        <div style={{ display: 'flex', justifyContent: 'flex-end', padding: '14px 14px 0' }}>
          <button className="btn ghost" style={{ padding: 6 }} onClick={onClose} aria-label="Close">
            <X size={14} />
          </button>
        </div>
        <div style={{ display: 'grid', gridTemplateColumns: 'minmax(0, 1.1fr) minmax(0, 1fr)', gap: 24, padding: '0 28px 28px' }}>
          <div>
            {mainImage ? (
              <img
                src={mainImage}
                alt=""
                style={{ width: '100%', aspectRatio: '4 / 3', objectFit: 'cover', borderRadius: 'var(--radius)', border: 'var(--bw) solid var(--ink)', display: 'block' }}
              />
            ) : (
              <div className="glass" style={{ aspectRatio: '4 / 3', display: 'grid', placeItems: 'center', color: 'var(--faint)' }}>
                <Package size={40} />
              </div>
            )}
            {images.length > 1 && (
              <div style={{ display: 'flex', gap: 8, marginTop: 10 }}>
                {images.map((img, i) => (
                  <button
                    key={img.id}
                    type="button"
                    onClick={() => setActive(i)}
                    style={{
                      padding: 0,
                      width: 56,
                      height: 56,
                      borderRadius: 10,
                      border: `2px solid ${i === active ? 'var(--blue)' : 'var(--ink)'}`,
                      overflow: 'hidden',
                      cursor: 'pointer',
                      opacity: i === active ? 1 : 0.75,
                    }}
                  >
                    <img src={img.url} alt="" style={{ width: '100%', height: '100%', objectFit: 'cover', display: 'block' }} />
                  </button>
                ))}
              </div>
            )}
          </div>

          <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
            <div>
              <h1 style={{ fontSize: 21, margin: '0 0 4px', fontWeight: 800, letterSpacing: '-0.01em' }}>{item.name}</h1>
              <div className="meta" style={{ color: 'var(--faint)', fontSize: 12.5, fontWeight: 600 }}>SKU {item.sku}</div>
            </div>

            {item.description && <p style={{ margin: 0, fontSize: 13.5, color: 'var(--muted)', lineHeight: 1.55 }}>{item.description}</p>}

            <div style={{ display: 'flex', alignItems: 'center', gap: 8, fontSize: 13, color: 'var(--muted)', fontWeight: 600 }}>
              <MapPin size={14} style={{ flex: '0 0 auto' }} />
              Ships from {item.warehouse_name || 'a connected warehouse'}
              {item.warehouse_region ? ` · ${item.warehouse_region}` : ''}
              {item.warehouse_address ? ` · ${item.warehouse_address}` : ''}
            </div>

            <table style={{ marginTop: 4 }}>
              <thead>
                <tr>
                  <th>Tier</th>
                  <th>Price</th>
                  <th>Stock</th>
                </tr>
              </thead>
              <tbody>
                <tr>
                  <td>Unit ({item.unit_label})</td>
                  <td>{item.currency} {item.unit_price.toFixed(2)}</td>
                  <td rowSpan={3} style={{ verticalAlign: 'middle' }}>{item.stock_qty_units}</td>
                </tr>
                {item.case_price != null && (
                  <tr>
                    <td>Case ({item.case_size ?? '—'} units)</td>
                    <td>{item.currency} {item.case_price.toFixed(2)}</td>
                  </tr>
                )}
                {item.pallet_price != null && (
                  <tr>
                    <td>Pallet ({item.pallet_size ?? '—'} units)</td>
                    <td>{item.currency} {item.pallet_price.toFixed(2)}</td>
                  </tr>
                )}
              </tbody>
            </table>

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
                <ShoppingCart size={14} /> Add to cart
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
