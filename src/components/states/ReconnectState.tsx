import { LinkSlash } from "../icons";

export function ReconnectState({
  onConnect,
  busy,
  detail,
  error,
}: {
  onConnect: () => void;
  busy: boolean;
  detail: string | null;
  error: string | null;
}) {
  return (
    <div className="sw-body">
      <div className="sw-empty">
        <div className="sw-empty-ico">
          <LinkSlash />
        </div>
        <div className="sw-empty-title">Authorization expired</div>
        <div className="sw-empty-body">
          Stepwise lost access to Google Health. Reconnect to keep syncing steps from your Pixel.
        </div>
        <button className="sw-btn" onClick={onConnect} disabled={busy}>
          {busy ? "Connecting…" : "Reconnect Google Health"}
        </button>
        {detail && <div className="sw-subnote">Last synced {detail}</div>}
        {error && <div className="sw-err">{error}</div>}
      </div>
    </div>
  );
}
