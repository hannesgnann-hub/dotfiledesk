import { useEffect, useRef, useState } from "react";
import { useNavigate } from "react-router-dom";
import { api, errorMessage } from "../services/api";
import type { DiscoveredConfig } from "../types";
import DiscoveredList from "../components/DiscoveredList";
import ConfirmDialog from "../components/ConfirmDialog";
import SensitivityNote from "../components/SensitivityNote";

export default function OnboardingPage() {
  const navigate = useNavigate();
  const [phase, setPhase] = useState<"scanning" | "review">("scanning");
  const [items, setItems] = useState<DiscoveredConfig[]>([]);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [submitting, setSubmitting] = useState(false);
  const [confirmItem, setConfirmItem] = useState<DiscoveredConfig | null>(null);
  const resolverRef = useRef<((value: boolean) => void) | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const timer = setTimeout(async () => {
      try {
        const found = await api.scanConfigurations();
        setItems(found);
        setSelected(new Set(found.filter((i) => !i.is_private_key).map((i) => i.definition_id)));
      } catch (e) {
        setError(errorMessage(e));
      } finally {
        setPhase("review");
      }
    }, 650);
    return () => clearTimeout(timer);
  }, []);

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

  async function handleManageSelected() {
    setSubmitting(true);
    setError(null);
    for (const id of selected) {
      const item = items.find((i) => i.definition_id === id);
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

  if (phase === "scanning") {
    return (
      <div className="onboarding-screen">
        <div className="onboarding-brand">DotfileDesk</div>
        <div className="onboarding-status">
          <div className="spinner" />
          Scanning your system...
        </div>
      </div>
    );
  }

  return (
    <div className="onboarding-screen onboarding-review">
      <div className="onboarding-brand">DotfileDesk</div>
      <h1 className="onboarding-heading">Found configurations</h1>
      <p className="onboarding-subheading">
        Nothing is changed until you choose to manage it. Uncheck anything you'd rather leave alone.
      </p>
      {error && <div className="banner banner-error">{error}</div>}
      <div className="onboarding-list-wrap">
        <DiscoveredList items={items} selected={selected} onToggle={toggle} />
      </div>
      <div className="onboarding-actions">
        <button className="btn btn-ghost" onClick={() => navigate("/", { replace: true })}>
          Skip for now
        </button>
        <button
          className="btn btn-primary"
          disabled={selected.size === 0 || submitting}
          onClick={handleManageSelected}
        >
          {submitting ? "Adding…" : `Manage Selected (${selected.size})`}
        </button>
      </div>

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
