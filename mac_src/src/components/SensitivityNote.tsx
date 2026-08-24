import { Lock, TriangleAlert } from "lucide-react";
import type { Sensitivity } from "../types";

export default function SensitivityNote({ sensitivity }: { sensitivity: Sensitivity }) {
  if (sensitivity === "normal") return null;
  if (sensitivity === "potential_secret") {
    return (
      <div className="sensitivity-note sensitivity-warning">
        <span className="sensitivity-icon">
          <TriangleAlert size={16} strokeWidth={1.75} />
        </span>
        This file may contain authentication tokens or other secrets.
      </div>
    );
  }
  return (
    <div className="sensitivity-note sensitivity-blocked">
      <span className="sensitivity-icon">
        <Lock size={16} strokeWidth={1.75} />
      </span>
      Private key detected. DotfileDesk will not track private keys automatically.
    </div>
  );
}
