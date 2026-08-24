import { ReactNode, useCallback, useEffect, useState } from "react";
import { useNavigate, useParams } from "react-router-dom";
import { openPath, revealItemInDir } from "@tauri-apps/plugin-opener";
import { api, errorMessage } from "../services/api";
import type { ConfigurationDetail } from "../types";
import { formatBytes, formatDate } from "../services/format";
import PageHeader from "../components/PageHeader";
import StatusBadge from "../components/StatusBadge";
import ConfirmDialog from "../components/ConfirmDialog";

export default function ConfigDetailPage() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const [detail, setDetail] = useState<ConfigurationDetail | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [confirmRemove, setConfirmRemove] = useState(false);

  const load = useCallback(async () => {
    if (!id) return;
    try {
      const result = await api.getConfigurationDetail(id);
      setDetail(result);
    } catch (e) {
      setError(errorMessage(e));
    }
  }, [id]);

  useEffect(() => {
    load();
  }, [load]);

  if (!id) return null;

  if (!detail) {
    return (
      <div className="page-content">
        <PageHeader title="Configurations" backTo="/" backLabel="Configurations" />
        {error ? <div className="banner banner-error">{error}</div> : <div className="empty-state">Loading…</div>}
      </div>
    );
  }

  const { configuration, status, size_bytes } = detail;

  async function handleSnapshot() {
    setBusy(true);
    setError(null);
    try {
      const snapshot = await api.snapshotConfiguration(configuration.id, "Manual snapshot");
      if (!snapshot) {
        setError("Nothing to snapshot — already matches the last snapshot.");
      }
      await load();
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setBusy(false);
    }
  }

  async function handleRemove() {
    setBusy(true);
    try {
      await api.removeConfiguration(configuration.id);
      navigate("/", { replace: true });
    } catch (e) {
      setError(errorMessage(e));
      setBusy(false);
    }
  }

  return (
    <div className="page-content">
      <PageHeader title={configuration.name} backTo="/" backLabel="Configurations" />

      <div className="detail-path">{configuration.path}</div>

      {error && <div className="banner banner-error">{error}</div>}

      {status === "missing" ? (
        <div className="detail-missing">
          <div className="banner banner-error">
            This configuration is missing from disk. The last known snapshot is still available.
          </div>
          <div className="detail-fact-list">
            <DetailFact label="Last known snapshot" value={formatDate(configuration.last_snapshot_at)} />
          </div>
          <div className="detail-actions">
            <button
              className="btn btn-primary"
              disabled={busy}
              onClick={() => navigate(`/configurations/${configuration.id}/history`)}
            >
              Restore
            </button>
            <button className="btn btn-danger" disabled={busy} onClick={() => setConfirmRemove(true)}>
              Remove from DotfileDesk
            </button>
          </div>
        </div>
      ) : (
        <>
          <div className="detail-fact-list">
            <DetailFact label="Status" value={<StatusBadge status={status} />} />
            <DetailFact label="Last snapshot" value={formatDate(configuration.last_snapshot_at)} />
            <DetailFact label="Tracked since" value={formatDate(configuration.added_at)} />
            <DetailFact label="Size" value={formatBytes(size_bytes)} />
          </div>

          <div className="detail-actions">
            <button
              className="btn btn-secondary"
              onClick={() => navigate(`/configurations/${configuration.id}/edit`)}
            >
              Edit
            </button>
            <button className="btn btn-secondary" onClick={() => openPath(configuration.path)}>
              Open
            </button>
            <button className="btn btn-secondary" onClick={() => revealItemInDir(configuration.path)}>
              Show in Finder
            </button>
            {status === "modified" && (
              <button
                className="btn btn-secondary"
                onClick={() => navigate(`/configurations/${configuration.id}/diff`)}
              >
                View Changes
              </button>
            )}
            <button className="btn btn-primary" disabled={busy} onClick={handleSnapshot}>
              {busy ? "Snapshotting…" : "Create Snapshot"}
            </button>
          </div>

          <div className="detail-secondary-actions">
            <button
              className="link-button"
              onClick={() => navigate(`/configurations/${configuration.id}/history`)}
            >
              View history
            </button>
            <button className="link-button link-button-danger" onClick={() => setConfirmRemove(true)}>
              Remove from DotfileDesk
            </button>
          </div>
        </>
      )}

      {confirmRemove && (
        <ConfirmDialog
          title="Remove from DotfileDesk?"
          message="DotfileDesk will stop tracking this configuration. The file on disk is not touched, and existing snapshots stay in your local history."
          confirmLabel="Remove"
          danger
          onConfirm={() => {
            setConfirmRemove(false);
            handleRemove();
          }}
          onCancel={() => setConfirmRemove(false)}
        />
      )}
    </div>
  );
}

function DetailFact({ label, value }: { label: string; value: ReactNode }) {
  return (
    <div className="detail-fact">
      <span className="detail-fact-label">{label}</span>
      <span className="detail-fact-value">{value}</span>
    </div>
  );
}
