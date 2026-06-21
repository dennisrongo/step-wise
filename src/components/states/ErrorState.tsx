import { openExternal } from "../../openExternal";
import { WarningIcon } from "../icons";

// Where the user finishes linking their account to Google Health. Same entry
// point Google returns in the ACCOUNT_NOT_LINKED error metadata.
const HEALTH_SETUP_URL = "https://fitbit.google.com/auth/signup";

// Shown when Stepwise is connected (a token exists) but loading the week's data
// failed. This replaces the eternal spinner so the real reason — usually a
// Google API / permissions error — is visible and actionable.
export function ErrorState({
  onRetry,
  onReconnect,
  busy,
  error,
}: {
  onRetry: () => void;
  onReconnect: () => void;
  busy: boolean;
  error: string | null;
}) {
  // The account authorized fine but has no Google Health profile — the most
  // common first-run cause. The backend tags this with a stable token. The data
  // may live under a different Google account, so leading action is to switch;
  // linking this account or retrying are secondary.
  if (error && error.includes("ACCOUNT_NOT_LINKED")) {
    return (
      <div className="sw-body">
        <div className="sw-empty">
          <div className="sw-empty-ico">
            <WarningIcon />
          </div>
          <div className="sw-empty-title">This account isn't linked to Google Health</div>
          <div className="sw-empty-body">
            You authorized Stepwise, but this Google account has no Google Health data to read. Pick
            an account that's set up with Google Health — or link this one.
          </div>
          <button className="sw-btn" onClick={onReconnect} disabled={busy}>
            {busy ? "Opening…" : "Use a different account"}
          </button>
          <div className="sw-link-row">
            <button
              className="sw-link"
              onClick={() => void openExternal(HEALTH_SETUP_URL)}
              disabled={busy}
            >
              Set up Google Health
            </button>
            <button className="sw-link" onClick={onRetry} disabled={busy}>
              Try again
            </button>
          </div>
        </div>
      </div>
    );
  }

  // The saved token can't be decrypted on this machine — typically after an
  // update that changed the machine id (the wmic→registry switch on Windows) or
  // moving to a new device. Retrying can never succeed (the key won't match), so
  // the only action is a one-time reconnect; the backend has already cleared the
  // dead token. Lead with reconnect and omit the dead-end "Try again".
  if (error && error.includes("NEEDS_RECONNECT")) {
    return (
      <div className="sw-body">
        <div className="sw-empty">
          <div className="sw-empty-ico">
            <WarningIcon />
          </div>
          <div className="sw-empty-title">Reconnect to continue</div>
          <div className="sw-empty-body">
            Your saved Google sign-in couldn't be read on this device — this can happen after an
            update or when moving to a new machine. Reconnect once to refresh access.
          </div>
          <button className="sw-btn" onClick={onReconnect} disabled={busy}>
            {busy ? "Opening…" : "Reconnect Google Health"}
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="sw-body">
      <div className="sw-empty">
        <div className="sw-empty-ico">
          <WarningIcon />
        </div>
        <div className="sw-empty-title">Couldn't load your steps</div>
        <div className="sw-empty-body">
          Stepwise is connected to Google, but the request for your activity failed. Retry below — if
          it keeps failing, reconnect to refresh access.
        </div>
        <button className="sw-btn" onClick={onRetry} disabled={busy}>
          {busy ? "Retrying…" : "Try again"}
        </button>
        <button className="sw-link" onClick={onReconnect} disabled={busy}>
          Reconnect Google Health
        </button>
        {error && <div className="sw-err-detail">{error}</div>}
      </div>
    </div>
  );
}
