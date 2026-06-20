import { useEffect, useState, type ReactNode } from "react";

const R = 74;
const CIRC = 2 * Math.PI * R;

/** Thin circular progress ring. `pct` is 0..1. Fills from empty on mount. */
export function Ring({
  pct,
  showProgress = true,
  children,
}: {
  pct: number;
  showProgress?: boolean;
  children: ReactNode;
}) {
  const target = CIRC * (1 - Math.min(Math.max(pct, 0), 1));
  // Start empty, then transition to target on the next frame so the ring
  // animates in (the CSS transition on .sw-ring-prog does the easing).
  const [offset, setOffset] = useState(CIRC);
  useEffect(() => {
    const id = requestAnimationFrame(() => setOffset(target));
    return () => cancelAnimationFrame(id);
  }, [target]);

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
