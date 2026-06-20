import type { WeekSummary } from "../../types";
import { Ring } from "../Ring";
import { WeekStrip } from "../WeekStrip";

export function NoDataState({
  week,
  selected,
  onSelect,
}: {
  week: WeekSummary;
  selected: number;
  onSelect: (i: number) => void;
}) {
  return (
    <div className="sw-body">
      <div className="sw-hero ghost">
        <Ring pct={0} showProgress={false}>
          <div className="sw-steps">0</div>
          <div className="sw-steps-label">STEPS</div>
        </Ring>
        <div className="sw-togo">No steps yet today</div>
      </div>
      <div className="sw-inline-note">
        Your activity shows up here as soon as your Pixel syncs with Google Health. Time for a short walk?
      </div>
      <WeekStrip week={week} selected={selected} onSelect={onSelect} />
    </div>
  );
}
