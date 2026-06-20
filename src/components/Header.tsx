import type { SyncStatus } from "../types";
import { RefreshIcon } from "./icons";

// Honest freshness: a sync stamp or a spinner — never a live ticking counter.
export function Header({
  status,
  syncing,
  onRefresh,
}: {
  status: SyncStatus | null;
  syncing: boolean;
  onRefresh?: () => void;
}) {
  let right;
  if (syncing) {
    right = (
      <>
        <span className="sw-spin" /> Syncing…
      </>
    );
  } else if (!status || status.state === "reconnect") {
    right = (
      <>
        <span className="sw-dot warn" /> Not connected
      </>
    );
  } else {
    right = (
      <>
        <span className="sw-dot" /> Synced {status.lastSyncedLabel ?? "just now"}
        {onRefresh && (
          <button className="sw-refresh" onClick={onRefresh} aria-label="Refresh now" title="Refresh now">
            <RefreshIcon />
          </button>
        )}
      </>
    );
  }
  return (
    <div className="sw-head">
      <div className="sw-name">Stepwise</div>
      <div className="sw-status">{right}</div>
    </div>
  );
}
