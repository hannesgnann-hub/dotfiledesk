import { useEffect, useRef, useState } from "react";
import { useNavigate } from "react-router-dom";
import { open } from "@tauri-apps/plugin-dialog";
import { api, errorMessage } from "../services/api";
import type { Category, ConfigurationView, DiscoveredConfig, PathPreview } from "../types";
import { CATEGORY_LABELS, CATEGORY_ORDER } from "../types";
import PageHeader from "../components/PageHeader";
import DiscoveredList from "../components/DiscoveredList";
import ConfirmDialog from "../components/ConfirmDialog";
import SensitivityNote from "../components/SensitivityNote";

export default function AddConfigurationPage() {
  const navigate = useNavigate();
  const [discovered, setDiscovered] = useState<DiscoveredConfig[]>([]);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [confirmItem, setConfirmItem] = useState<DiscoveredConfig | null>(null);
  const resolverRef = useRef<((value: boolean) => void) | null>(null);
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const [name, setName] = useState("");
  const [path, setPath] = useState("");
  const [category, setCategory] = useState<Category>("other");
  const [preview, setPreview] = useState<PathPreview | null>(null);
  const [customBusy, setCustomBusy] = useState(false);

  useEffect(() => {
    (async () => {
      try {
        const [found, existing]: [DiscoveredConfig[], ConfigurationView[]] = await Promise.all([
          api.scanConfigurations(),
          api.listConfigurations()
        ]);
        const trackedIds = new Set(existing.map((v) => v.configuration.definition_id).filter(Boolean));
        setDiscovered(found.filter((f) => !trackedIds.has(f.definition_id)));
      } catch (e) {
        setError(errorMessage(e));
      }
    })();
  }, []);

  useEffect(() => {
    if (!path.trim()) {
      setPreview(null);
      return;
    }
    const timer = setTimeout(async () => {
      try {
        setPreview(await api.previewCustomPath(path.trim()));
      } catch {
        setPreview(null);
      }
    }, 300);
    return () => clearTimeout(timer);
  }, [path]);

  function toggle(id: string) {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  }

  function askConfirm(item: DiscoveredConfig): Promise<boolean> {
    setConfirmItem(item);
    return new Promise((resolve) => {
      resolverRef.current = resolve;
    });
  }

  function respondConfirm(result: boolean) {
    resolverRef.current?.(result);
    resolverRef.current = null;
    setConfirmItem(null);
  }

  async function handleAddSelected() {
    setSubmitting(true);
    setError(null);
    for (const id of selected) {
      const item = discovered.find((i) => i.definition_id === id);
      if (!item) continue;
      let confirmed = true;
      if (item.sensitivity !== "normal") {
        confirmed = await askConfirm(item);
      }
      if (!confirmed) continue;
      try {
        await api.addDiscovered(id, confirmed);
      } catch (e) {
        setError(errorMessage(e));
      }
    }
    setSubmitting(false);
    navigate("/", { replace: true });
  }

  async function handleBrowse(directory: boolean) {
    const result = await open({ directory, multiple: false });
    if (typeof result === "string") {
      setPath(result);
      if (!name) {
        const parts = result.split("/");
        setName(parts[parts.length - 1] || result);
      }
    }
  }

  async function handleAddCustom() {
    if (!preview) return;
    setError(null);
    let confirmed = preview.sensitivity === "normal";
    if (!confirmed) {
      confirmed = await new Promise<boolean>((resolve) => {
        setConfirmItem({
          definition_id: "__custom__",
          application: name || path,
          category,
          kind: preview.is_directory ? "directory" : "file",
          path,
          sensitivity: preview.sensitivity,
          is_private_key: preview.is_private_key
        });
        resolverRef.current = resolve;
      });
    }
    if (!confirmed) return;
    setCustomBusy(true);
    try {
      await api.addCustom(name.trim() || path, path.trim(), category, confirmed);
      navigate("/", { replace: true });
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setCustomBusy(false);
    }
  }

  const customDisabled =
    !name.trim() || !path.trim() || !preview?.exists || preview?.is_private_key || customBusy;

  return (
    <div className="page-content">
      <PageHeader title="Add Configuration" backTo="/" backLabel="Configurations" />

      {error && <div className="banner banner-error">{error}</div>}

      {discovered.length > 0 && (
        <section className="add-section">
          <h2 className="section-heading">Discovered on this system</h2>
          <DiscoveredList items={discovered} selected={selected} onToggle={toggle} />
          <div className="section-actions">
            <button
              className="btn btn-primary"
              disabled={selected.size === 0 || submitting}
              onClick={handleAddSelected}
            >
              {submitting ? "Adding…" : `Add Selected (${selected.size})`}
            </button>
          </div>
        </section>
      )}

      <section className="add-section">
        <h2 className="section-heading">Add Custom Configuration</h2>
        <div className="form">
          <label className="form-field">
            <span className="form-label">Name</span>
            <input
              className="form-input"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="My Tool"
            />
          </label>
          <label className="form-field">
            <span className="form-label">Path</span>
            <div className="form-row">
              <input
                className="form-input"
                value={path}
                onChange={(e) => setPath(e.target.value)}
                placeholder="~/.config/mytool/config.toml"
              />
              <button type="button" className="btn btn-secondary" onClick={() => handleBrowse(false)}>
                File…
              </button>
              <button type="button" className="btn btn-secondary" onClick={() => handleBrowse(true)}>
                Folder…
              </button>
            </div>
          </label>
          <label className="form-field">
            <span className="form-label">Category</span>
            <select
              className="form-input"
              value={category}
              onChange={(e) => setCategory(e.target.value as Category)}
            >
              {CATEGORY_ORDER.map((c) => (
                <option key={c} value={c}>
                  {CATEGORY_LABELS[c]}
                </option>
              ))}
            </select>
          </label>

          {preview && !preview.exists && (
            <div className="banner banner-error">That path doesn't exist yet.</div>
          )}
          {preview && preview.exists && <SensitivityNote sensitivity={preview.sensitivity} />}

          <div className="section-actions">
            <button className="btn btn-primary" disabled={customDisabled} onClick={handleAddCustom}>
              {customBusy ? "Adding…" : "Add"}
            </button>
          </div>
        </div>
      </section>

      {confirmItem && (
        <ConfirmDialog
          title={confirmItem.application}
          message={<SensitivityNote sensitivity={confirmItem.sensitivity} />}
          confirmLabel="Track locally"
          cancelLabel="Cancel"
          onConfirm={() => respondConfirm(true)}
          onCancel={() => respondConfirm(false)}
        />
      )}
    </div>
  );
}
