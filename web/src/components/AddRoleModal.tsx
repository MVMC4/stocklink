import { useState } from 'react';
import { Store as StoreIcon, Warehouse as WarehouseIcon, X } from 'lucide-react';
import { api, type Store, type Warehouse } from '../lib/api';
import { useAuth } from '../lib/auth';
import { useToast } from '../lib/toast';

/** The same "register as a warehouse/store" flow Onboarding.tsx uses for a
 *  brand-new account, reused here so an existing account can pick up its
 *  *other* role without signing out — this is what Onboarding's own "You
 *  can add another role later from settings" line promised but nothing
 *  previously delivered on. */
export function AddRoleModal({ missing, onClose }: { missing: 'warehouse' | 'store'; onClose: () => void }) {
  const { refreshRoles } = useAuth();
  const { push } = useToast();
  const [name, setName] = useState('');
  const [region, setRegion] = useState('');
  const [busy, setBusy] = useState(false);

  async function submit(e: React.FormEvent) {
    e.preventDefault();
    setBusy(true);
    try {
      if (missing === 'warehouse') {
        await api.post<Warehouse>('/v1/onboarding/warehouses', { name, region });
      } else {
        await api.post<Store>('/v1/onboarding/stores', { name, region });
      }
      refreshRoles([missing]);
      push(`Registered as a ${missing}`);
      onClose();
    } catch {
      push('Could not register — try again', 'error');
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="palette-scrim" onClick={onClose}>
      <div className="glass pop palette" style={{ padding: 22, width: 'min(420px, calc(100vw - 32px))' }} onClick={(e) => e.stopPropagation()}>
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: 6 }}>
          <h1 style={{ fontSize: 16, margin: 0, display: 'flex', alignItems: 'center', gap: 8 }}>
            {missing === 'warehouse' ? <WarehouseIcon size={17} /> : <StoreIcon size={17} />}
            Register your {missing}
          </h1>
          <button className="btn ghost" style={{ padding: 6 }} onClick={onClose} aria-label="Close">
            <X size={14} />
          </button>
        </div>
        <p className="lead" style={{ textAlign: 'left', margin: '0 0 18px' }}>
          Adds the {missing} role to this account — your existing role stays too.
        </p>
        <form onSubmit={submit}>
          <div className="field">
            <label htmlFor="add-role-name">Name</label>
            <input id="add-role-name" className="input" required value={name} onChange={(e) => setName(e.target.value)} />
          </div>
          <div className="field">
            <label htmlFor="add-role-region">Region</label>
            <input
              id="add-role-region"
              className="input"
              required
              placeholder="Gaborone"
              value={region}
              onChange={(e) => setRegion(e.target.value)}
            />
          </div>
          <div className="form-actions">
            <button className="btn" type="submit" disabled={busy}>
              {busy ? 'Registering…' : 'Add role'}
            </button>
            <button type="button" className="btn ghost" onClick={onClose}>
              Cancel
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
