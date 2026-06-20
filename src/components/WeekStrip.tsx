import type { WeekSummary } from "../types";

// Last 7 days. Today carries a small accent dot; the selected day's bar is the
// accent. Selecting a day swaps the hero + cards (handled by the parent).
export function WeekStrip({
  week,
  selected,
  onSelect,
  interactive = true,
}: {
  week: WeekSummary;
  selected: number;
  onSelect: (i: number) => void;
  interactive?: boolean;
}) {
  const max = Math.max(1, ...week.days.map((d) => d.steps));
  return (
    <div className="sw-sec">
      <div className="sw-sec-label">
        <span>LAST 7 DAYS</span>
      </div>
      <div className="sw-week">
        {week.days.map((d, i) => {
          const height = Math.max(4, Math.round((d.steps / max) * 42));
          const cls = `sw-day${i === selected ? " sel" : ""}${d.isToday ? " today" : ""}`;
          return (
            <button
              key={d.date}
              className={cls}
              onClick={interactive ? () => onSelect(i) : undefined}
              disabled={!interactive}
              aria-label={`${d.label}, ${d.steps.toLocaleString()} steps`}
            >
              <div className="sw-day-track">
                <div className="sw-day-bar" style={{ height }} />
              </div>
              <div className="sw-day-lbl">{d.label}</div>
            </button>
          );
        })}
      </div>
    </div>
  );
}
