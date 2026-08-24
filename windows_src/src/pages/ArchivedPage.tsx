import { useCallback, useEffect, useState } from "react";
import { useNavigate } from "react-router-dom";
import { api, errorMessage } from "../services/api";
import type { ConfigurationView } from "../types";
import PageHeader from "../components/PageHeader";

export default function ArchivedPage() {
  const navigate = useNavigate();
  const [views, setViews] = useState<ConfigurationView[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busyId, setBusyId] = useState<string | null>(null);

  const load = useCallback(async () => {
    try {
      setViews(await api.listArchivedConfigurations());
    } catch (e) {
      setError(errorMessage(e));
    }
  }, []);

  useEffect(() => {
    load();
  }, [load]);

  async function handleUnarchive(id: string) {
    setBusyId(id);
    try {
      await api.unarchiveConfiguration(id);
      await load();
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setBusyId(null);
    }
  }

  return (
    <div className="page-content">
      <PageHeader title="Archived" backTo="/" backLabel="Configurations" />

      {error && <div className="banner banner-error">{error}</div>}

      {views === null ? (
        <div className="empty-state">Loading…</div>
      ) : views.length === 0 ? (
        <div className="empty-state">Nothing archived. Archived configurations keep their history but stay off the dashboard.</div>
      ) : (
        <div className="config-list">
          {views.map((view) => (
            <div key={view.configuration.id} className="config-row config-row-archived">
              <button
                className="config-row-main config-row-link"
                onClick={() => navigate(`/configurations/${view.configuration.id}`)}
              >
                <div className="config-row-name">{view.configuration.name}</div>
                <div className="config-row-path">{view.configuration.path}</div>
              </button>
              <button
                className="btn btn-secondary"
                disabled={busyId === view.configuration.id}
                onClick={() => handleUnarchive(view.configuration.id)}
              >
                Unarchive
              </button>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
