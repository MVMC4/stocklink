import { useMemo, useState, type ReactNode } from 'react';
import {
  Boxes,
  LayoutDashboard,
  LogOut,
  Plus,
  ShoppingCart,
  Store as StoreIcon,
  Truck,
  Bell,
  Receipt,
  Warehouse as WarehouseIcon,
} from 'lucide-react';
import { useAuth } from '../lib/auth';
import type { Role } from '../lib/api';
import { AddRoleModal } from './AddRoleModal';

export type ViewKey =
  | 'dashboard'
  | 'catalog'
  | 'warehouse-orders'
  | 'marketplace'
  | 'cart'
  | 'orders'
  | 'notifications';

interface NavItem {
  key: ViewKey;
  label: string;
  icon: ReactNode;
  roles: Role[];
}

const NAV_ITEMS: NavItem[] = [
  { key: 'dashboard', label: 'Dashboard', icon: <LayoutDashboard size={16} />, roles: ['warehouse'] },
  { key: 'catalog', label: 'Catalogue', icon: <Boxes size={16} />, roles: ['warehouse'] },
  { key: 'warehouse-orders', label: 'Orders', icon: <Receipt size={16} />, roles: ['warehouse'] },
  { key: 'marketplace', label: 'Marketplace', icon: <StoreIcon size={16} />, roles: ['store'] },
  { key: 'cart', label: 'Cart', icon: <ShoppingCart size={16} />, roles: ['store'] },
  { key: 'orders', label: 'My orders', icon: <Truck size={16} />, roles: ['store'] },
  { key: 'notifications', label: 'Notifications', icon: <Bell size={16} />, roles: ['warehouse', 'store'] },
];

/** Account avatar + roles + sign-out/add-role, as a standalone trigger so
 *  it can sit in the sidebar on desktop and move into the topbar on narrow
 *  viewports (see `.profile-anchor` — it, not this component, decides
 *  where the popover opens from). */
function ProfileMenu() {
  const { session, logout } = useAuth();
  const [open, setOpen] = useState(false);
  const [addRoleFor, setAddRoleFor] = useState<'warehouse' | 'store' | null>(null);

  return (
    <div className="profile-anchor">
      {open && (
        <>
          <div className="profile-scrim" onClick={() => setOpen(false)} />
          <div className="glass pop-sm profile-popover">
            <div className="pp-head">
              <div className="email">{session?.accountId.slice(0, 8)}…</div>
              <div className="roles">
                {(session?.roles ?? []).map((r) => (
                  <span key={r} className="pill accent">
                    {r}
                  </span>
                ))}
              </div>
            </div>
            {!session?.roles.includes('warehouse') && (
              <button className="pp-item" onClick={() => { setAddRoleFor('warehouse'); setOpen(false); }}>
                <Plus size={14} /> Add warehouse role
              </button>
            )}
            {!session?.roles.includes('store') && (
              <button className="pp-item" onClick={() => { setAddRoleFor('store'); setOpen(false); }}>
                <Plus size={14} /> Add store role
              </button>
            )}
            <button className="pp-item danger" onClick={() => void logout()}>
              <LogOut size={14} /> Sign out
            </button>
          </div>
        </>
      )}
      <button className="profile-trigger" onClick={() => setOpen((o) => !o)}>
        <span className="avatar">{session?.accountId.slice(0, 2).toUpperCase()}</span>
        <span className="who">
          <div className="email">{session?.accountId.slice(0, 10)}…</div>
          <div className="roles">
            {(session?.roles ?? []).map((r) => (
              <span key={r} className="pill accent" style={{ fontSize: 10.5, padding: '2px 7px' }}>
                {r === 'warehouse' ? <WarehouseIcon size={10} /> : <StoreIcon size={10} />}
              </span>
            ))}
          </div>
        </span>
      </button>
      {addRoleFor && <AddRoleModal missing={addRoleFor} onClose={() => setAddRoleFor(null)} />}
    </div>
  );
}

export function Shell({
  view,
  onNavigate,
  cartCount,
  children,
}: {
  view: ViewKey;
  onNavigate: (view: ViewKey) => void;
  cartCount?: number;
  children: ReactNode;
}) {
  const { session } = useAuth();

  const items = useMemo(() => {
    const roles = session?.roles ?? [];
    return NAV_ITEMS.filter((item) => item.roles.some((r) => roles.includes(r)));
  }, [session]);

  const activeLabel = items.find((i) => i.key === view)?.label ?? 'StockLink';

  return (
    <div className="app-root">
      <div className="shell">
        <nav className="sidebar glass">
          <div className="brand">
            <span className="brand-mark">SL</span>
            StockLink
          </div>
          <div className="nav">
            {items.map((item) => (
              <button
                key={item.key}
                aria-current={view === item.key ? 'page' : undefined}
                onClick={() => onNavigate(item.key)}
              >
                {item.icon}
                {item.label}
                {item.key === 'cart' && !!cartCount && (
                  <span className="pill accent" style={{ marginLeft: 'auto' }}>
                    {cartCount}
                  </span>
                )}
              </button>
            ))}
          </div>
          <div className="sidebar-foot desktop-only">
            <ProfileMenu />
          </div>
        </nav>
        <div className="main">
          <div className="topbar glass">
            <h1>{activeLabel}</h1>
            <div className="spacer" />
            <div className="mobile-only">
              <ProfileMenu />
            </div>
          </div>
          {children}
        </div>
      </div>
    </div>
  );
}
