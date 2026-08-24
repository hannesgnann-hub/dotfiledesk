import type { DiscoveredConfig } from "../types";
import { CATEGORY_LABELS } from "../types";

interface DiscoveredListProps {
  items: DiscoveredConfig[];
  selected: Set<string>;
  onToggle: (definitionId: string) => void;
}

export default function DiscoveredList({ items, selected, onToggle }: DiscoveredListProps) {
  if (items.length === 0) {
    return <div className="empty-state">No new configurations found.</div>;
  }

  return (
    <div className="discovered-list">
      {items.map((item) => {
        const blocked = item.is_private_key;
        const isChecked = selected.has(item.definition_id);
        return (
          <label
            key={item.definition_id}
            className={`discovered-row ${blocked ? "discovered-row-blocked" : ""}`}
          >
            <input
              type="checkbox"
              checked={isChecked}
              disabled={blocked}
              onChange={() => onToggle(item.definition_id)}
            />
            <div className="discovered-row-main">
              <div className="discovered-row-title">
                {item.application}
                {item.sensitivity !== "normal" && (
                  <span className={`sensitivity-pill sensitivity-pill-${item.sensitivity}`}>
                    {item.is_private_key ? "Private key" : "Sensitive"}
                  </span>
                )}
              </div>
              <div className="discovered-row-path">{item.path}</div>
            </div>
            <div className="discovered-row-category">{CATEGORY_LABELS[item.category]}</div>
          </label>
        );
      })}
    </div>
  );
}
