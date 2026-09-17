import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { Bell, BellRing } from 'lucide-react';
import { api, type Notification } from '../lib/api';
import { CardSkeleton, EmptyState } from '../components/ui';

export function Notifications() {
  const qc = useQueryClient();
  const { data: notifications, isLoading } = useQuery({
    queryKey: ['notifications'],
    queryFn: () => api.get<Notification[]>('/v1/notifications'),
  });

  const markRead = useMutation({
    mutationFn: (id: string) => api.post(`/v1/notifications/${id}/read`),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['notifications'] }),
  });

  return (
    <div className="view">
      <div className="glass card">
        <div className="card-head">
          <h2>Notifications</h2>
          <span>{notifications?.filter((n) => !n.read_at).length ?? 0} unread</span>
        </div>
        {isLoading ? (
          <CardSkeleton rows={3} />
        ) : !notifications?.length ? (
          <EmptyState icon={<Bell size={28} />} title="No notifications" hint="You're all caught up." />
        ) : (
          <div className="grid" style={{ gap: 8 }}>
            {notifications.map((n) => (
              <div
                key={n.id}
                className="suggestion"
                style={{ background: n.read_at ? 'var(--line)' : 'var(--accent-soft)', cursor: n.read_at ? 'default' : 'pointer' }}
                onClick={() => !n.read_at && markRead.mutate(n.id)}
              >
                {n.read_at ? <Bell size={16} /> : <BellRing size={16} />}
                <div>
                  <p style={{ fontWeight: 600, marginBottom: 2 }}>{n.title}</p>
                  <p style={{ margin: 0 }}>{n.body}</p>
                  <small>{new Date(n.created_at).toLocaleString()}</small>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
