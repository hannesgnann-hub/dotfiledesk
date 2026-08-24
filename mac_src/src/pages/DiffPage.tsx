import { useCallback, useEffect, useState } from "react";
import { useNavigate, useParams } from "react-router-dom";
import { api, errorMessage } from "../services/api";
import type { Configuration, DiffResult, Snapshot } from "../types";
import { formatDateTime } from "../services/format";
import PageHeader from "../components/PageHeader";
import DiffView from "../components/DiffView";
import ConfirmDialog from "../components/ConfirmDialog";

export default function DiffPage({ mode }: { mode: "working" | "snapshot" }) {
  const { id, commit } = useParams<{ id: string; commit?: string }>();
  const navigate = useNavigate();
  const [configuration, setConfiguration] = useState<Configuration | null>(null);
  const [snapshot, setSnapshot] = useState<Snapshot | null>(null);
  const [diff, setDiff] = useState<DiffResult | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [confirmRestore, setConfirmRestore] = useState(false);
  const [restoring, setRestoring] = useState(false);

  const load = useCallback(async () => {
    if (!id) return;
    try {
      const detail = await api.getConfigurationDetail(id);
      setConfiguration(detail?.configuration ?? null);
      if (mode === "working") {
        setDiff(await api.diffWorking(id));
      } else {
        const [result, history] = await Promise.all([
          api.diffSnapshot(id, commit!),
          api.listHistory(id)
        ]);
        setDiff(result);
        setSnapshot(history.find((s) => s.git_commit === commit) ?? null);
      }
    } catch (e) {
      setError(errorMessage(e));
    }
  }, [id, commit, mode]);

  useEffect(() => {
    load();
  }, [load]);

  if (!id) return null;

  async function handleRestore() {
    if (!commit) return;
    setRestoring(true);
    try {
      await api.restoreSnapshot(id!, commit);
      navigate(`/configurations/${id}`, { replace: true });
    } catch (e) {
      setError(errorMessage(e));
      setRestoring(false);
    }
  }

  const backTo = `/configurations/${id}`;

  return (
    <div className="page-content">
      <PageHeader
        title={mode === "working" ? "Changes" : snapshot?.reason ?? "Snapshot"}
        subtitle={mode === "working" ? configuration?.name : formatDateTime(snapshot?.created_at ?? null)}
        backTo={backTo}
        backLabel={configuration?.name ?? "Back"}
      />

      {error && <div className="banner banner-error">{error}</div>}

      {diff === null ? (
        <div className="empty-state">Loading…</div>
      ) : (
        <>
          <DiffView diff={diff} />
          {mode === "snapshot" && (
            <div className="detail-actions">
              <button className="btn btn-primary" disabled={restoring} onClick={() => setConfirmRestore(true)}>
                {restoring ? "Restoring…" : "Restore this Version"}
              </button>
            </div>
          )}
        </>
      )}

      {confirmRestore && (
        <ConfirmDialog
          title="Restore this version?"
          message="Current file will be backed up before restore."
          confirmLabel="Restore"
          onConfirm={() => {
            setConfirmRestore(false);
            handleRestore();
          }}
          onCancel={() => setConfirmRestore(false)}
        />
      )}
    </div>
  );
}
