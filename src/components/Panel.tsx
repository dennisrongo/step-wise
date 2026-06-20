import type { ReactNode } from "react";

export function Panel({ children }: { children: ReactNode }) {
  return <div className="sw-panel">{children}</div>;
}
