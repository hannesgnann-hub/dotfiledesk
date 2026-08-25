//! Content-level suggestions for *what to put inside* an already-tracked
//! file, loaded from `/definitions/snippets.json`. Distinct from
//! [`crate::CatalogSuggestion`], which suggests entire files/tools to start
//! tracking — this suggests content to add to one you already have.
//!
//! Two insertion strategies, picked per suggestion via [`SnippetFormat`]:
//! - `text` — blindly appended at the end of the file. Only used for
//!   line-oriented formats where that can't corrupt anything: shell rc
//!   files, gitconfig/gitignore, ssh config, npmrc, tmux/vim config,
//!   editorconfig, ripgrep config, a Brewfile, and Emacs Lisp.
//! - `json_object` — the file is parsed as JSON, the suggestion's own JSON
//!   object is shallow-merged in (existing keys are never overwritten), and
//!   the result is re-serialized. Used for flat JSON-object configs like
//!   VS Code's `settings.json` or Windows Terminal's `settings.json`, where
//!   blindly appending text would break the file's syntax outright.
//!
//! Structured formats this module can't yet insert into safely (TOML, Lua
//! tables, JSON arrays like VS Code's `keybindings.json`) simply have no
//! catalog entries.

use serde::{Deserialize, Serialize};

const SNIPPETS_JSON: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../definitions/snippets.json"));

/// How a [`SnippetSuggestion`]'s `snippet` text should be inserted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SnippetFormat {
    /// Append `snippet` verbatim at the end of the file (adding a newline
    /// first if the file doesn't already end with one).
    #[default]
    Text,
    /// Parse `snippet` as a JSON object and shallow-merge its keys into the
    /// file's own parsed JSON object, without overwriting existing keys.
    JsonObject,
}

/// One suggested block of content for a specific kind of file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnippetSuggestion {
    pub label: String,
    /// Plain-language explanation of what the snippet does and why you'd
    /// want it, shown next to the preview so the suggestion is legible
    /// without having to parse the shell/INI/JSON/Lisp syntax yourself.
    pub description: String,
    pub snippet: String,
    #[serde(default)]
    pub format: SnippetFormat,
}

#[derive(Debug, Deserialize)]
struct CatalogEntry {
    definition_id: String,
    suggestions: Vec<SnippetSuggestion>,
}

/// The catalog of known snippet suggestions, loaded from
/// `/definitions/snippets.json`.
pub struct SnippetCatalog {
    entries: Vec<CatalogEntry>,
}

impl SnippetCatalog {
    /// Loads the catalog embedded in the binary at compile time.
    pub fn load_builtin() -> Self {
        let entries: Vec<CatalogEntry> =
            serde_json::from_str(SNIPPETS_JSON).expect("built-in snippets must be valid JSON");
        SnippetCatalog { entries }
    }

    /// Suggestions for `definition_id`, excluding any already effectively
    /// present in `current_content` — a `text` snippet whose exact text is
    /// already there, or a `json_object` snippet whose keys already exist in
    /// the file (regardless of value, since applying it wouldn't change
    /// anything: existing keys are never overwritten).
    pub fn suggestions_for(&self, definition_id: &str, current_content: &str) -> Vec<SnippetSuggestion> {
        self.entries
            .iter()
            .find(|entry| entry.definition_id == definition_id)
            .map(|entry| {
                entry
                    .suggestions
                    .iter()
                    .filter(|s| !already_present(s, current_content))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Looks up one suggestion by its (definition, label) pair — labels are
    /// unique within a definition's suggestion list, which is all the
    /// catalog guarantees.
    pub fn find(&self, definition_id: &str, label: &str) -> Option<&SnippetSuggestion> {
        self.entries
            .iter()
            .find(|entry| entry.definition_id == definition_id)?
            .suggestions
            .iter()
            .find(|s| s.label == label)
    }
}

fn already_present(suggestion: &SnippetSuggestion, current_content: &str) -> bool {
    match suggestion.format {
        SnippetFormat::Text => current_content.contains(suggestion.snippet.trim()),
        SnippetFormat::JsonObject => {
            let Ok(addition) = serde_json::from_str::<serde_json::Value>(&suggestion.snippet) else {
                return false;
            };
            let Ok(current) = serde_json::from_str::<serde_json::Value>(current_content) else {
                return false;
            };
            match (addition.as_object(), current.as_object()) {
                (Some(addition), Some(current)) => addition.keys().all(|key| current.contains_key(key)),
                _ => false,
            }
        }
    }
}

/// Applies `suggestion` to `current_content`, returning the new full file
/// content. Never writes anything — the caller (the editor's "Insert"
/// button) puts this straight into its in-memory buffer, so the user still
/// has to hit Save, same as any other edit.
pub fn apply(suggestion: &SnippetSuggestion, current_content: &str) -> Result<String, String> {
    match suggestion.format {
        SnippetFormat::Text => {
            let needs_newline = !current_content.is_empty() && !current_content.ends_with('\n');
            Ok(format!(
                "{current_content}{}{}",
                if needs_newline { "\n" } else { "" },
                suggestion.snippet
            ))
        }
        SnippetFormat::JsonObject => {
            let mut base: serde_json::Value = if current_content.trim().is_empty() {
                serde_json::Value::Object(serde_json::Map::new())
            } else {
                serde_json::from_str(current_content)
                    .map_err(|e| format!("this file isn't valid JSON, so a suggestion can't be merged in: {e}"))?
            };
            let addition: serde_json::Value = serde_json::from_str(&suggestion.snippet)
                .expect("built-in json_object snippets must themselves be valid JSON");

            let (Some(base_obj), Some(addition_obj)) = (base.as_object_mut(), addition.as_object()) else {
                return Err("this suggestion or file isn't a JSON object".to_string());
            };
            for (key, value) in addition_obj {
                base_obj.entry(key.clone()).or_insert_with(|| value.clone());
            }

            serde_json::to_string_pretty(&base)
                .map(|s| s + "\n")
                .map_err(|e| format!("failed to write the merged JSON back out: {e}"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_builtin_snippets_without_panicking() {
        let catalog = SnippetCatalog::load_builtin();
        assert!(!catalog.entries.is_empty());
    }

    #[test]
    fn suggests_and_then_filters_out_once_present() {
        let catalog = SnippetCatalog::load_builtin();
        let before = catalog.suggestions_for("zsh", "export FOO=bar\n");
        assert!(!before.is_empty());

        let first = &before[0];
        let after_content = format!("export FOO=bar\n{}", first.snippet);
        let after = catalog.suggestions_for("zsh", &after_content);
        assert!(after.iter().all(|s| s.label != first.label));
    }

    #[test]
    fn unknown_definition_has_no_suggestions() {
        let catalog = SnippetCatalog::load_builtin();
        assert!(catalog.suggestions_for("not_a_real_definition", "").is_empty());
    }

    #[test]
    fn text_snippets_are_appended_with_a_leading_newline_if_needed() {
        let suggestion = SnippetSuggestion {
            label: "Test".into(),
            description: "".into(),
            snippet: "export FOO=bar\n".into(),
            format: SnippetFormat::Text,
        };
        assert_eq!(apply(&suggestion, "").unwrap(), "export FOO=bar\n");
        assert_eq!(
            apply(&suggestion, "export BAZ=qux\n").unwrap(),
            "export BAZ=qux\nexport FOO=bar\n"
        );
        assert_eq!(
            apply(&suggestion, "export BAZ=qux").unwrap(),
            "export BAZ=qux\nexport FOO=bar\n"
        );
    }

    #[test]
    fn json_object_snippets_merge_without_overwriting_existing_keys() {
        let catalog = SnippetCatalog::load_builtin();
        let suggestion = catalog.find("vscode_settings", "Format on save").unwrap();

        let merged = apply(suggestion, "{\n  \"editor.tabSize\": 4\n}").unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&merged).unwrap();
        assert_eq!(parsed["editor.tabSize"], 4);
        assert_eq!(parsed["editor.formatOnSave"], true);

        // Applying to an empty file starts from `{}` instead of failing.
        let from_empty = apply(suggestion, "").unwrap();
        let parsed_empty: serde_json::Value = serde_json::from_str(&from_empty).unwrap();
        assert_eq!(parsed_empty["editor.formatOnSave"], true);

        // An existing, different value for the same key is preserved.
        let existing = "{\"editor.formatOnSave\": false}";
        let merged_existing = apply(suggestion, existing).unwrap();
        let parsed_existing: serde_json::Value = serde_json::from_str(&merged_existing).unwrap();
        assert_eq!(parsed_existing["editor.formatOnSave"], false);
    }

    #[test]
    fn json_object_suggestion_disappears_once_its_keys_exist() {
        let catalog = SnippetCatalog::load_builtin();
        let before = catalog.suggestions_for("vscode_settings", "{}");
        assert!(before.iter().any(|s| s.label == "Format on save"));

        let after = catalog.suggestions_for("vscode_settings", "{\"editor.formatOnSave\": false}");
        assert!(after.iter().all(|s| s.label != "Format on save"));
    }

    #[test]
    fn invalid_existing_json_produces_a_clear_error_instead_of_corrupting_the_file() {
        let catalog = SnippetCatalog::load_builtin();
        let suggestion = catalog.find("vscode_settings", "Format on save").unwrap();
        let result = apply(suggestion, "{ not valid json");
        assert!(result.is_err());
    }
}
