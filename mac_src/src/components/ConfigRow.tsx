import { useState } from "react";
import { useNavigate } from "react-router-dom";
import type { ConfigurationView } from "../types";
import { api, errorMessage } from "../services/api";

function statusText(view: ConfigurationView): string {
  switch (view.status) {
    case "synced":
      return "Synced";
    case "modified":
      return "Modified";
    case "missing":
      return "Missing";
    case "warning":
      return "Needs attention";
    case "not_tracked":
    default:
      return "Not managed";
  }
}

interface ConfigRowProps {
  view: ConfigurationView;
  onArchived?: () => void;
}

export default function ConfigRow({ view, onArchived }: ConfigRowProps) {
  const navigate = useNavigate();
  const { configuration } = view;
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function handleArchive(e: React.MouseEvent) {
    e.stopPropagation();
    setBusy(true);
    setError(null);
    try {
      await api.archiveConfiguration(configuration.id);
      onArchived?.();
    } catch (err) {
      setError(errorMessage(err));
      setBusy(false);
    }
  }

  return (
    <div className="config-row">
      <button
        className="config-row-main config-row-link"
        onClick={() => navigate(`/configurations/${configuration.id}`)}
      >
        <div className="config-row-text">
          <div className="config-row-name">{configuration.name}</div>
          <div className="config-row-path">{configuration.path}</div>
        </div>
        <div className={`config-row-status status-text-${view.status}`}>{statusText(view)}</div>
      </button>
      {error && <span className="config-row-error" title={error}>⚠</span>}
      <button className="icon-btn" title="Archive" disabled={busy} onClick={handleArchive}>
        {"\u{1F5C4}\u{FE0F}"}
      </button>
      <button
        className="config-row-chevron-btn"
        aria-label="Open"
        onClick={() => navigate(`/configurations/${configuration.id}`)}
      >
        <span className="config-row-chevron">›</span>
      </button>
    </div>
  );
}
