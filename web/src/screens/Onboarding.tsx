import { useState } from 'react';
import { Store as StoreIcon, Warehouse as WarehouseIcon } from 'lucide-react';
import { api, type Warehouse, type Store } from '../lib/api';
import { useAuth } from '../lib/auth';
import { useToast } from '../lib/toast';

export function Onboarding() {
  const [kind, setKind] = useState<'warehouse' | 'store' | null>(null);
  const { refreshRoles } = useAuth();
  const { push } = useToast();
  const [name, setName] = useState('');
  const [region, setRegion] = useState('');
  const [busy, setBusy] = useState(false);

  async function submit(e: React.FormEvent) {
    e.preventDefault();
    if (!kind) return;
    setBusy(true);
    try {
      if (kind === 'warehouse') {
        await api.post<Warehouse>('/v1/onboarding/warehouses', { name, region });
        refreshRoles(['warehouse']);
      } else {
        await api.post<Store>('/v1/onboarding/stores', { name, region });
        refreshRoles(['store']);
      }
      push(`Registered as a ${kind}`);
    } catch {
      push('Could not register — try again', 'error');
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="auth-screen">
      <div className="glass auth-card" style={{ width: 460 }}>
        <div className="brand">
          <span className="brand-mark">SL</span>
          StockLink
        </div>
        {!kind ? (
          <>
            <h1>How will you use StockLink?</h1>
            <p className="lead">You can add another role later from your profile menu.</p>
            <div className="grid" style={{ gap: 10 }}>
              <button className="glass card" style={{ textAlign: 'left' }} onClick={() => setKind('warehouse')}>
                <div style={{ display: 'flex', gap: 12, alignItems: 'center' }}>
                  <WarehouseIcon size={20} />
                  <div>
                    <div style={{ fontWeight: 600 }}>I run a warehouse</div>
                    <div className="meta" style={{ color: 'var(--faint)', fontSize: 12.5 }}>
                      Publish stock, accept orders, manage settlements.
                    </div>
                  </div>
                </div>
              </button>
              <button className="glass card" style={{ textAlign: 'left' }} onClick={() => setKind('store')}>
                <div style={{ display: 'flex', gap: 12, alignItems: 'center' }}>
                  <StoreIcon size={20} />
                  <div>
                    <div style={{ fontWeight: 600 }}>I run a retail store</div>
                    <div className="meta" style={{ color: 'var(--faint)', fontSize: 12.5 }}>
                      Order stock, join bulk orders, track deliveries.
                    </div>
                  </div>
                </div>
              </button>
            </div>
          </>
        ) : (
          <>
            <h1>{kind === 'warehouse' ? 'Register your warehouse' : 'Register your store'}</h1>
            <form onSubmit={submit}>
              <div className="field">
                <label htmlFor="name">Name</label>
                <input id="name" className="input" required value={name} onChange={(e) => setName(e.target.value)} />
              </div>
              <div className="field">
                <label htmlFor="region">Region</label>
                <input
                  id="region"
                  className="input"
                  required
                  placeholder="Gaborone"
                  value={region}
                  onChange={(e) => setRegion(e.target.value)}
                />
              </div>
              <div className="form-actions">
                <button className="btn" type="submit" disabled={busy}>
                  {busy ? 'Registering…' : 'Continue'}
                </button>
                <button type="button" className="btn ghost" onClick={() => setKind(null)}>
                  Back
                </button>
              </div>
            </form>
          </>
        )}
      </div>
    </div>
  );
}
