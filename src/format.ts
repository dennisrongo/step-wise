import type { DaySummary } from "./types";

export const nf = (n: number): string => Math.round(n).toLocaleString("en-US");

export const fmtSleep = (min: number): string =>
  `${Math.floor(min / 60)}h ${String(min % 60).padStart(2, "0")}m`;

export interface ToGo {
  lead: string; // bold number, may be empty
  tail: string;
}

export function toGo(day: DaySummary): ToGo {
  const rem = day.goal - day.steps;
  if (day.isToday) {
    return rem > 0 ? { lead: nf(rem), tail: " to go" } : { lead: "", tail: "Goal reached" };
  }
  if (day.steps >= day.goal) return { lead: "", tail: "Goal reached" };
  return { lead: nf(rem), tail: " short of goal" };
}

export interface Trend {
  cls: "good" | "bad" | "flat";
  dir: "up" | "down" | "flat";
  label: string;
}

export function trend(
  delta: number | null,
  goodUp: boolean,
  fmt: (x: number) => string,
  eps = 0.5,
): Trend {
  if (delta === null) return { cls: "flat", dir: "flat", label: "— vs yest" };
  if (Math.abs(delta) < eps) return { cls: "flat", dir: "flat", label: "no change" };
  const up = delta > 0;
  const good = up === goodUp;
  return { cls: good ? "good" : "bad", dir: up ? "up" : "down", label: fmt(Math.abs(delta)) };
}
