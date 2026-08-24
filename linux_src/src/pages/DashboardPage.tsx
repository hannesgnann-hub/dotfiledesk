import { useCallback, useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { api, errorMessage } from "../services/api";
import type { ConfigurationView } from "../types";
import { CATEGORY_LABELS, CATEGORY_ORDER } from "../types";
import CategorySection from "../components/CategorySection";

export default function DashboardPage() {
  const [views, setViews] = useState<ConfigurationView[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [banner, setBanner] = useState<string | null>(null);
  const [snapshotting, setSnapshotting] = useState(false);

  const load = useCallback(async () => {
    try {
      const list = await api.listConfigurations();
      setViews(list);
    } catch (e) {
      setError(errorMessage(e));
    }
  }, []);

  useEffect(() => {
    load();
  }, [load]);

  async function handleSnapshotAll() {
    setSnapshotting(true);
    setBanner(null);
    try {
      const result = await api.snapshotAll();
      if (result.snapshotted.length === 0) {
        setBanner(`Snapshot complete. All ${result.unchanged_count} configurations unchanged.`);
      } else {
        const names = result.snapshotted.map((s) => s.name).join(", ");
        setBanner(
          `Snapshot complete. ${result.snapshotted.length} new snapshot${
            result.snapshotted.length === 1 ? "" : "s"
          } (${names}). ${result.unchanged_count} unchanged.`
        );
      }
      await load();
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setSnapshotting(false);
    }
  }

  if (views === null) {
    return (
      <div className="page-content">
        <div className="empty-state">Loading…</div>
      </div>
    );
  }

  if (views.length === 0) {
    return (
      <div className="page-content dashboard-empty">
        <h1 className="page-title">DotfileDesk</h1>
        <p className="empty-state-message">
          Nothing is managed yet. Scan your system to find configuration files already sitting where
          your tools expect them.
        </p>
        <Link className="btn btn-primary" to="/onboarding">
          Scan for configurations
        </Link>
      </div>
    );
  }

  const modifiedCount = views.filter((v) => v.status === "modified").length;

  return (
    <div className="page-content">
      <header className="page-header">
        <div className="page-header-top">
          <div />
          <div className="page-header-actions">
            <button className="btn btn-secondary" onClick={handleSnapshotAll} disabled={snapshotting}>
              {snapshotting ? "Snapshotting…" : "Snapshot All"}
            </button>
            <Link className="btn btn-icon" to="/add" aria-label="Add configuration">
              +
            </Link>
          </div>
        </div>
        <h1 className="page-title">Configurations</h1>
        <p className="page-subtitle">
          {views.length} configuration{views.length === 1 ? "" : "s"}
          {modifiedCount > 0 ? ` · ${modifiedCount} modified` : ""}
        </p>
      </header>

      {error && <div className="banner banner-error">{error}</div>}
      {banner && <div className="banner banner-info">{banner}</div>}

      <div className="category-list">
        {CATEGORY_ORDER.map((category) => (
          <CategorySection
            key={category}
            title={CATEGORY_LABELS[category]}
            items={views.filter((v) => v.configuration.category === category)}
          />
        ))}
      </div>
    </div>
  );
}
