import { useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import {
  AlertCircle,
  Boxes,
  ChevronLeft,
  ChevronRight,
  Image as ImageIcon,
  Package,
  Plus,
  Star,
  TrendingUp,
  Warehouse as WarehouseIcon,
  X,
} from 'lucide-react';
import { api, uploadCatalogImage, MAX_CATALOG_IMAGES, type CatalogItem, type Order } from '../lib/api';
import { useMyWarehouse } from '../lib/hooks';
import { useToast } from '../lib/toast';
import { CardSkeleton, EmptyState, InfoTip, KpiSkeleton, Sparkline, StatusPill } from '../components/ui';

export function WarehouseDashboard() {
  const { data: warehouse, isLoading: loadingWarehouse } = useMyWarehouse();

  const { data: items } = useQuery({
    queryKey: ['catalog', warehouse?.id],
    queryFn: () => api.get<CatalogItem[]>(`/v1/warehouses/${warehouse!.id}/catalog`),
    enabled: !!warehouse,
  });

  const { data: orders } = useQuery({
    queryKey: ['warehouse-orders', warehouse?.id],
    queryFn: () => api.get<Order[]>(`/v1/warehouses/${warehouse!.id}/orders`),
    enabled: !!warehouse,
  });

  if (loadingWarehouse) {
    return (
      <div className="view">
        <div className="grid kpis">
          <KpiSkeleton />
          <KpiSkeleton />
          <KpiSkeleton />
          <KpiSkeleton />
        </div>
        <div className="glass"><CardSkeleton rows={4} /></div>
      </div>
    );
  }
  if (!warehouse) return <NoWarehouse />;

  const pendingOrders = orders?.filter((o) => o.status === 'pending' || o.status === 'confirmed').length ?? 0;
  const delivered = orders?.filter((o) => o.status === 'delivered').sort((a, b) => a.placed_at.localeCompare(b.placed_at)) ?? [];
  const revenue = delivered.reduce((sum, o) => sum + o.total, 0);
  // Real cumulative running total, in delivery order — not fabricated.
  const revenueTrend = delivered.reduce<number[]>((acc, o) => [...acc, (acc.at(-1) ?? 0) + o.total], []);
  const listingsCount = items?.filter((i) => i.active).length ?? 0;

  return (
    <div className="view">
      <div className="grid kpis">
        <div className="glass pop kpi">
          <div className="kpi-head">
            <div className="label">Warehouse</div>
            <div className="icon-chip c1"><WarehouseIcon size={15} /></div>
          </div>
          <div className="value" style={{ fontSize: 18 }}>
            {warehouse.name}
          </div>
          <div className="delta">{warehouse.region}</div>
        </div>
        <div className="glass pop kpi">
          <div className="kpi-head">
            <div className="label">Active listings</div>
            <div className="icon-chip c0"><Boxes size={15} /></div>
          </div>
          <div className="value">{listingsCount}</div>
          <Sparkline points={items?.map((_, i) => i + 1) ?? []} color="var(--chip-blue)" />
        </div>
        <div className="glass pop kpi">
          <div className="kpi-head">
            <div className="label" style={{ display: 'flex', alignItems: 'center', gap: 5 }}>
              Orders needing attention
              <InfoTip>Pending or confirmed orders — not yet packed.</InfoTip>
            </div>
            <div className="icon-chip c2"><AlertCircle size={15} /></div>
          </div>
          <div className="value">{pendingOrders}</div>
          <div className={`delta ${pendingOrders ? '' : 'flat'}`}>{pendingOrders ? 'needs a status update' : 'all caught up'}</div>
        </div>
        <div className="glass pop kpi">
          <div className="kpi-head">
            <div className="label" style={{ display: 'flex', alignItems: 'center', gap: 5 }}>
              Settled revenue
              <InfoTip>Total of orders marked delivered — not orders in progress.</InfoTip>
            </div>
            <div className="icon-chip c3"><TrendingUp size={15} /></div>
          </div>
          <div className="value">P{revenue.toFixed(2)}</div>
          <Sparkline points={revenueTrend} color="var(--chip-teal)" />
        </div>
      </div>

      <div className="glass card">
        <div className="card-head">
          <h2>Recent orders</h2>
          <span>{orders?.length ?? 0} total</span>
        </div>
        {!orders?.length ? (
          <EmptyState icon={<Package size={28} />} title="No orders yet" hint="Orders placed by stores will show up here." />
        ) : (
          <table>
            <thead>
              <tr>
                <th>Order</th>
                <th>Status</th>
                <th>Total</th>
                <th>Placed</th>
              </tr>
            </thead>
            <tbody>
              {orders.slice(0, 8).map((o) => (
                <tr key={o.id}>
                  <td>{o.id.slice(0, 8)}</td>
                  <td>
                    <StatusPill status={o.status} />
                  </td>
                  <td>
                    {o.currency} {o.total.toFixed(2)}
                  </td>
                  <td>{new Date(o.placed_at).toLocaleDateString()}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>
    </div>
  );
}

function NoWarehouse() {
  return (
    <div className="view">
      <div className="glass card empty">
        <WarehouseIcon size={28} />
        <strong>No warehouse registered</strong>
        <p style={{ margin: 0 }}>Register a warehouse from onboarding to start publishing stock.</p>
      </div>
    </div>
  );
}

/** Up to `MAX_CATALOG_IMAGES` photos for one catalogue item: upload, pick a
 *  thumbnail, reorder, remove. Every action round-trips through the API and
 *  replaces the whole item (images included) via `onChange` — there's no
 *  local reordering state to keep in sync with the server. */
function ImageManager({
  warehouseId,
  item,
  onClose,
  onChange,
}: {
  warehouseId: string;
  item: CatalogItem;
  onClose: () => void;
  onChange: (item: CatalogItem) => void;
}) {
  const { push } = useToast();
  const [uploading, setUploading] = useState(false);
  const images = [...item.images].sort((a, b) => a.sort_order - b.sort_order);

  async function handleFile(file: File) {
    setUploading(true);
    try {
      onChange(await uploadCatalogImage(warehouseId, item.id, file));
    } catch {
      push('Could not upload photo', 'error');
    } finally {
      setUploading(false);
    }
  }

  async function move(imageId: string, direction: -1 | 1) {
    const ids = images.map((i) => i.id);
    const from = ids.indexOf(imageId);
    const to = from + direction;
    if (to < 0 || to >= ids.length) return;
    [ids[from], ids[to]] = [ids[to], ids[from]];
    try {
      onChange(
        await api.put<CatalogItem>(`/v1/warehouses/${warehouseId}/catalog/${item.id}/images/order`, {
          image_ids: ids,
        }),
      );
    } catch {
      push('Could not reorder photos', 'error');
    }
  }

  async function setThumbnail(imageId: string) {
    try {
      onChange(
        await api.put<CatalogItem>(
          `/v1/warehouses/${warehouseId}/catalog/${item.id}/images/${imageId}/thumbnail`,
        ),
      );
    } catch {
      push('Could not set thumbnail', 'error');
    }
  }

  async function remove(imageId: string) {
    try {
      onChange(
        await api.delete<CatalogItem>(`/v1/warehouses/${warehouseId}/catalog/${item.id}/images/${imageId}`),
      );
    } catch {
      push('Could not remove photo', 'error');
    }
  }

  const iconBtnStyle = {
    padding: 3,
    borderRadius: 8,
    border: '1.5px solid var(--ink)',
    background: 'var(--card)',
    display: 'grid',
    placeItems: 'center',
  } as const;

  return (
    <div className="palette-scrim" onClick={onClose}>
      <div
        className="glass pop palette"
        style={{ padding: 22, width: 'min(560px, calc(100vw - 32px))' }}
        onClick={(e) => e.stopPropagation()}
      >
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: 4 }}>
          <h1 style={{ fontSize: 16, margin: 0 }}>Photos — {item.name}</h1>
          <button className="btn ghost" style={{ padding: 6 }} onClick={onClose} aria-label="Close">
            <X size={14} />
          </button>
        </div>
        <p style={{ margin: '0 0 14px', fontSize: 12.5, color: 'var(--muted)' }}>
          Up to {MAX_CATALOG_IMAGES} photos. The starred one is the thumbnail stores see first.
        </p>

        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(96px, 1fr))', gap: 10, marginBottom: 16 }}>
          {images.map((img, i) => (
            <div
              key={img.id}
              style={{
                position: 'relative',
                border: 'var(--bw) solid var(--ink)',
                borderRadius: 'var(--radius-sm)',
                overflow: 'hidden',
                aspectRatio: '1',
                background: 'var(--card)',
              }}
            >
              <img src={img.url} alt="" style={{ width: '100%', height: '100%', objectFit: 'cover', display: 'block' }} />
              <button
                type="button"
                onClick={() => setThumbnail(img.id)}
                title={img.is_thumbnail ? 'Thumbnail' : 'Set as thumbnail'}
                style={{ ...iconBtnStyle, position: 'absolute', top: 4, left: 4, background: img.is_thumbnail ? 'var(--acid)' : 'var(--card)' }}
              >
                <Star size={11} fill={img.is_thumbnail ? 'var(--ink)' : 'none'} />
              </button>
              <button type="button" onClick={() => remove(img.id)} title="Remove photo" style={{ ...iconBtnStyle, position: 'absolute', top: 4, right: 4 }}>
                <X size={11} />
              </button>
              <div style={{ position: 'absolute', bottom: 4, left: 4, right: 4, display: 'flex', justifyContent: 'space-between' }}>
                <button type="button" onClick={() => move(img.id, -1)} disabled={i === 0} title="Move earlier" style={iconBtnStyle}>
                  <ChevronLeft size={11} />
                </button>
                <button type="button" onClick={() => move(img.id, 1)} disabled={i === images.length - 1} title="Move later" style={iconBtnStyle}>
                  <ChevronRight size={11} />
                </button>
              </div>
            </div>
          ))}
          {images.length < MAX_CATALOG_IMAGES && (
            <label
              style={{
                display: 'grid',
                placeItems: 'center',
                aspectRatio: '1',
                border: 'var(--bw) dashed var(--ink)',
                borderRadius: 'var(--radius-sm)',
                cursor: uploading ? 'wait' : 'pointer',
                color: 'var(--muted)',
              }}
            >
              {uploading ? <span style={{ fontSize: 11 }}>Uploading…</span> : <Plus size={18} />}
              <input
                type="file"
                accept="image/*"
                style={{ display: 'none' }}
                disabled={uploading}
                onChange={(e) => {
                  const file = e.target.files?.[0];
                  e.target.value = '';
                  if (file) handleFile(file);
                }}
              />
            </label>
          )}
        </div>
        <div className="form-actions">
          <button className="btn" type="button" onClick={onClose}>
            Done
          </button>
        </div>
      </div>
    </div>
  );
}

export function CatalogScreen() {
  const { data: warehouse } = useMyWarehouse();
  const qc = useQueryClient();
  const { push } = useToast();
  const [showForm, setShowForm] = useState(false);
  const [managingImagesFor, setManagingImagesFor] = useState<CatalogItem | null>(null);

  const { data: items, isLoading: loadingItems } = useQuery({
    queryKey: ['catalog', warehouse?.id],
    queryFn: () => api.get<CatalogItem[]>(`/v1/warehouses/${warehouse!.id}/catalog`),
    enabled: !!warehouse,
  });

  const publish = useMutation({
    mutationFn: (payload: Record<string, unknown>) =>
      api.post<CatalogItem>(`/v1/warehouses/${warehouse!.id}/catalog`, payload),
    onSuccess: (created) => {
      qc.invalidateQueries({ queryKey: ['catalog', warehouse?.id] });
      push('Item published — add some photos below');
      setShowForm(false);
      setManagingImagesFor(created);
    },
    onError: () => push('Could not publish item', 'error'),
  });

  function onImagesChanged(updated: CatalogItem) {
    setManagingImagesFor(updated);
    qc.setQueryData<CatalogItem[]>(['catalog', warehouse?.id], (items) =>
      items?.map((i) => (i.id === updated.id ? updated : i)),
    );
  }

  if (!warehouse) return <NoWarehouse />;

  return (
    <div className="view">
      <div className="glass card">
        <div className="card-head">
          <h2>Catalogue</h2>
          <button className="btn" onClick={() => setShowForm(true)}>
            <Plus size={14} /> New item
          </button>
        </div>

        {showForm && (
          <div className="palette-scrim" onClick={() => setShowForm(false)}>
            <div className="glass pop palette" style={{ padding: 22, width: 'min(560px, calc(100vw - 32px))' }} onClick={(e) => e.stopPropagation()}>
              <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: 14 }}>
                <h1 style={{ fontSize: 16, margin: 0 }}>Publish a catalogue item</h1>
                <button className="btn ghost" style={{ padding: 6 }} onClick={() => setShowForm(false)} aria-label="Close">
                  <X size={14} />
                </button>
              </div>
              <form
                onSubmit={(e) => {
                  e.preventDefault();
                  const form = new FormData(e.currentTarget);
                  publish.mutate({
                    sku: form.get('sku'),
                    name: form.get('name'),
                    currency: 'BWP',
                    unit_label: form.get('unit_label') || 'unit',
                    unit_price: Number(form.get('unit_price')),
                    case_size: form.get('case_size') ? Number(form.get('case_size')) : null,
                    case_price: form.get('case_price') ? Number(form.get('case_price')) : null,
                    stock_qty_units: Number(form.get('stock_qty_units')),
                  });
                }}
              >
                <div className="grid" style={{ gridTemplateColumns: '1fr 1fr', gap: 12 }}>
                  <div className="field">
                    <label>SKU</label>
                    <input name="sku" className="input" required />
                  </div>
                  <div className="field">
                    <label>Name</label>
                    <input name="name" className="input" required />
                  </div>
                  <div className="field">
                    <label>Unit label</label>
                    <input name="unit_label" className="input" placeholder="bag, unit, box" />
                  </div>
                  <div className="field">
                    <label>Unit price (BWP)</label>
                    <input name="unit_price" type="number" step="0.01" min="0" className="input" required />
                  </div>
                  <div className="field">
                    <label>Case size (units)</label>
                    <input name="case_size" type="number" min="1" className="input" />
                  </div>
                  <div className="field">
                    <label>Case price (BWP)</label>
                    <input name="case_price" type="number" step="0.01" min="0" className="input" />
                  </div>
                  <div className="field">
                    <label>Stock (units)</label>
                    <input name="stock_qty_units" type="number" step="0.01" min="0" className="input" required />
                  </div>
                </div>
                <div className="form-actions">
                  <button className="btn" type="submit" disabled={publish.isPending}>
                    {publish.isPending ? 'Publishing…' : 'Publish item'}
                  </button>
                  <button type="button" className="btn ghost" onClick={() => setShowForm(false)}>
                    Cancel
                  </button>
                </div>
              </form>
            </div>
          </div>
        )}

        {loadingItems ? (
          <CardSkeleton rows={3} />
        ) : !items?.length ? (
          <EmptyState icon={<Package size={28} />} title="No items yet" hint="Publish your first catalogue item above." />
        ) : (
          <table>
            <thead>
              <tr>
                <th></th>
                <th>SKU</th>
                <th>Name</th>
                <th>Unit price</th>
                <th>Case price</th>
                <th>Stock</th>
                <th>Status</th>
                <th></th>
              </tr>
            </thead>
            <tbody>
              {items.map((item) => (
                <tr key={item.id}>
                  <td style={{ width: 40 }}>
                    {item.thumbnail_url ? (
                      <img
                        src={item.thumbnail_url}
                        alt=""
                        style={{ width: 32, height: 32, borderRadius: 8, objectFit: 'cover', border: '1.5px solid var(--ink)', display: 'block' }}
                      />
                    ) : (
                      <div className="icon-chip c0" style={{ width: 32, height: 32 }}>
                        <ImageIcon size={14} />
                      </div>
                    )}
                  </td>
                  <td>{item.sku}</td>
                  <td>{item.name}</td>
                  <td>
                    {item.currency} {item.unit_price.toFixed(2)}/{item.unit_label}
                  </td>
                  <td>{item.case_price ? `${item.currency} ${item.case_price.toFixed(2)}` : '—'}</td>
                  <td>{item.stock_qty_units}</td>
                  <td>
                    <span className={`pill ${item.active ? 'ok' : ''}`}>{item.active ? 'Active' : 'Inactive'}</span>
                  </td>
                  <td style={{ textAlign: 'right' }}>
                    <button className="btn ghost" onClick={() => setManagingImagesFor(item)}>
                      Photos ({item.images.length}/{MAX_CATALOG_IMAGES})
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>

      {managingImagesFor && warehouse && (
        <ImageManager
          warehouseId={warehouse.id}
          item={managingImagesFor}
          onClose={() => setManagingImagesFor(null)}
          onChange={onImagesChanged}
        />
      )}
    </div>
  );
}

export function WarehouseOrdersScreen() {
  const { data: warehouse } = useMyWarehouse();
  const qc = useQueryClient();
  const { push } = useToast();

  const { data: orders, isLoading: loadingOrders } = useQuery({
    queryKey: ['warehouse-orders', warehouse?.id],
    queryFn: () => api.get<Order[]>(`/v1/warehouses/${warehouse!.id}/orders`),
    enabled: !!warehouse,
  });

  const advance = useMutation({
    mutationFn: ({ orderId, status }: { orderId: string; status: string }) =>
      api.patch(`/v1/warehouses/${warehouse!.id}/orders/${orderId}/status`, { status }),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['warehouse-orders', warehouse?.id] });
      push('Order updated');
    },
    onError: () => push('Could not update order', 'error'),
  });

  if (!warehouse) return <NoWarehouse />;

  const nextStatus: Record<string, string | undefined> = {
    pending: 'confirmed',
    confirmed: 'packed',
    packed: 'in_transit',
    in_transit: 'delivered',
  };

  return (
    <div className="view">
      <div className="glass card">
        <div className="card-head">
          <h2>Orders</h2>
          <span>{orders?.length ?? 0} total</span>
        </div>
        {loadingOrders ? (
          <CardSkeleton rows={3} />
        ) : !orders?.length ? (
          <EmptyState icon={<Package size={28} />} title="No orders yet" hint="Orders placed by stores will show up here." />
        ) : (
          <table>
            <thead>
              <tr>
                <th>Order</th>
                <th>Status</th>
                <th>Total</th>
                <th></th>
              </tr>
            </thead>
            <tbody>
              {orders.map((o) => {
                const next = nextStatus[o.status];
                return (
                  <tr key={o.id}>
                    <td>{o.id.slice(0, 8)}</td>
                    <td>
                      <StatusPill status={o.status} />
                    </td>
                    <td>
                      {o.currency} {o.total.toFixed(2)}
                    </td>
                    <td style={{ textAlign: 'right' }}>
                      {next && (
                        <button
                          className="btn ghost"
                          disabled={advance.isPending}
                          onClick={() => advance.mutate({ orderId: o.id, status: next })}
                        >
                          Mark {next.replace('_', ' ')}
                        </button>
                      )}
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        )}
      </div>
    </div>
  );
}

