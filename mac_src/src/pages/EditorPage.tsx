import { useCallback, useEffect, useState } from "react";
import { useNavigate, useParams } from "react-router-dom";
import { Sparkles } from "lucide-react";
import { api, errorMessage } from "../services/api";
import type { Configuration, SnippetSuggestion } from "../types";
import PageHeader from "../components/PageHeader";
import ConfirmDialog from "../components/ConfirmDialog";

export default function EditorPage() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();

  const [configuration, setConfiguration] = useState<Configuration | null>(null);
  const [files, setFiles] = useState<string[] | null>(null);
  const [selectedFile, setSelectedFile] = useState<string | null>(null);

  const [content, setContent] = useState("");
  const [originalContent, setOriginalContent] = useState("");
  const [isBinary, setIsBinary] = useState(false);
  const [fileLoaded, setFileLoaded] = useState(false);

  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [savedNotice, setSavedNotice] = useState(false);
  const [pendingNav, setPendingNav] = useState<(() => void) | null>(null);
  const [snippets, setSnippets] = useState<SnippetSuggestion[]>([]);
  const [insertingLabel, setInsertingLabel] = useState<string | null>(null);

  const dirty = fileLoaded && !isBinary && content !== originalContent;

  useEffect(() => {
    if (!id) return;
    (async () => {
      try {
        const detail = await api.getConfigurationDetail(id);
        if (!detail) return;
        setConfiguration(detail.configuration);
        if (detail.configuration.kind === "directory") {
          setFiles(await api.listConfigurationFiles(id));
        } else {
          setSnippets(await api.listSnippetSuggestions(id));
        }
      } catch (e) {
        setError(errorMessage(e));
      }
    })();
  }, [id]);

  const loadFile = useCallback(
    async (relativePath?: string) => {
      if (!id) return;
      setError(null);
      setFileLoaded(false);
      try {
        const result = await api.readConfigurationFile(id, relativePath);
        setContent(result.content);
        setOriginalContent(result.content);
        setIsBinary(result.is_binary);
        setSelectedFile(relativePath ?? null);
        setFileLoaded(true);
      } catch (e) {
        setError(errorMessage(e));
      }
    },
    [id]
  );

  useEffect(() => {
    if (!configuration) return;
    if (configuration.kind === "file") {
      loadFile();
    }
  }, [configuration, loadFile]);

  useEffect(() => {
    function handleKeyDown(e: KeyboardEvent) {
      if ((e.metaKey || e.ctrlKey) && e.key === "s") {
        e.preventDefault();
        handleSave();
      }
    }
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [content, dirty, id, selectedFile]);

  async function handleSave() {
    if (!id || !dirty || saving) return;
    setSaving(true);
    setError(null);
    try {
      await api.writeConfigurationFile(id, content, selectedFile ?? undefined);
      setOriginalContent(content);
      setSavedNotice(true);
      setTimeout(() => setSavedNotice(false), 2000);
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setSaving(false);
    }
  }

  async function insertSnippet(suggestion: SnippetSuggestion) {
    if (!id || insertingLabel) return;
    setInsertingLabel(suggestion.label);
    setError(null);
    try {
      // The backend applies the suggestion itself (plain append for
      // line-based files, a real JSON parse-and-merge for configs like
      // VS Code's settings.json) so it can never produce invalid syntax.
      // Nothing is written to disk here — just the in-memory buffer.
      const merged = await api.previewSnippetInsertion(id, suggestion.label, content);
      setContent(merged);
      setSnippets((prev) => prev.filter((s) => s.label !== suggestion.label));
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setInsertingLabel(null);
    }
  }

  function guardedNavigate(action: () => void) {
    if (dirty) {
      setPendingNav(() => action);
    } else {
      action();
    }
  }

  if (!id) return null;

  const backTo = `/configurations/${id}`;

  if (!configuration) {
    return (
      <div className="page-content">
        <PageHeader title="Edit" backTo={backTo} backLabel="Back" />
        {error ? <div className="banner banner-error">{error}</div> : <div className="empty-state">Loading…</div>}
      </div>
    );
  }

  return (
    <div className="page-content editor-page">
      <PageHeader
        title={selectedFile ?? configuration.name}
        subtitle={configuration.kind === "directory" ? configuration.name : configuration.path}
        backTo={backTo}
        backLabel={configuration.name}
        onBack={() => guardedNavigate(() => navigate(backTo))}
      />

      {error && <div className="banner banner-error">{error}</div>}

      {configuration.kind === "directory" && !selectedFile ? (
        <div className="editor-file-list">
          {files === null ? (
            <div className="empty-state">Loading files…</div>
          ) : files.length === 0 ? (
            <div className="empty-state">No files in this directory.</div>
          ) : (
            files.map((file) => (
              <button key={file} className="history-row" onClick={() => loadFile(file)}>
                <div className="history-row-main">
                  <div className="history-row-reason">{file}</div>
                </div>
                <div className="config-row-chevron">›</div>
              </button>
            ))
          )}
        </div>
      ) : (
        <>
          {configuration.kind === "directory" && (
            <button
              className="link-button editor-change-file"
              onClick={() => guardedNavigate(() => setSelectedFile(null))}
            >
              ‹ Choose a different file
            </button>
          )}

          {!fileLoaded ? (
            <div className="empty-state">Loading…</div>
          ) : isBinary ? (
            <div className="empty-state">
              This file appears to be binary and can't be edited here. Use "Open" on the configuration
              page instead.
            </div>
          ) : (
            <>
              <textarea
                className="editor-textarea"
                value={content}
                onChange={(e) => setContent(e.target.value)}
                spellCheck={false}
              />
              <div className="editor-toolbar">
                <span className="editor-status">
                  {saving ? "Saving…" : savedNotice ? "Saved" : dirty ? "Unsaved changes" : "No changes"}
                </span>
                <button className="btn btn-primary" disabled={!dirty || saving} onClick={handleSave}>
                  Save
                </button>
              </div>

              {snippets.length > 0 && (
                <section className="snippet-suggestions">
                  <h2 className="section-heading snippet-suggestions-heading">
                    <Sparkles size={15} strokeWidth={1.75} />
                    Suggestions
                  </h2>
                  <p className="section-hint">
                    Merged into the current buffer — review the change, then Save. JSON files are
                    reformatted when a suggestion is applied; nothing else is touched until you save.
                  </p>
                  <div className="snippet-list">
                    {snippets.map((suggestion) => (
                      <div className="snippet-row" key={suggestion.label}>
                        <div className="snippet-row-main">
                          <div className="snippet-row-label">{suggestion.label}</div>
                          <p className="snippet-row-description">{suggestion.description}</p>
                          <pre className="snippet-row-preview">{suggestion.snippet}</pre>
                        </div>
                        <button
                          className="btn btn-secondary"
                          disabled={insertingLabel !== null}
                          onClick={() => insertSnippet(suggestion)}
                        >
                          {insertingLabel === suggestion.label ? "Inserting…" : "Insert"}
                        </button>
                      </div>
                    ))}
                  </div>
                </section>
              )}
            </>
          )}
        </>
      )}

      {pendingNav && (
        <ConfirmDialog
          title="Discard unsaved changes?"
          message="You have unsaved edits that will be lost."
          confirmLabel="Discard"
          danger
          onConfirm={() => {
            const action = pendingNav;
            setPendingNav(null);
            action();
          }}
          onCancel={() => setPendingNav(null)}
        />
      )}
    </div>
  );
}
