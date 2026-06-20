import { useEffect, useRef, useState } from "react";
import type { DaySummary } from "../types";
import { nf, toGo } from "../format";
import { Ring } from "./Ring";

// Count up to `target` over ~700ms with an ease-out curve.
function useCountUp(target: number): number {
  // Start at 0 so the count animates up on mount (and on each re-open, since
  // the dashboard is re-keyed when the window regains focus).
  const [value, setValue] = useState(0);
  const fromRef = useRef(0);
  useEffect(() => {
    const from = fromRef.current;
    const start = performance.now();
    let raf = 0;
    const tick = (t: number) => {
      const p = Math.min(1, (t - start) / 700);
      const eased = 1 - Math.pow(1 - p, 3);
      setValue(Math.round(from + (target - from) * eased));
      if (p < 1) raf = requestAnimationFrame(tick);
      else fromRef.current = target;
    };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  }, [target]);
  return value;
}

export function Hero({ day }: { day: DaySummary }) {
  const count = useCountUp(day.steps);
  const { lead, tail } = toGo(day);
  return (
    <div className="sw-hero">
      <Ring pct={day.steps / day.goal}>
        <div className="sw-steps">{nf(count)}</div>
        <div className="sw-steps-label">STEPS</div>
      </Ring>
      <div className="sw-togo">
        {lead && <b>{lead}</b>}
        {tail}
      </div>
    </div>
  );
}
