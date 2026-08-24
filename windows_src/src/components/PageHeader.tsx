import { ReactNode } from "react";
import { useNavigate } from "react-router-dom";

interface PageHeaderProps {
  title: string;
  subtitle?: string;
  backTo?: string;
  backLabel?: string;
  /** Overrides the default `navigate(backTo)` — use to guard against navigating away with unsaved changes. */
  onBack?: () => void;
  actions?: ReactNode;
}

export default function PageHeader({ title, subtitle, backTo, backLabel, onBack, actions }: PageHeaderProps) {
  const navigate = useNavigate();
  return (
    <header className="page-header">
      <div className="page-header-top">
        {backTo && (
          <button className="back-link" onClick={() => (onBack ? onBack() : navigate(backTo))}>
            ‹ {backLabel ?? "Back"}
          </button>
        )}
        {actions && <div className="page-header-actions">{actions}</div>}
      </div>
      <h1 className="page-title">{title}</h1>
      {subtitle && <p className="page-subtitle">{subtitle}</p>}
    </header>
  );
}
