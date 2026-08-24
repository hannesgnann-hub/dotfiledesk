import { useNavigate } from "react-router-dom";
import type { ConfigurationView } from "../types";

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

export default function ConfigRow({ view }: { view: ConfigurationView }) {
  const navigate = useNavigate();
  const { configuration } = view;

  return (
    <button className="config-row" onClick={() => navigate(`/configurations/${configuration.id}`)}>
      <div className="config-row-main">
        <div className="config-row-name">{configuration.name}</div>
        <div className="config-row-path">{configuration.path}</div>
      </div>
      <div className={`config-row-status status-text-${view.status}`}>{statusText(view)}</div>
      <div className="config-row-chevron">›</div>
    </button>
  );
}
