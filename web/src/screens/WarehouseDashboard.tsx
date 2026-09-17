import { useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { AlertCircle, Boxes, Package, Plus, TrendingUp, Warehouse as WarehouseIcon, X } from 'lucide-react';
import { api, type CatalogItem, type Order } from '../lib/api';
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

export function CatalogScreen() {
  const { data: warehouse } = useMyWarehouse();
  const qc = useQueryClient();
  const { push } = useToast();
  const [showForm, setShowForm] = useState(false);

  const { data: items, isLoading: loadingItems } = useQuery({
    queryKey: ['catalog', warehouse?.id],
    queryFn: () => api.get<CatalogItem[]>(`/v1/warehouses/${warehouse!.id}/catalog`),
    enabled: !!warehouse,
  });

  const publish = useMutation({
    mutationFn: (payload: Record<string, unknown>) =>
      api.post<CatalogItem>(`/v1/warehouses/${warehouse!.id}/catalog`, payload),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['catalog', warehouse?.id] });
      push('Item published');
      setShowForm(false);
    },
    onError: () => push('Could not publish item', 'error'),
  });

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
                <th>SKU</th>
                <th>Name</th>
                <th>Unit price</th>
                <th>Case price</th>
                <th>Stock</th>
                <th>Status</th>
              </tr>
            </thead>
            <tbody>
              {items.map((item) => (
                <tr key={item.id}>
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
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>
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

