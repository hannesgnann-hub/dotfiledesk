import type { DiffResult } from "../types";

const CHANGE_LABELS: Record<string, string> = {
  added: "New",
  modified: "Changed",
  deleted: "Deleted"
};

export default function DiffView({ diff }: { diff: DiffResult }) {
  if (diff.files.length === 0) {
    return <div className="empty-state">No differences.</div>;
  }

  return (
    <div className="diff-view">
      {diff.files.map((file) => (
        <div className="diff-file" key={file.path}>
          <div className="diff-file-header">
            <span className={`diff-file-tag diff-file-tag-${file.change}`}>
              {CHANGE_LABELS[file.change] ?? file.change}
            </span>
            <span className="diff-file-path">{file.path}</span>
          </div>
          {file.binary ? (
            <div className="diff-binary-note">Binary file — contents not shown.</div>
          ) : (
            <pre className="diff-lines">
              {file.lines.map((line, idx) => (
                <div key={idx} className={`diff-line diff-line-${line.tag}`}>
                  <span className="diff-line-marker">
                    {line.tag === "added" ? "+" : line.tag === "removed" ? "-" : " "}
                  </span>
                  <span className="diff-line-content">{line.content}</span>
                </div>
              ))}
            </pre>
          )}
        </div>
      ))}
    </div>
  );
}
