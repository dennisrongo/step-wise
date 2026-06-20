import type { SyncStatus } from "../types";
import { RefreshIcon, GearIcon } from "./icons";

// Honest freshness: a sync stamp or a spinner — never a live ticking counter.
// The gear opens Settings, which is the always-available home for the account
// actions (switch account / disconnect), so they aren't trapped behind an
// error state.
export function Header({
  status,
  syncing,
  onRefresh,
  onSettings,
}: {
  status: SyncStatus | null;
  syncing: boolean;
  onRefresh?: () => void;
  onSettings?: () => void;
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
      <div className="sw-status">
        {right}
        {status && onSettings && (
          <button
            className="sw-refresh"
            onClick={onSettings}
            aria-label="Settings"
            title="Settings"
          >
            <GearIcon />
          </button>
        )}
      </div>
    </div>
  );
}
