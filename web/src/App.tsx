import { lazy, Suspense, useState } from 'react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { AuthProvider, useAuth } from './lib/auth';
import { ToastProvider } from './lib/toast';
import { useCart } from './lib/hooks';
import { Shell, type ViewKey } from './components/Shell';
import { Login } from './screens/Login';
import { Onboarding } from './screens/Onboarding';
import { CardSkeleton, KpiSkeleton } from './components/ui';

// Route-level code splitting: each screen ships in its own chunk, fetched
// only when its nav item is first opened, instead of one bundle carrying
// every screen up front.
const WarehouseDashboard = lazy(() => import('./screens/WarehouseDashboard').then((m) => ({ default: m.WarehouseDashboard })));
const CatalogScreen = lazy(() => import('./screens/WarehouseDashboard').then((m) => ({ default: m.CatalogScreen })));
const WarehouseOrdersScreen = lazy(() => import('./screens/WarehouseDashboard').then((m) => ({ default: m.WarehouseOrdersScreen })));
const Marketplace = lazy(() => import('./screens/Marketplace').then((m) => ({ default: m.Marketplace })));
const Cart = lazy(() => import('./screens/Cart').then((m) => ({ default: m.Cart })));
const Orders = lazy(() => import('./screens/Orders').then((m) => ({ default: m.Orders })));
const Notifications = lazy(() => import('./screens/Notifications').then((m) => ({ default: m.Notifications })));

const queryClient = new QueryClient({
  defaultOptions: { queries: { retry: 1, staleTime: 10_000 } },
});

/** A user should never sit on a blank screen while a lazy chunk loads —
 *  this is what Suspense falls back to between navigations. */
function ScreenSkeleton() {
  return (
    <div className="view">
      <div className="grid kpis">
        <KpiSkeleton />
        <KpiSkeleton />
      </div>
      <div className="glass"><CardSkeleton rows={4} /></div>
    </div>
  );
}

export function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <ToastProvider>
        <AuthProvider>
          <Root />
        </AuthProvider>
      </ToastProvider>
    </QueryClientProvider>
  );
}

function Root() {
  const { session } = useAuth();

  if (!session) return <Login />;
  if (session.roles.length === 0) return <Onboarding />;
  return <AuthenticatedApp />;
}

function AuthenticatedApp() {
  const { session } = useAuth();
  const isWarehouse = session!.roles.includes('warehouse');
  const [view, setView] = useState<ViewKey>(isWarehouse ? 'dashboard' : 'marketplace');
  const { data: cartItems } = useCart();
  const goToMarketplace = () => setView('marketplace');

  return (
    <Shell view={view} onNavigate={setView} cartCount={cartItems?.length}>
      <Suspense fallback={<ScreenSkeleton />}>
        {view === 'dashboard' && <WarehouseDashboard />}
        {view === 'catalog' && <CatalogScreen />}
        {view === 'warehouse-orders' && <WarehouseOrdersScreen />}
        {view === 'marketplace' && <Marketplace />}
        {view === 'cart' && <Cart onCheckedOut={() => setView('orders')} onBrowse={goToMarketplace} />}
        {view === 'orders' && <Orders onBrowse={goToMarketplace} />}
        {view === 'notifications' && <Notifications />}
      </Suspense>
    </Shell>
  );
}
