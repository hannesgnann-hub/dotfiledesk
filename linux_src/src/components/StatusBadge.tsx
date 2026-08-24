import type { Status } from "../types";
import { STATUS_LABELS } from "../types";

export default function StatusBadge({ status }: { status: Status }) {
  return <span className={`status-badge status-${status}`}>{STATUS_LABELS[status]}</span>;
}
