import { useQuery } from '@tanstack/react-query';
import { Truck } from 'lucide-react';
import { api, type Order, type OrderStatus } from '../lib/api';
import { CardSkeleton, EmptyState, StatusPill } from '../components/ui';

const MILESTONES: OrderStatus[] = ['pending', 'confirmed', 'packed', 'in_transit', 'delivered'];

export function Orders({ onBrowse }: { onBrowse: () => void }) {
  const { data: orders, isLoading } = useQuery({
    queryKey: ['my-orders'],
    queryFn: () => api.get<Order[]>('/v1/orders'),
  });

  return (
    <div className="view">
      <div className="glass card">
        <div className="card-head">
          <h2>My orders</h2>
          <span>{orders?.length ?? 0} total</span>
        </div>
        {isLoading ? (
          <CardSkeleton rows={3} />
        ) : !orders?.length ? (
          <EmptyState
            icon={<Truck size={28} />}
            title="No orders yet"
            hint="Orders you place will appear here."
            action={{ label: 'Browse marketplace', onClick: onBrowse }}
          />
        ) : (
          <div className="grid" style={{ gap: 14 }}>
            {orders.map((o) => (
              <div key={o.id} className="glass card" style={{ padding: 16 }}>
                <div className="card-head" style={{ marginBottom: 10 }}>
                  <div>
                    <strong>Order {o.id.slice(0, 8)}</strong>
                    <span style={{ marginLeft: 10 }}>
                      <StatusPill status={o.status} />
                    </span>
                  </div>
                  <span>
                    {o.currency} {o.total.toFixed(2)}
                  </span>
                </div>
                <OrderTimeline status={o.status} />
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

function OrderTimeline({ status }: { status: OrderStatus }) {
  if (status === 'cancelled') {
    return (
      <ul className="timeline">
        <li className="done">
          <span className="dot" /> Cancelled <span></span>
        </li>
      </ul>
    );
  }
  const currentIndex = MILESTONES.indexOf(status);
  return (
    <ul className="timeline">
      {MILESTONES.map((m, i) => (
        <li key={m} className={i < currentIndex ? 'done' : i === currentIndex ? 'now' : ''}>
          <span className="dot" />
          {m.replace('_', ' ')}
          <span></span>
        </li>
      ))}
    </ul>
  );
}
