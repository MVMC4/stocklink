/** Shared, cross-screen UI primitives — kept out of any one screen file so
 *  App.tsx's `React.lazy()` code-splitting for the screens actually works.
 *  (A shared piece imported both statically, here, and dynamically, via a
 *  screen's lazy import, silently defeats the split — Vite folds the whole
 *  module into the main chunk instead of the screen's own chunk. Every
 *  screen file below now exports only its screen component(s).) */
import { Info } from 'lucide-react';
import type { ReactNode } from 'react';

/** A small info affordance with a hover/focus tooltip — CSS-only, no
 *  portal or positioning JS needed for a note this short-lived. */
export function InfoTip({ children }: { children: ReactNode }) {
  return (
    <span className="tooltip" tabIndex={0}>
      <Info size={13} />
      <span className="tip">{children}</span>
    </span>
  );
}

/** A small honest trend line: real cumulative values, no fabricated data.
 *  Renders flat when there isn't enough real history to show a slope yet —
 *  that's the truth, not a placeholder pretending otherwise. Single stroke
 *  colour, no fill, no gradient. */
export function Sparkline({ points, color = 'var(--accent)' }: { points: number[]; color?: string }) {
  const w = 84;
  const h = 28;
  const series = points.length ? points : [0, 0];
  const max = Math.max(...series, 1);
  const min = Math.min(...series, 0);
  const range = max - min || 1;
  const step = w / Math.max(series.length - 1, 1);
  const d = series.map((v, i) => `${i * step},${h - ((v - min) / range) * h}`).join(' ');
  return (
    <svg className="spark" width={w} height={h} viewBox={`0 0 ${w} ${h}`} aria-hidden="true">
      <polyline points={d} fill="none" stroke={color} strokeWidth="1.75" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}

export function StatusPill({ status }: { status: string }) {
  const variant = status === 'delivered' ? 'ok' : status === 'cancelled' ? 'warn' : 'accent';
  return <span className={`pill ${variant}`}>{status.replace('_', ' ')}</span>;
}

export function EmptyState({
  icon,
  title,
  hint,
  action,
}: {
  icon: ReactNode;
  title: string;
  hint: string;
  /** A user should never land on a dead end — where there's somewhere
   *  useful to go from an empty screen, this puts the button right there. */
  action?: { label: string; onClick: () => void };
}) {
  return (
    <div className="empty">
      {icon}
      <strong>{title}</strong>
      {hint && <p style={{ margin: 0 }}>{hint}</p>}
      {action && (
        <button className="btn" style={{ marginTop: 6 }} onClick={action.onClick}>
          {action.label}
        </button>
      )}
    </div>
  );
}

/** Skeleton placeholder for a card mid-load — a user should never sit on a
 *  blank screen while a query is in flight. Flat shimmer bars only, no
 *  gradient sweep (rule 8) — the "shimmer" here is a plain opacity pulse. */
export function CardSkeleton({ rows = 3 }: { rows?: number }) {
  return (
    <div className="skeleton-card" aria-busy="true" aria-label="Loading">
      {Array.from({ length: rows }).map((_, i) => (
        <div key={i} className="skeleton-line" style={{ width: `${85 - i * 15}%` }} />
      ))}
    </div>
  );
}

export function KpiSkeleton() {
  return (
    <div className="glass kpi skeleton-card" aria-busy="true" aria-label="Loading">
      <div className="skeleton-line" style={{ width: '50%', height: 10 }} />
      <div className="skeleton-line" style={{ width: '70%', height: 22 }} />
    </div>
  );
}
