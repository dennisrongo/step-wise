import { useHealth } from "./hooks/useHealth";
import { Panel } from "./components/Panel";
import { Header } from "./components/Header";
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

  if (!status) {
    return (
      <Panel>
        <Header status={null} syncing />
        <Loading />
      </Panel>
    );
  }

  if (status.state === "reconnect") {
    return (
      <Panel>
        <Header status={status} syncing={syncing} />
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
            onReconnect={connect}
            onDisconnect={disconnect}
          />
          <ErrorState onRetry={refreshNow} onReconnect={connect} busy={syncing} error={error} />
        </Panel>
      );
    }
    return (
      <Panel>
        <Header status={status} syncing={syncing} onRefresh={refreshNow} />
        <Loading />
      </Panel>
    );
  }

  const zeroToday = selectedDay.isToday && selectedDay.steps === 0;

  return (
    <Panel>
      <Header
        status={status}
        syncing={syncing}
        onRefresh={refreshNow}
        onReconnect={connect}
        onDisconnect={disconnect}
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
