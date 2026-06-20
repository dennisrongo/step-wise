import type { DaySummary, WeekSummary } from "../types";
import { Hero } from "./Hero";
import { HourlyBars } from "./HourlyBars";
import { MetricCards } from "./MetricCards";
import { WeekStrip } from "./WeekStrip";

export function ConnectedView({
  day,
  week,
  selected,
  onSelect,
  dim,
}: {
  day: DaySummary;
  week: WeekSummary;
  selected: number;
  onSelect: (i: number) => void;
  dim: boolean;
}) {
  return (
    <div className={`sw-body${dim ? " dim" : ""}`}>
      <Hero day={day} />
      <HourlyBars day={day} />
      <MetricCards day={day} />
      <WeekStrip week={week} selected={selected} onSelect={onSelect} />
    </div>
  );
}
