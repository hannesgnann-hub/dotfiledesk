# DotfileDesk Architecture

This document describes how DotfileDesk is put together: the shared Rust core, the three Tauri
shells built on top of it, and the data model connecting them. For platform-specific notes (window
config, Tauri plugins, packaging), see the `docs/ARCHITECTURE.md` inside `mac_src/`, `linux_src/`,
and `windows_src/`.

## Layers

```
┌─────────────────────────────────────────────────────────────────┐
│  React + TypeScript UI          (mac_src|linux_src|windows_src)  │
│  pages/ · components/ · services/api.ts                          │
└───────────────────────────────┬─────────────────────────────────┘
                                 │ @tauri-apps/api invoke()
┌───────────────────────────────▼─────────────────────────────────┐
│  Tauri command layer             src-tauri/src/main.rs           │
│  thin #[tauri::command] wrappers around one Mutex<Core>          │
└───────────────────────────────┬─────────────────────────────────┘
                                 │ plain Rust calls
┌───────────────────────────────▼─────────────────────────────────┐
│  dotfiledesk-core                crates/dotfiledesk-core         │
│  discovery · tracking · history · backup · security · models     │
│  No Tauri, no GUI toolkit — testable and driveable on its own    │
└───────────────────────────────┬─────────────────────────────────┘
                                 │
              ┌──────────────────┴──────────────────┐
              ▼                                      ▼
     SQLite (dotfiledesk.sqlite)         Internal Git repo (repository/)
     configuration + snapshot metadata    actual snapshot contents
```

The command layer in each `src-tauri/main.rs` is intentionally thin: every command locks the
shared `Core`, calls one method, and serializes the result (or a `CoreError`, which serializes to
its `Display` string so the frontend can show it directly). All three platforms use the identical
command signatures — only window chrome, plugin config, and packaging differ between them.

## `dotfiledesk-core` modules

- **`discovery`** — `Registry::load_builtin()` embeds `/definitions/*.json` into the binary at
  compile time via `include_str!`. `scan()` expands `~` for the current platform, checks
  `platforms` in each definition against `Platform::current()`, and reports back only paths that
  actually exist. Purely read-only.
- **`tracking`** — `Store` wraps a `rusqlite::Connection`. Two tables: `configurations` (what's
  tracked) and `snapshots` (metadata only — id, timestamp, git commit, reason). File contents never
  touch SQLite.
- **`history`** — `HistoryRepo` wraps a non-bare `git2::Repository`. Each tracked configuration
  gets its own `{configuration_id}/` subtree inside the repo's working directory, so two
  configurations can never collide even if their real paths are unrelated. See
  [Snapshot storage](#snapshot-storage) below.
- **`backup`** — `restore()` implements the mandatory restore workflow: snapshot current state →
  write the target commit's files to the real path → verify they exist → snapshot the restored
  state as a new history entry.
- **`security`** — filename-pattern classification (`is_private_key`, `classify_path`) and the
  directory-snapshot ignore list. See [Security model](#security-model).
- **`models`** — shared `Category`, `Sensitivity`, `ConfigKind`, `Status`, `Configuration`,
  `Snapshot`.

The `Core` struct in `lib.rs` is the single API surface the Tauri layer talks to — every operation
the UI can trigger is one `Core` method away from a unit test, which is how the 15 tests in
`crates/dotfiledesk-core/src/lib.rs` cover the full add → snapshot → modify → diff → restore →
missing-file lifecycle without touching Tauri at all.

## Snapshot storage

DotfileDesk does not use a literal `git diff`/`git log` on the real file paths — it copies each
tracked configuration's current contents into its own subtree inside an internal repository, then
commits. This has two consequences that shape the rest of the design:

1. **Isolation.** Two unrelated configurations (say, a custom path and a catalog entry) can never
   collide in the repo, and a directory configuration's internal layout is mirrored exactly under
   its subtree.
2. **Commits are per-configuration, not global.** Calling `snapshot()` on one configuration creates
   one commit touching only that configuration's subtree. `git log` on the repo as a whole is a
   linear interleaving of every configuration's history, which is why `Snapshot` rows in SQLite —
   not `git log` — are the source of truth for "this configuration's history." Diffing a snapshot
   against "the one before it" means finding the previous `Snapshot` row for that configuration,
   not walking git parents.

Diffing itself (`history::diff_commits`, `diff_against_working`) reads both sides into
`BTreeMap<String, Vec<u8>>` (either from a git tree or straight off disk) and hands matching files
to the `similar` crate for a line-level diff. This is what lets "diff a snapshot against the
previous one" and "diff the latest snapshot against what's on disk right now" (used by "View
Changes") share one code path.

## Restore workflow

```
restore(id, target_commit)
  1. repo.snapshot(config, "Pre-restore backup")   // only commits if the working copy differs
  2. repo.restore_files(config, target_commit)     // writes every file the snapshot has
  3. verify config.path exists on disk
  4. repo.snapshot(config, "Restored to <short-oid>") // records the restored state as new history
```

Restoring never deletes files that aren't part of the snapshot being restored — a directory
restore only creates/overwrites the files recorded in that snapshot, leaving anything else (build
artifacts, ignored files) untouched.

## Security model

Two independent gates run before anything is tracked:

- **`security::is_private_key`** — a hard filename-pattern block (`id_rsa`, `id_ed25519`, `*.pem`,
  `*.key`, …). `Core::add_discovered` and `Core::add_custom` both check this *before* the
  sensitivity check and return `CoreError::PrivateKeyBlocked` with no way to override it from the
  UI.
- **`security::classify_path` / catalog `sensitivity` field** — anything other than `Normal`
  (`PotentialSecret`, `HighlySensitive`) requires the caller to pass `confirmed: true`, or
  `Core` returns `CoreError::ConfirmationRequired`. The frontend surfaces this as the "this file
  may contain secrets" dialog before ever calling back with `confirmed: true`.

Both checks live in the core, not the UI — a future CLI or scripting surface gets the same
guarantees for free.

## Editor writes

The integrated editor (`read_configuration_file` / `write_configuration_file`) writes straight to
the configuration's real path. It never snapshots automatically: saving a file just changes its
live `Status` from `Synced` to `Modified`, same as if the user had edited it in any other program.
Consistent with the "no automatic snapshotting" rule for v0.1, the user still has to click
**Create Snapshot** to record it in history.

## Data flow example: editing `~/.zshrc`

1. `EditorPage` calls `read_configuration_file` → `Core::read_configuration_file` resolves the
   real path from the `Configuration` row and reads it.
2. User edits and saves → `write_configuration_file` writes straight to `~/.zshrc`.
3. Back on the dashboard, `list_configurations` recomputes `Status` by diffing the live file
   against the latest snapshot's tree (`history::compute_status`) — no caching, always live.
4. User clicks **Create Snapshot** → `Core::snapshot` copies the file into the repo, commits if
   the content actually changed, and records a `Snapshot` row.
5. **View Changes** / History diff both go through the same `diff_file_maps` path described above.

## Extending the catalog

`definitions/*.json` is loaded once, at compile time, via `include_str!`. Adding a new entry is a
one-file PR (see the root [README](../README.md#extending-the-catalog)) — no runtime plugin system
is needed for v0.1, since the whole catalog is small enough to ship in the binary.
