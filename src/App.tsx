import { useState } from "react";
import { useHealth } from "./hooks/useHealth";
import { useOpenSignal } from "./useOpenSignal";
import { Panel } from "./components/Panel";
import { Header } from "./components/Header";
import { Settings } from "./components/Settings";
import { ConnectedView } from "./components/ConnectedView";
import { ReconnectState } from "./components/states/ReconnectState";
import { NoDataState } from "./components/states/NoDataState";
import { ErrorState } from "./components/states/ErrorState";

function Loading() {
  return (
    <div className="sw-body">
      <div className="sw-empty">
        <span className="sw-spin" />
      </div>
    </div>
  );
}

export default function App() {
  const {
    status,
    week,
    selected,
    selectedDay,
    setSelected,
    syncing,
    error,
    connect,
    disconnect,
    refreshNow,
  } = useHealth();

  const [showSettings, setShowSettings] = useState(false);
  const openSettings = () => setShowSettings(true);
  const openSignal = useOpenSignal();

  if (!status) {
    return (
      <Panel>
        <Header status={null} syncing />
        <Loading />
      </Panel>
    );
  }

  // Settings is reachable from any connected/reconnect state via the kebab menu.
  if (showSettings) {
    return (
      <Panel>
        <Settings
          status={status}
          onBack={() => setShowSettings(false)}
          onReconnect={connect}
          onDisconnect={disconnect}
          onActiveModeChange={refreshNow}
        />
      </Panel>
    );
  }

  if (status.state === "reconnect") {
    return (
      <Panel>
        <Header status={status} syncing={syncing} onSettings={openSettings} />
        <ReconnectState
          onConnect={connect}
          busy={syncing}
          detail={status.lastSyncedDetail}
          error={error}
        />
      </Panel>
    );
  }

  if (!week || !selectedDay) {
    // Connected, but the data request failed: show the real error and let the
    // user retry, rather than spinning forever with the cause hidden.
    if (error) {
      return (
        <Panel>
          <Header
            status={status}
            syncing={syncing}
            onRefresh={refreshNow}
            onSettings={openSettings}
          />
          <ErrorState onRetry={refreshNow} onReconnect={connect} busy={syncing} error={error} />
        </Panel>
      );
    }
    return (
      <Panel>
        <Header status={status} syncing={syncing} onRefresh={refreshNow} onSettings={openSettings} />
        <Loading />
      </Panel>
    );
  }

  const zeroToday = selectedDay.isToday && selectedDay.steps === 0;

  return (
    <Panel key={openSignal}>
      <Header
        status={status}
        syncing={syncing}
        onRefresh={refreshNow}
        onSettings={openSettings}
      />
      {zeroToday ? (
        <NoDataState week={week} selected={selected} onSelect={setSelected} />
      ) : (
        <ConnectedView
          day={selectedDay}
          week={week}
          selected={selected}
          onSelect={setSelected}
          dim={syncing}
        />
      )}
    </Panel>
  );
}
