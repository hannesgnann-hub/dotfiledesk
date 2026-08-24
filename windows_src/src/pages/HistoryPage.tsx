import { useEffect, useState } from "react";
import { useNavigate, useParams } from "react-router-dom";
import { api, errorMessage } from "../services/api";
import type { Configuration, Snapshot } from "../types";
import { formatDateTime, shortCommit } from "../services/format";
import PageHeader from "../components/PageHeader";

export default function HistoryPage() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const [configuration, setConfiguration] = useState<Configuration | null>(null);
  const [snapshots, setSnapshots] = useState<Snapshot[] | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!id) return;
    (async () => {
      try {
        const [detail, history] = await Promise.all([
          api.getConfigurationDetail(id),
          api.listHistory(id)
        ]);
        setConfiguration(detail?.configuration ?? null);
        setSnapshots(history);
      } catch (e) {
        setError(errorMessage(e));
      }
    })();
  }, [id]);

  if (!id) return null;

  return (
    <div className="page-content">
      <PageHeader
        title={configuration ? `${configuration.name} History` : "History"}
        backTo={`/configurations/${id}`}
        backLabel={configuration?.name ?? "Back"}
      />

      {error && <div className="banner banner-error">{error}</div>}

      {snapshots === null ? (
        <div className="empty-state">Loading…</div>
      ) : snapshots.length === 0 ? (
        <div className="empty-state">No snapshots yet. Create one from the configuration page.</div>
      ) : (
        <div className="history-list">
          {snapshots.map((snap) => (
            <button
              key={snap.id}
              className="history-row"
              onClick={() => navigate(`/configurations/${id}/history/${snap.git_commit}`)}
            >
              <div className="history-row-main">
                <div className="history-row-reason">{snap.reason}</div>
                <div className="history-row-date">{formatDateTime(snap.created_at)}</div>
              </div>
              <div className="history-row-commit">{shortCommit(snap.git_commit)}</div>
              <div className="config-row-chevron">›</div>
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
