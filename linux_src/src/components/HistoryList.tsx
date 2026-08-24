import { useCallback, useEffect, useState } from "react";
import { useNavigate } from "react-router-dom";
import { api, errorMessage } from "../services/api";
import type { Snapshot } from "../types";
import { formatDateTime, shortCommit } from "../services/format";
import ConfirmDialog from "./ConfirmDialog";

interface HistoryListProps {
  configurationId: string;
  /** Called after a restore, since that can change the configuration's live status. */
  onRestored?: () => void;
}

type PendingAction =
  | { kind: "restore"; snapshot: Snapshot }
  | { kind: "delete"; snapshot: Snapshot };

export default function HistoryList({ configurationId, onRestored }: HistoryListProps) {
  const navigate = useNavigate();
  const [snapshots, setSnapshots] = useState<Snapshot[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busyId, setBusyId] = useState<string | null>(null);
  const [showArchived, setShowArchived] = useState(false);
  const [pending, setPending] = useState<PendingAction | null>(null);

  const load = useCallback(async () => {
    try {
      setSnapshots(await api.listHistory(configurationId));
    } catch (e) {
      setError(errorMessage(e));
    }
  }, [configurationId]);

  useEffect(() => {
    load();
  }, [load]);

  async function toggleFavorite(snapshot: Snapshot) {
    setBusyId(snapshot.id);
    try {
      await api.favoriteSnapshot(snapshot.id, !snapshot.favorite);
      await load();
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setBusyId(null);
    }
  }

  async function toggleArchived(snapshot: Snapshot) {
    setBusyId(snapshot.id);
    try {
      await api.archiveSnapshot(snapshot.id, !snapshot.archived);
      await load();
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setBusyId(null);
    }
  }

  async function handleRestore(snapshot: Snapshot) {
    setBusyId(snapshot.id);
    try {
      await api.restoreSnapshot(configurationId, snapshot.git_commit);
      await load();
      onRestored?.();
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setBusyId(null);
    }
  }

  async function handleDelete(snapshot: Snapshot) {
    setBusyId(snapshot.id);
    try {
      await api.deleteSnapshot(snapshot.id);
      await load();
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setBusyId(null);
    }
  }

  if (snapshots === null) {
    return <div className="empty-state">Loading history…</div>;
  }

  const archivedCount = snapshots.filter((s) => s.archived).length;
  const visible = snapshots.filter((s) => showArchived || !s.archived);

  return (
    <div className="history-section">
      {error && <div className="banner banner-error">{error}</div>}

      {visible.length === 0 ? (
        <div className="empty-state">No snapshots yet. Create one above.</div>
      ) : (
        <div className="history-list">
          {visible.map((snap) => (
            <div key={snap.id} className={`history-row ${snap.archived ? "history-row-archived" : ""}`}>
              <button
                className="icon-btn icon-btn-star"
                title={snap.favorite ? "Unfavorite" : "Favorite"}
                disabled={busyId === snap.id}
                onClick={() => toggleFavorite(snap)}
              >
                {snap.favorite ? "★" : "☆"}
              </button>

              <button
                className="history-row-main"
                onClick={() => navigate(`/configurations/${configurationId}/history/${snap.git_commit}`)}
              >
                <div className="history-row-reason">{snap.reason}</div>
                <div className="history-row-date">{formatDateTime(snap.created_at)}</div>
              </button>

              <div className="history-row-commit">{shortCommit(snap.git_commit)}</div>

              <div className="history-row-actions">
                <button
                  className="icon-btn"
                  title="Edit current file"
                  onClick={() => navigate(`/configurations/${configurationId}/edit`)}
                >
                  ✎
                </button>
                <button
                  className="icon-btn"
                  title="Restore this version"
                  disabled={busyId === snap.id}
                  onClick={() => setPending({ kind: "restore", snapshot: snap })}
                >
                  ↺
                </button>
                <button
                  className="icon-btn"
                  title={snap.archived ? "Unarchive" : "Archive"}
                  disabled={busyId === snap.id}
                  onClick={() => toggleArchived(snap)}
                >
                  {snap.archived ? "\u{1F4E4}" : "\u{1F5C4}\u{FE0F}"}
                </button>
                <button
                  className="icon-btn icon-btn-danger"
                  title="Delete this snapshot"
                  disabled={busyId === snap.id}
                  onClick={() => setPending({ kind: "delete", snapshot: snap })}
                >
                  🗑
                </button>
              </div>
            </div>
          ))}
        </div>
      )}

      {archivedCount > 0 && (
        <button className="link-button history-archived-toggle" onClick={() => setShowArchived((v) => !v)}>
          {showArchived ? "Hide archived" : `Show archived (${archivedCount})`}
        </button>
      )}

      {pending?.kind === "restore" && (
        <ConfirmDialog
          title="Restore this version?"
          message="Current file will be backed up before restore."
          confirmLabel="Restore"
          onConfirm={() => {
            const snap = pending.snapshot;
            setPending(null);
            handleRestore(snap);
          }}
          onCancel={() => setPending(null)}
        />
      )}

      {pending?.kind === "delete" && (
        <ConfirmDialog
          title="Delete this snapshot?"
          message="This removes it from DotfileDesk's history permanently. This can't be undone."
          confirmLabel="Delete"
          danger
          onConfirm={() => {
            const snap = pending.snapshot;
            setPending(null);
            handleDelete(snap);
          }}
          onCancel={() => setPending(null)}
        />
      )}
    </div>
  );
}
