import type { DaySummary } from "../types";
import { fmtSleep, trend, type Trend } from "../format";
import { ChevronDown, ChevronUp } from "./icons";

interface MetricDef {
  key: string;
  label: string;
  unit: string;
  value: (d: DaySummary) => number | null;
  delta: (d: DaySummary) => number | null;
  format: (v: number) => string;
  deltaFormat: (x: number) => string;
  goodUp: boolean;
  eps: number;
}

const METRICS: MetricDef[] = [
  {
    key: "hr",
    label: "Resting HR",
    unit: "bpm",
    value: (d) => d.restingHr,
    delta: (d) => d.restingHrDelta,
    format: (v) => `${v}`,
    deltaFormat: (x) => `${x}`,
    goodUp: false,
    eps: 0.5,
  },
  {
    key: "sleep",
    label: "Sleep",
    unit: "",
    value: (d) => d.sleepMinutes,
    delta: (d) => d.sleepMinutesDelta,
    format: (v) => fmtSleep(v),
    deltaFormat: (x) => (x >= 60 ? `${Math.floor(x / 60)}h${x % 60 ? ` ${x % 60}m` : ""}` : `${x}m`),
    goodUp: true,
    eps: 0.5,
  },
  {
    key: "dist",
    label: "Distance",
    unit: "mi",
    value: (d) => d.distanceMi,
    delta: (d) => d.distanceMiDelta,
    format: (v) => v.toFixed(1),
    deltaFormat: (x) => x.toFixed(1),
    goodUp: true,
    eps: 0.05,
  },
  {
    key: "active",
    label: "Active",
    unit: "min",
    value: (d) => d.activeMinutes,
    delta: (d) => d.activeMinutesDelta,
    format: (v) => `${v}`,
    deltaFormat: (x) => `${x}`,
    goodUp: true,
    eps: 0.5,
  },
];

function TrendHint({ t }: { t: Trend }) {
  return (
    <span className={`sw-trend ${t.cls}`}>
      {t.dir === "up" && <ChevronUp />}
      {t.dir === "down" && <ChevronDown />}
      {t.label}
    </span>
  );
}

export function MetricCards({ day }: { day: DaySummary }) {
  return (
    <div className="sw-cards">
      {METRICS.map((m) => {
        const v = m.value(day);
        const t = trend(m.delta(day), m.goodUp, m.deltaFormat, m.eps);
        return (
          <div className="sw-card" key={m.key}>
            <div className="sw-card-label">{m.label}</div>
            <div className="sw-card-val">
              {v === null ? "—" : m.format(v)}
              {v !== null && m.unit && <span className="sw-card-unit">{m.unit}</span>}
            </div>
            {v !== null && <TrendHint t={t} />}
          </div>
        );
      })}
    </div>
  );
}
