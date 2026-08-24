import { useCallback, useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { Archive } from "lucide-react";
import { api, errorMessage } from "../services/api";
import type { ConfigurationView, DashboardSummary } from "../types";
import { CATEGORY_LABELS, CATEGORY_ORDER } from "../types";
import { formatBytes } from "../services/format";
import CategorySection from "../components/CategorySection";
import StatCard from "../components/StatCard";

export default function DashboardPage() {
  const [views, setViews] = useState<ConfigurationView[] | null>(null);
  const [summary, setSummary] = useState<DashboardSummary | null>(null);
  const [archivedCount, setArchivedCount] = useState(0);
  const [error, setError] = useState<string | null>(null);
  const [banner, setBanner] = useState<string | null>(null);
  const [snapshotting, setSnapshotting] = useState(false);

  const load = useCallback(async () => {
    try {
      const [list, archived, stats] = await Promise.all([
        api.listConfigurations(),
        api.listArchivedConfigurations(),
        api.dashboardSummary()
      ]);
      setViews(list);
      setArchivedCount(archived.length);
      setSummary(stats);
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
        <h1 className="page-title dashboard-brand">DotfileDesk</h1>
        <p className="page-subtitle">Your developer configuration, under control.</p>
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

  return (
    <div className="page-content dashboard-wide">
      <header className="page-header">
        <div className="page-header-top">
          <div />
          <div className="page-header-actions">
            {archivedCount > 0 && (
              <Link className="btn btn-secondary" to="/archived">
                <Archive size={15} strokeWidth={1.75} />
                Archived ({archivedCount})
              </Link>
            )}
            <button className="btn btn-secondary" onClick={handleSnapshotAll} disabled={snapshotting}>
              {snapshotting ? "Snapshotting…" : "Snapshot All"}
            </button>
            <Link className="btn btn-icon" to="/add" aria-label="Add configuration">
              +
            </Link>
          </div>
        </div>
        <h1 className="page-title dashboard-brand">DotfileDesk</h1>
        <p className="page-subtitle">Your developer configuration, under control.</p>
      </header>

      {error && <div className="banner banner-error">{error}</div>}
      {banner && <div className="banner banner-info">{banner}</div>}

      {summary && (
        <div className="stat-card-row">
          <StatCard label="Configurations" value={summary.configuration_count} />
          <StatCard label="Files tracked" value={summary.file_count} />
          <StatCard label="Total size" value={formatBytes(summary.total_size_bytes)} />
          <StatCard label="Modified" value={summary.modified_count} />
          <StatCard label="Snapshots" value={summary.snapshot_count} />
        </div>
      )}

      <h2 className="section-heading dashboard-list-heading">Configurations</h2>
      <div className="category-list">
        {CATEGORY_ORDER.map((category) => (
          <CategorySection
            key={category}
            title={CATEGORY_LABELS[category]}
            items={views.filter((v) => v.configuration.category === category)}
            onArchived={load}
          />
        ))}
      </div>
    </div>
  );
}
