import type { DaySummary } from "../types";

// Slim vertical bars, one per hour. The current hour (today's last bucket) is
// highlighted in the accent. No axis clutter — just the shape of the day.
export function HourlyBars({ day }: { day: DaySummary }) {
  const max = Math.max(1, ...day.hourly.map((h) => h.steps));
  const lastIdx = day.hourly.length - 1;
  return (
    <div className="sw-sec sw-hours-row">
      <div className="sw-sec-label">
        <span>{day.isToday ? "TODAY" : day.label.toUpperCase()}</span>
        <span>{day.isToday ? "since 12 AM" : "full day"}</span>
      </div>
      <div className="sw-hours">
        {day.hourly.map((h, i) => {
          const height = day.steps > 0 ? Math.max(3, Math.round((h.steps / max) * 48)) : 3;
          const now = day.isToday && i === lastIdx;
          return (
            <div
              key={h.hour}
              className={`sw-hour${now ? " now" : ""}`}
              style={{ height }}
              title={`${String(h.hour).padStart(2, "0")}:00 · ${h.steps.toLocaleString()} steps`}
            />
          );
        })}
      </div>
    </div>
  );
}
