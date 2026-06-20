import { useState } from "react";
import type { SyncStatus } from "../types";
import { RefreshIcon, KebabIcon } from "./icons";

// Honest freshness: a sync stamp or a spinner — never a live ticking counter.
// The kebab menu is the always-available account control (switch account /
// disconnect), so those actions aren't trapped behind an error state.
export function Header({
  status,
  syncing,
  onRefresh,
  onReconnect,
  onDisconnect,
}: {
  status: SyncStatus | null;
  syncing: boolean;
  onRefresh?: () => void;
  onReconnect?: () => void;
  onDisconnect?: () => void;
}) {
  const [menuOpen, setMenuOpen] = useState(false);
  const showMenu = !!status && (!!onReconnect || !!onDisconnect);

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

  const choose = (fn?: () => void) => () => {
    setMenuOpen(false);
    fn?.();
  };

  return (
    <div className="sw-head">
      <div className="sw-name">Stepwise</div>
      <div className="sw-status">
        {right}
        {showMenu && (
          <div className="sw-menu-wrap">
            <button
              className="sw-refresh"
              onClick={() => setMenuOpen((o) => !o)}
              aria-label="Account menu"
              aria-haspopup="menu"
              aria-expanded={menuOpen}
              title="Account"
            >
              <KebabIcon />
            </button>
            {menuOpen && (
              <>
                <div className="sw-menu-scrim" onClick={() => setMenuOpen(false)} />
                <div className="sw-menu" role="menu">
                  {onReconnect && (
                    <button className="sw-menu-item" role="menuitem" onClick={choose(onReconnect)}>
                      Switch account…
                    </button>
                  )}
                  {onDisconnect && (
                    <button
                      className="sw-menu-item danger"
                      role="menuitem"
                      onClick={choose(onDisconnect)}
                    >
                      Disconnect
                    </button>
                  )}
                </div>
              </>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
