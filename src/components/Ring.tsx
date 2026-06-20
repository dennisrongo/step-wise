import type { ReactNode } from "react";

const R = 74;
const CIRC = 2 * Math.PI * R;

/** Thin circular progress ring. `pct` is 0..1. */
export function Ring({
  pct,
  showProgress = true,
  children,
}: {
  pct: number;
  showProgress?: boolean;
  children: ReactNode;
}) {
  const offset = CIRC * (1 - Math.min(Math.max(pct, 0), 1));
  return (
    <div className="sw-ring-wrap">
      <svg width="168" height="168" viewBox="0 0 168 168">
        <circle cx="84" cy="84" r={R} fill="none" stroke="var(--p-track)" strokeWidth="7" />
        {showProgress && (
          <circle
            className="sw-ring-prog"
            cx="84"
            cy="84"
            r={R}
            fill="none"
            stroke="var(--accent)"
            strokeWidth="7"
            strokeLinecap="round"
            transform="rotate(-90 84 84)"
            style={{ strokeDasharray: CIRC, strokeDashoffset: offset }}
          />
        )}
      </svg>
      <div className="sw-ring-num">{children}</div>
    </div>
  );
}
