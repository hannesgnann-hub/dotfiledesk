import type { ConfigurationView } from "../types";
import ConfigRow from "./ConfigRow";

export default function CategorySection({
  title,
  items
}: {
  title: string;
  items: ConfigurationView[];
}) {
  if (items.length === 0) return null;
  return (
    <section className="category-section">
      <h2 className="category-title">{title}</h2>
      <div className="config-list">
        {items.map((view) => (
          <ConfigRow key={view.configuration.id} view={view} />
        ))}
      </div>
    </section>
  );
}
