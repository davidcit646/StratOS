# TALKING-2.md

## Strat Terminal Requirements Summary (from design/checklist)
- Build `stratterm` as a graphical Strat Terminal app with a Wayland window and default interface shape from `StratOS-Design-Doc-v0.4.md` section 13.
- MVP must include: file explorer + terminal I/O in one app, shell over PTY (`fish` preferred, `bash` fallback), basic ANSI/VT100 handling, breadcrumb navigation, and `..`/double-click folder navigation.
- File listing must be sorted with directories first and files after.
- File browser must stay in sync with shell working directory when user runs `cd`.
- Integrate project in repo (`stratterm/`) and provide build/run path suitable for later rootfs staging.
- Maintain this file as append-only handoff log with dated entries for edits, file creations, build/test runs, and dependency decisions.

## Architecture Decision
- Chosen approach: **Option B (GTK4 + VTE)**.
- Rationale: fastest route to a working MVP with robust terminal emulation (ANSI/VT100) while preserving room to iterate toward fuller StratOS-specific UI behaviors later.
- Tradeoff accepted: stronger native toolkit dependency footprint than a pure custom renderer, but much faster MVP delivery and lower terminal-emulation risk.

## Append-Only Work Log

### 2026-04-14 13:18 UCT
- Action: Read required spec sections.
- Paths: `StratOS-Design-Doc-v0.4.md` (section `## 13. STRAT TERMINAL`), `StratOS-Codex-Checklist-v2.md` (section `## PHASE 9 — STRAT TERMINAL`).
- Rationale: lock mandatory scope before implementation.

### 2026-04-14 13:19 UCT
- Action: Scanned integration points and runtime paths.
- Paths: `Makefile`, `stratvm/Makefile`, `sysroot/system-init.c`, `scripts/phase7/build-initramfs.sh`, `scripts/phase7/build-phase7-artifacts.sh`, `scripts/phase7/prepare-minimal-rootfs.sh`.
- Rationale: ensure `stratterm` can be built now and staged into rootfs later without breaking Phase 7 flow.

### 2026-04-14 13:20 UCT
- Action: Checked local tool availability.
- Paths: environment command probes (`cargo`, `rustc`, `pkg-config`, `flatpak-spawn`).
- Rationale: assess whether compile/test can run in this sandbox.
- Result: required local build tooling is missing in current sandbox environment (no `cargo`, `rustc`, `pkg-config`, `flatpak-spawn`). Implementation can proceed; local compile validation may require host/toolchain install.

### 2026-04-14 13:22 UCT
- Action: Created this handoff file with requirements summary, architecture decision, and ongoing append-only log.
- Paths: `TALKING-2.md`.
- Rationale: mandatory project tracking artifact for this implementation.

### 2026-04-14 13:23 UCT
- Action: Appended major milestone note.
- Paths: `TALKING.md`.
- Rationale: requirement asks for short Cursor/Codex updates at major milestones (architecture decision + environment blocker).

### 2026-04-14 13:29 UCT
- Action: Scaffolded initial `stratterm` MVP crate and implementation.
- Paths:
  - `stratterm/Cargo.toml`
  - `stratterm/Makefile`
  - `stratterm/src/main.rs`
- Rationale:
  - Introduced Option B architecture implementation (GTK4 + VTE).
  - Added first runnable UI shell: title/header, working directory line, breadcrumb bar, sorted file list (`DIR` first + `FILE`), quick action buttons, and terminal pane.
  - Implemented PTY shell spawn (`fish` preferred, `bash` fallback), VTE scrollback (10,000 lines), double-click directory navigation, `..` row, breadcrumb clicks, and browser/shell CWD sync via terminal URI updates + `/proc/<pid>/cwd` polling.

### 2026-04-14 13:30 UCT
- Action: Added repo-level build integration for Strat Terminal.
- Paths: `Makefile`.
- Rationale: satisfy integration requirement and provide a stable entry point (`make stratterm`) that delegates to `make -C stratterm build`.

### 2026-04-14 13:31 UCT
- Action: Added optional Phase 7 staging path for `stratterm` binary.
- Paths: `scripts/phase7/prepare-minimal-rootfs.sh`.
- Rationale: make MVP stageable into StratOS rootfs later by copying `stratterm` into `/bin/stratterm` when present and including it in runtime dependency closure processing.

### 2026-04-14 13:32 UCT
- Action: Added project README for build/run/staging workflow.
- Paths: `stratterm/README.md`.
- Rationale: provide deterministic handoff instructions and explicitly document MVP behavior + rootfs staging path.

### 2026-04-14 13:33 UCT
- Action: Ran build command for repo-level integration check.
- Command: `make stratterm`
- Result: failed (`make: command not found`) in current sandbox.
- Rationale: validate top-level target and confirm runtime/toolchain constraints early.

### 2026-04-14 13:33 UCT
- Action: Ran direct crate build check.
- Command: `cd stratterm && cargo build`
- Result: failed (`cargo: command not found`) in current sandbox.
- Rationale: verify whether Rust toolchain is available for local MVP compile validation.

### 2026-04-14 13:35 UCT
- Action: Appended MVP milestone note.
- Paths: `TALKING.md`.
- Rationale: requirement asks for short Cursor/Codex updates on major milestones; MVP implementation wiring completed.

### 2026-04-14 13:37 UCT
- Action: Updated GTK dependency version for crate compatibility.
- Paths: `stratterm/Cargo.toml`.
- Dependency change: `gtk4` `0.10` -> `0.11` (as `gtk` package alias).
- Rationale: align with `vte4 0.10` dependency graph (`gtk4 ^0.11`) to avoid mixed GTK type versions at compile time.

### 2026-04-14 13:38 UCT
- Action: Updated run target to prefer Wayland backend.
- Paths: `stratterm/Makefile`.
- Rationale: reinforce MVP requirement for Wayland window path (`GDK_BACKEND=wayland` on `make -C stratterm run`).

### 2026-04-14 13:41 UCT
- Action: Applied navigation/CWD sync robustness polish.
- Paths: `stratterm/src/main.rs`.
- Rationale:
  - Always show `DIR  .. (go up)` row (root now maps to `/` rather than hiding the row).
  - Normalize paths with `canonicalize` when possible to reduce CWD alias drift and keep shell/browser sync stable.

### 2026-04-14 13:49 UCT
- Action: Expanded `stratterm` beyond baseline MVP terminal behavior.
- Paths: `stratterm/src/main.rs`.
- Rationale:
  - Added Tree/Flat toggle (`Tree: On/Off`) with recursive tree listing mode.
  - Added single-click preview panel with context-aware behavior:
    - Folder preview summary
    - File first-lines preview
    - Script preview + run behavior hint
    - Config plain-language summary + preview
  - Added file-type-aware double-click actions:
    - Folder/`..` navigate
    - Script run in terminal
    - Config open in editor (`nano`/`vi` fallback)
    - Generic file open via `xdg-open` (`less` fallback)
  - Added quick action wiring with in-app Help surface and docs/user-guide open commands.

### 2026-04-14 13:50 UCT
- Action: Updated docs for new behavior.
- Paths: `stratterm/README.md`.
- Rationale: keep handoff/build docs aligned with current implemented features (tree mode, previews, and file action semantics).

### 2026-04-14 13:50 UCT
- Action: Appended major milestone note.
- Paths: `TALKING.md`.
- Rationale: requirement asks for short Cursor/Codex updates on major milestones; this marks the first feature-expansion pass beyond baseline terminal behavior.

### 2026-04-14 13:51 UCT
- Action: Ran build validation after feature expansion.
- Command: `cd stratterm && cargo build`
- Result: failed (`cargo: command not found`) in current sandbox.
- Rationale: attempt compile-time sanity check for newly added tree/preview/file-action functionality.

### 2026-04-14 13:53 UCT
- Action: Patched tree view ordering semantics.
- Paths: `stratterm/src/main.rs`.
- Rationale: ensure Tree mode appears as inline expansion (children rendered directly beneath each directory) rather than batching all nested nodes after top-level files.

### 2026-04-14 14:10 UTC
- Action: Audit fix for shell portability + doc alignment ahead of merge.
- Paths:
  - `stratterm/src/main.rs`
  - `stratterm/README.md`
- Rationale:
  - Removed `disown` from `xdg-open` launch commands (not portable across shells; fish is preferred).
  - Updated README to avoid Phase 7 staging instructions since current integration is targeting Phase 8.x+.

### 2026-04-14 14:02 UCT
- Action: Added prompt-line ghost completion + smart `cd` behavior.
- Paths: `stratterm/src/main.rs`.
- Rationale:
  - Added dedicated bottom prompt line (`>`) with ghost suggestion label.
  - Implemented ghost update loop with accept/dismiss controls:
    - `Tab` / `Right` accept current ghost suggestion
    - `Esc` dismiss current ghost suggestion
  - Implemented `cd` ghost ranking sources (in order): frecency, cwd dirs, home-scan dirs, system PATH dirs.
  - Implemented `cd -s` smart shorthand expansion using directory-segment initials.
  - Added command ghosting from local prompt history.
  - Added lightweight persistent frecency storage at `~/.config/strat/frecency.tsv` (fallback `/tmp/stratterm_frecency.tsv`) and update-on-directory-change tracking.

### 2026-04-14 14:03 UCT
- Action: Updated README for prompt-line/ghost/smart-cd behavior.
- Paths: `stratterm/README.md`.
- Rationale: keep handoff docs aligned with implemented non-basic Strat Terminal behaviors.

### 2026-04-14 14:03 UCT
- Action: Appended major milestone note.
- Paths: `TALKING.md`.
- Rationale: requirement asks for short Cursor/Codex milestone updates; this marks first ghost-completion + smart-cd pass.

### 2026-04-14 14:04 UCT
- Action: Ran build validation after ghost/smart-cd implementation.
- Command: `cd stratterm && cargo build`
- Result: failed (`cargo: command not found`) in current sandbox.
- Rationale: attempt compile-time sanity check for prompt-line ghost completion and `cd -s` changes.

### 2026-04-14 14:05 UCT
- Action: Patched ghost-dismiss state behavior.
- Paths: `stratterm/src/main.rs`.
- Rationale: ensure dismissed ghost suggestions are truly cleared internally (prevents stale suggestion acceptance via `Tab` after `Esc` dismiss).

### 2026-04-14 14:16 UCT
- Action: Locked prompt-line semantics to non-destructive ghost behavior.
- Paths: `stratterm/src/main.rs`.
- Rationale:
  - Ghost acceptance (`Tab`/Right) now only updates prompt text; it never executes or mutates shell state.
  - `Esc` dismissal keeps shell unchanged and clears active ghost suggestion state.

### 2026-04-14 14:17 UCT
- Action: Hardened file action safety on double-click.
- Paths: `stratterm/src/main.rs`.
- Rationale:
  - Removed executable-bit auto-run shortcut from script detection.
  - Only clearly script-like files (shebang / known script extensions) auto-run.
  - Arbitrary executables now refuse auto-run and print a manual-run warning instead.

### 2026-04-14 14:18 UCT
- Action: Chose and documented CWD sync source-of-truth policy.
- Paths: `stratterm/src/main.rs`.
- Rationale:
  - Removed split-path CWD update path from terminal URI callbacks.
  - Standardized on `/proc/<shell-pid>/cwd` polling as the single CWD authority.
  - Added inline code comment documenting this design decision.

### 2026-04-14 14:19 UCT
- Action: Updated docs to match behavior locks.
- Paths: `stratterm/README.md`.
- Rationale: documented non-destructive ghost semantics, executable safety behavior, and polling-based CWD authority.

### 2026-04-14 14:20 UCT
- Action: Ran post-lock build validation attempt.
- Command: `cd stratterm && cargo build`
- Result: failed (`cargo: command not found`) in current sandbox.
- Rationale: compile sanity check after prompt/file-safety/CWD-source policy updates.

### 2026-04-14 14:21 UCT
- Action: Appended major milestone note.
- Paths: `TALKING.md`.
- Rationale: requirement asks for short Cursor/Codex updates for major behavior/architecture locks.

### 2026-04-14 14:29 UCT
- Action: Switched frecency persistence from TSV to SQLite.
- Paths:
  - `stratterm/Cargo.toml`
  - `stratterm/src/main.rs`
- Dependency change:
  - Added `rusqlite` with `bundled` feature.
- Rationale:
  - Align frecency storage with Phase 9 direction (`frecency.db`) and make ranking data durable with schema-based storage.
  - Replaced ad-hoc text parsing/writes with SQLite table initialization, row loading, and upsert-on-visit.
  - Kept existing in-memory ranking behavior and updated persistence path to `~/.config/strat/frecency.db` (fallback `/tmp/stratterm_frecency.db`).

### 2026-04-14 14:30 UCT
- Action: Updated README frecency storage reference.
- Paths: `stratterm/README.md`.
- Rationale: documentation now matches SQLite-backed implementation.

### 2026-04-14 14:30 UCT
- Action: Ran post-SQLite build validation attempt.
- Command: `cd stratterm && cargo build`
- Result: failed (`cargo: command not found`) in current sandbox.
- Rationale: compile sanity check after frecency backend migration.

### 2026-04-14 14:44 UTC
- Action: Implemented SQLite path+metadata index for impeccable search.
- Paths:
  - `stratterm/src/main.rs`
  - `stratterm/README.md`
- Rationale:
  - Replaced bounded HOME pre-scan with a persistent index to keep startup fast and make suggestions scale.
  - Added `index.db` schema (`paths` table) and a UI-safe background indexer (batched ticks; capped queue).
  - Wired `cd` and `cd -s` suggestions to query the index for directory candidates.
  - Documented index DB location and behavior in README.

### 2026-04-14 14:31 UCT
- Action: Appended major milestone note.
- Paths: `TALKING.md`.
- Rationale: requirement asks for milestone notes; SQLite backend migration materially changes terminal persistence architecture.

### 2026-04-14 14:33 UCT
- Action: Applied compile-safety import fix after CWD sync refactor.
- Paths: `stratterm/src/main.rs`.
- Rationale: re-added `gtk::gio` import required by VTE `spawn_async` cancellable type usage.

### 2026-04-14 14:34 UCT
- Action: Synced UI mode indicators with app state on refresh.
- Paths: `stratterm/src/main.rs`.
- Rationale: keep `Mode: Guided/Advanced` label and prompt placeholder consistent after redraws/navigation updates.

### 2026-04-14 14:40 UCT
- Action: Added tree inline expansion controls with persistent per-folder state.
- Paths: `stratterm/src/main.rs`.
- Rationale:
  - Added `Expand/Collapse` UI action for tree mode.
  - Added `expanded_dirs` state tracking and tree-row expansion markers (`[+]` / `[-]`) in tree mode.
  - Updated tree builder to recurse only into expanded directories instead of always expanding all descendants.
  - Preserved double-click folder navigation semantics while giving explicit inline tree expansion control.

### 2026-04-14 14:41 UCT
- Action: Updated README with tree expansion behavior note.
- Paths: `stratterm/README.md`.
- Rationale: document new inline expansion behavior in tree mode.

### 2026-04-14 14:41 UCT
- Action: Ran post-tree/SQLite build validation attempt.
- Command: `cd stratterm && cargo build`
- Result: failed (`cargo: command not found`) in current sandbox.
- Rationale: compile sanity check after persistence and tree expansion updates.

### 2026-04-14 14:42 UCT
- Action: Appended major milestone note.
- Paths: `TALKING.md`.
- Rationale: summarize maturity pass (SQLite persistence + tree expansion controls + preserved safety semantics) for Cursor audit.

### 2026-04-14 14:49 UCT
- Action: Added explicit script-run confirmation flow in file activation path.
- Paths: `stratterm/src/main.rs`.
- Rationale:
  - Script files no longer run on first double-click.
  - First activation arms confirmation; second activation of the same script executes.
  - Directory changes and non-script actions clear pending script confirmation state.
  - Keeps shell state safer while still supporting quick script execution.

### 2026-04-14 14:50 UCT
- Action: Updated script behavior docs and preview text.
- Paths:
  - `stratterm/src/main.rs`
  - `stratterm/README.md`
- Rationale: make confirmation semantics explicit in both UI messaging and repository docs.

### 2026-04-14 14:51 UCT
- Action: Ran post-script-confirmation build validation attempt.
- Command: `cd stratterm && cargo build`
- Result: failed (`cargo: command not found`) in current sandbox.
- Rationale: compile sanity check after safety-flow changes.

### 2026-04-14 14:52 UCT
- Action: Appended major milestone note.
- Paths: `TALKING.md`.
- Rationale: record safety-flow milestone for Cursor/Codex audit continuity.

### 2026-04-14 14:56 UCT
- Action: Tightened ghost acceptance/rendering contract.
- Paths: `stratterm/src/main.rs`.
- Rationale:
  - Ghost accept now requires strict prefix extension of current prompt text.
  - Non-prefix suggestions are suppressed from both rendering and acceptance state.
  - Reinforces non-destructive prompt semantics.

### 2026-04-14 14:56 UCT
- Action: Ran post-ghost-contract build validation attempt.
- Command: `cd stratterm && cargo build`
- Result: failed (`cargo: command not found`) in current sandbox.
- Rationale: compile sanity check after strict ghost prefix enforcement changes.

### 2026-04-14 15:03 UCT
- Action: Applied UI modernization pass (presentation only).
- Paths: `stratterm/src/main.rs`.
- Rationale:
  - Added application-level GTK CSS theme injection with a modern visual system (gradient shell, cards/surfaces, chip/button styling, mono prompt/readouts, section typography).
  - Restyled layout sections with explicit headings (`BREADCRUMB`, `FILES`, `PREVIEW`, `TERMINAL`) and polished containers.
  - Styled key controls (action buttons, mode chip, prompt bar, prompt symbol, ghost text).
  - Styled file list rows by semantic type (`up`, `directory`, `file`) and applied better visual hierarchy.
  - Styled breadcrumb buttons to match new action language.
  - Kept interaction behavior unchanged while improving aesthetics and readability.

### 2026-04-14 15:04 UCT
- Action: Updated README for visual refresh.
- Paths: `stratterm/README.md`.
- Rationale: document modernized visual theme and control styling in feature list.

### 2026-04-14 15:04 UCT
- Action: Ran post-UI-refresh build validation attempt.
- Command: `cd stratterm && cargo build`
- Result: failed (`cargo: command not found`) in current sandbox.
- Rationale: compile sanity check after visual-system changes.

### 2026-04-14 15:05 UCT
- Action: Appended major milestone note.
- Paths: `TALKING.md`.
- Rationale: record UI-focused milestone so Cursor can clearly separate design polish work from indexing efforts.

### 2026-04-14 15:13 UCT
- Action: Rebased UI work onto latest `stratterm` file state that now includes path-indexing integration.
- Paths: `stratterm/src/main.rs`.
- Rationale: user requested UI-only iteration while Cursor handles indexing; this pass preserves indexer behavior and applies visual system changes on top of merged logic.

### 2026-04-14 15:14 UCT
- Action: Added modern visual system and themed layout structure.
- Paths: `stratterm/src/main.rs`.
- Rationale:
  - Added app-level GTK CSS provider (`apply_modern_styles`) with modernized typography, color, spacing, cards/surfaces, and action/control styles.
  - Applied theme classes to window shell, title/subtitle, section headers, scrollers, action buttons, terminal panel, prompt row, ghost text, and mode chip.
  - Added header status chip and wired it into UI model.

### 2026-04-14 15:15 UCT
- Action: Added dynamic status chip updates in render loop.
- Paths: `stratterm/src/main.rs`.
- Rationale:
  - Status now reflects mode (Guided/Advanced), view (Flat/Tree), visible item count, and indexer state (queued/error/idle) each refresh.
  - Gives immediate system feedback without changing command semantics.

### 2026-04-14 15:16 UCT
- Action: Styled file rows and breadcrumb controls with semantic visual classes.
- Paths: `stratterm/src/main.rs`.
- Rationale:
  - Added per-entry row classes (`up`, `directory`, `file`) and monospaced row label class for cleaner hierarchy.
  - Styled breadcrumb buttons to align with the modern action language.

### 2026-04-14 15:17 UCT
- Action: Updated README for latest UI capabilities.
- Paths: `stratterm/README.md`.
- Rationale: documented live status chip and retained modern-theme feature notes.

### 2026-04-14 15:17 UCT
- Action: Ran post-UI-integration build validation attempt.
- Command: `cd stratterm && cargo build`
- Result: failed (`cargo: command not found`) in current sandbox.
- Rationale: compile sanity check after integrated modern UI pass.

### 2026-04-14 15:18 UCT
- Action: Appended major milestone note.
- Paths: `TALKING.md`.
- Rationale: mark completion of UI modernization integration pass for Cursor audit continuity.

### 2026-04-14 15:26 UCT
- Action: Completed post-merge completeness audit and patched index/status integration gaps.
- Paths: `stratterm/src/main.rs`.
- Rationale:
  - Fixed potential index transaction mutability issue by opening DB connection as mutable before `transaction()`.
  - Added `start_status_updater` timer so status chip updates continuously (mode/view/item/index queue/error) without requiring manual UI refresh events.
  - Added `update_status_label` helper and switched `refresh_view` to use it.
  - Enqueued newly navigated directories into index queue on `change_directory` so indexing coverage grows with user navigation.
  - Cleared stale index error state after successful index commits.
  - Removed unused `OptionalExtension` import.

### 2026-04-14 15:27 UCT
- Action: Verified modern UI + indexing coexistence after fixes.
- Paths: `stratterm/src/main.rs`.
- Rationale: ensure UI refresh/status changes do not alter command safety behavior or indexing execution path.

### 2026-04-14 15:37 UCT
- Action: Implemented quiet indexing lifecycle policy (startup + idle + close).
- Paths: `stratterm/src/main.rs`.
- Rationale:
  - Added startup/idle/close indexing policy constants.
  - Added activity tracking timestamps (`startup_ts`, `last_activity_ts`) in app state.
  - `index_tick` now indexes during startup grace window and during idle periods only.
  - Added close hook (`connect_close_request`) with best-effort queued-index flush (`run_quiet_close_index`).
  - Added periodic status updater to keep status chip aligned with current index queue/error state.

### 2026-04-14 15:38 UCT
- Action: Added event-driven indexing on viewed/opened/edited/navigated paths.
- Paths: `stratterm/src/main.rs`.
- Rationale:
  - Added `mark_activity` helper and wired it into prompt input, key events, quick actions, tree controls, file selection, file activation, and directory changes.
  - Added `queue_path_for_index` helper to enqueue file paths and parents when viewed/opened/edited.
  - Added index queueing when changing directories.

### 2026-04-14 15:39 UCT
- Action: Updated index documentation to match requested behavior.
- Paths: `stratterm/README.md`.
- Rationale: documented quiet startup/idle/close indexing policy and event-driven queueing triggers.

### 2026-04-14 15:39 UCT
- Action: Ran post-index-policy build validation attempt.
- Command: `cd stratterm && cargo build`
- Result: failed (`cargo: command not found`) in current sandbox.
- Rationale: compile sanity check after indexing lifecycle and event-queue changes.

### 2026-04-14 15:40 UCT
- Action: Appended major milestone note.
- Paths: `TALKING.md`.
- Rationale: capture indexing lifecycle/event-queue completion milestone for Cursor/Codex audit continuity.

### 2026-04-14 15:49 UCT
- Action: Added high-usage backoff policy for indexing and lowered default indexing aggressiveness.
- Paths: `stratterm/src/main.rs`.
- Rationale:
  - Reduced indexing cadence/batch pressure to lower background resource impact:
    - `INDEX_TICK_MS`: `25` -> `750`
    - `INDEX_BATCH_LIMIT`: `600` -> `80`
    - `INDEX_IDLE_AFTER_SECS`: `6` -> `12`
    - `INDEX_STARTUP_GRACE_SECS`: `15` -> `8`
    - `INDEX_CLOSE_BATCH_LIMIT`: `6000` -> `2000`
  - Added load-sensitive pause logic via `/proc/loadavg` + CPU count (`is_usage_high`) with threshold (`INDEX_HIGH_USAGE_LOAD_PER_CPU=0.85`).
  - `index_tick` now waits when usage is high, and status tracks pause condition.
  - Close-time quiet flush now also respects high usage by reducing close flush budget.

### 2026-04-14 15:50 UCT
- Action: Updated status chip semantics for indexing pause state.
- Paths: `stratterm/src/main.rs`.
- Rationale: status now explicitly reports `Index paused (high usage)` when queue exists but indexing is intentionally deferred.

### 2026-04-14 15:50 UCT
- Action: Updated README with high-usage pause behavior.
- Paths: `stratterm/README.md`.
- Rationale: document requested policy that indexing should wait when system usage is high.

### 2026-04-14 15:51 UCT
- Action: Ran post-backoff-policy build validation attempt.
- Command: `cd stratterm && cargo build`
- Result: failed (`cargo: command not found`) in current sandbox.
- Rationale: compile sanity check after index throttling and usage-sensitive pause changes.

### 2026-04-14 15:51 UCT
- Action: Appended major milestone note.
- Paths: `TALKING.md`.
- Rationale: capture completion of requested high-usage wait policy for indexing.

### 2026-04-14 14:42 UCT
- Action: Adjusted folder-navigation indexing flow to load UI first, then queue silent indexing only for changed paths.
- Paths: `stratterm/src/main.rs`.
- Rationale:
  - Added post-navigation background scheduler (`schedule_post_navigation_index`) that runs after a short delay so folder contents render first.
  - Scheduler inspects the opened directory + immediate children, and only queues paths when metadata differs from index DB (`should_index_path`).
  - Added short index force window (`index_force_until_ts`) so queued post-navigation work can process promptly without waiting for full idle window.
  - Kept high-usage guard intact (`is_usage_high`): if system load is high, post-navigation indexing defers.
  - Removed eager queueing during directory activation; navigation now follows: render folder -> schedule quiet changed-only indexing.

### 2026-04-14 14:43 UCT
- Action: Ran post-change build validation attempt.
- Command: `cd stratterm && cargo build`
- Result: failed (`cargo: command not found`) in current sandbox.
- Rationale: compile sanity check after post-navigation indexing behavior changes.

### 2026-04-14 14:47 UCT
- Action: Added dedicated headless indexer binary for boot/background operation.
- Paths: `stratterm/src/bin/stratterm-indexer.rs`.
- Rationale:
  - Implemented lightweight daemon path (`stratterm-indexer`) so indexing can run without launching the GTK/VTE window.
  - Added continuous daemon mode plus one-shot mode (`--once`) for controlled runs.
  - Kept indexing low-priority: batch processing + load-aware backoff (`/proc/loadavg`) so indexing waits when system usage is high.
  - Implemented change-aware writes by comparing metadata fingerprint before SQLite upsert, so unchanged paths are skipped.
  - Added disable controls: environment variable `STRAT_INDEXER_DISABLE=1` and flag file `/config/strat/disable-indexer`.

### 2026-04-14 14:47 UCT
- Action: Added boot launcher script for background indexer startup.
- Paths: `sysroot/strat-indexer-boot.sh`.
- Rationale:
  - Script starts `/bin/stratterm-indexer --daemon` in background, writes PID/log under `/run`, and avoids duplicate launches.
  - Startup is intentionally default-on, but user can disable later via `/config/strat/disable-indexer`.

### 2026-04-14 14:47 UCT
- Action: Wired PID1 boot path to launch indexer bootstrap script.
- Paths: `sysroot/system-init.c`.
- Rationale: added `run_once_if_present("/bin/strat-indexer-boot.sh")` so background indexing starts during boot before UI interaction.

### 2026-04-14 14:47 UCT
- Action: Wired rootfs assembly to stage indexer daemon artifacts.
- Paths: `scripts/phase7/prepare-minimal-rootfs.sh`.
- Rationale:
  - Added staging support for `stratterm-indexer` binary and `strat-indexer-boot.sh` script into `/bin`.
  - Added indexer binary to runtime dependency queue so required libs are collected into minimal rootfs.

### 2026-04-14 14:47 UCT
- Action: Added Makefile run/install affordances for new daemon binary.
- Paths: `stratterm/Makefile`.
- Rationale:
  - Added `run-indexer` and `run-indexer-once` targets.
  - `install-rootfs` now installs both `stratterm` and `stratterm-indexer`.

### 2026-04-14 14:47 UCT
- Action: Updated docs for boot-time headless indexing behavior.
- Paths: `stratterm/README.md`.
- Rationale: documented that boot launches lightweight `stratterm-indexer` in background, plus disable controls.

### 2026-04-14 14:47 UCT
- Action: Ran script syntax verification checks.
- Commands:
  - `sh -n sysroot/strat-indexer-boot.sh`
  - `sh -n scripts/phase7/prepare-minimal-rootfs.sh`
- Result: both passed (no syntax errors emitted).
- Rationale: quick sanity check for boot/rootfs shell scripts after edits.

### 2026-04-14 14:47 UCT
- Action: Ran headless indexer build validation attempt.
- Command: `cd stratterm && cargo build --bin stratterm-indexer`
- Result: failed (`cargo: command not found`) in current sandbox.
- Rationale: compile sanity check for new daemon binary target.

### 2026-04-14 14:48 UCT
- Action: Scoped headless daemon root scan set to user/system content roots only.
- Paths: `stratterm/src/bin/stratterm-indexer.rs`.
- Rationale:
  - Removed implicit `current_dir` root from daemon indexing roots.
  - Prevents accidental full-root (`/`) crawl when daemon starts under PID1 boot context.
  - Daemon now focuses on `/home`, `/config`, and `/apps` for lower background impact.

### 2026-04-14 14:48 UCT
- Action: Documented daemon root scope in README.
- Paths: `stratterm/README.md`.
- Rationale: clarified that boot/background indexer scans only `/home`, `/config`, and `/apps`.

### 2026-04-14 15:02 UCT
- Action: Added shared index-settings backend module for future System Settings integration.
- Paths: `stratterm/src/lib.rs`, `stratterm/src/index_settings.rs`.
- Rationale:
  - Introduced centralized `IndexSettings` config model with sane defaults.
  - Added config loading from `/config/strat/indexer.conf` (preferred) and `~/.config/strat/indexer.conf` (fallback).
  - Added parsing for backend controls: enable/disable, boot-start toggle, indexing frequency/pacing, include roots/exclude prefixes, high-usage threshold, and UI indexer pacing controls.
  - Added helper gates: `indexer_is_disabled` (flag/env disable) and `path_allowed_for_indexing` (scope policy).

### 2026-04-14 15:02 UCT
- Action: Wired headless daemon to shared settings framework.
- Paths: `stratterm/src/bin/stratterm-indexer.rs`.
- Rationale:
  - Daemon now reads shared settings instead of fixed constants.
  - Added run mode `--boot-daemon` so boot launch can respect `boot_start` without extra shell parsing.
  - Frequency knobs now config-driven (`frequency_ms`, `rescan_secs`, `batch_limit`, `high_usage_load_per_cpu`).
  - Scope knobs now config-driven (`roots`, `exclude_prefixes`) and enforced at queue-time.
  - Global disable path now supports both setting/flag behavior (`enabled=false` and `disable-indexer` semantics).

### 2026-04-14 15:02 UCT
- Action: Wired UI-side indexer behavior to shared settings framework.
- Paths: `stratterm/src/main.rs`.
- Rationale:
  - Added app-state settings snapshot and `indexing_enabled` gate (`enabled`, `ui_enabled`, disable-flag aware).
  - Replaced fixed UI index constants with config-backed values:
    - `ui_tick_ms`, `ui_batch_limit`, `ui_idle_after_secs`, `ui_startup_grace_secs`
    - `ui_post_nav_delay_ms`, `ui_post_nav_scan_limit`, `ui_post_nav_force_secs`
    - `high_usage_load_per_cpu`
  - Enforced indexing scope policy via `path_allowed_for_indexing` for queued paths and root seeding.
  - Updated status chip to display `Index disabled` when indexing is turned off.
  - Post-navigation behavior on high load now defers work by queueing target path for later instead of dropping the attempt.

### 2026-04-14 15:02 UCT
- Action: Updated boot launcher to use boot-aware daemon mode.
- Paths: `sysroot/strat-indexer-boot.sh`.
- Rationale: launcher now starts `/bin/stratterm-indexer --boot-daemon` so `boot_start` config is honored centrally by daemon logic.

### 2026-04-14 15:02 UCT
- Action: Added concrete settings template for future settings UI.
- Paths: `stratterm/indexer.conf.example`.
- Rationale: provides backend schema surface now so future System Settings app can write deterministic key/value configuration.

### 2026-04-14 15:02 UCT
- Action: Updated documentation for settings backend surface.
- Paths: `stratterm/README.md`.
- Rationale:
  - Documented config file locations, template file, and supported keys.
  - Clarified boot launcher now uses `--boot-daemon` mode.

### 2026-04-14 15:02 UCT
- Action: Ran post-change script syntax checks.
- Commands:
  - `sh -n sysroot/strat-indexer-boot.sh`
  - `sh -n scripts/phase7/prepare-minimal-rootfs.sh`
- Result: both passed.
- Rationale: verify shell-side boot/rootfs integration still parses cleanly.

### 2026-04-14 15:02 UCT
- Action: Ran post-change build validation attempts.
- Commands:
  - `cd stratterm && cargo build --bin stratterm-indexer`
  - `cd stratterm && cargo build --bin stratterm`
- Result: both failed (`cargo: command not found`) in current sandbox.
- Rationale: attempted compile validation after shared settings framework integration.

### 2026-04-14 15:08 UCT
- Action: Added backend persistence helpers for indexer settings framework.
- Paths: `stratterm/src/index_settings.rs`.
- Rationale:
  - Added save path and serialization helpers for config-backed settings writes.
  - Added write-target selection with fallback behavior (`/config/strat/indexer.conf` preferred, home fallback).
  - Added disable-flag helpers for future settings UI integration (`disable_flag_path`, `set_disable_flag`, `disable_flag_exists`).
  - Added path-list serialization + utility helpers to keep config IO deterministic.

### 2026-04-14 15:08 UCT
- Action: Added MVP settings application binary.
- Paths: `stratterm/src/bin/strat-settings.rs`.
- Rationale:
  - Implemented a minimal GTK settings window for indexer controls.
  - Added load/reload/save flow for all key backend fields.
  - Added hard-disable switch handling via disable-flag helpers.
  - UI is intentionally simple and backend-focused so future System Settings app can replace front-end while keeping same config schema.

### 2026-04-14 15:08 UCT
- Action: Updated crate exports for shared settings backend consumption by multiple binaries.
- Paths: `stratterm/src/lib.rs`.
- Rationale: expose `index_settings` module for `stratterm`, `stratterm-indexer`, and `strat-settings` binaries.

### 2026-04-14 15:08 UCT
- Action: Added concrete example config file for settings app and future system settings integration.
- Paths: `stratterm/indexer.conf.example`.
- Rationale: define explicit editable key surface for current MVP and future settings UI writers.

### 2026-04-14 15:08 UCT
- Action: Updated Stratterm Makefile targets for settings app and multi-bin builds.
- Paths: `stratterm/Makefile`.
- Rationale:
  - `build`/`release` now build all binaries (`--bins`) so install scripts can rely on emitted artifacts.
  - Added `run-settings` target for launching `strat-settings`.
  - `install-rootfs` now stages `strat-settings` in addition to existing binaries.

### 2026-04-14 15:08 UCT
- Action: Updated rootfs assembly to include settings app binary.
- Paths: `scripts/phase7/prepare-minimal-rootfs.sh`.
- Rationale:
  - Added `STRAT_SETTINGS_BIN_SOURCE` and copy/chmod path for `/bin/strat-settings`.
  - Added settings app to runtime dependency queue for staged library resolution.

### 2026-04-14 15:08 UCT
- Action: Added top-level convenience target for launching settings app.
- Paths: `Makefile`.
- Rationale: added `strat-settings` target to simplify invocation from repo root.

### 2026-04-14 15:08 UCT
- Action: Updated README docs for settings app usage and backend config role.
- Paths: `stratterm/README.md`.
- Rationale:
  - Added run instructions for `run-settings`.
  - Documented `strat-settings` as temporary MVP editor for indexer backend configuration.

### 2026-04-14 15:08 UCT
- Action: Ran post-change shell script syntax checks.
- Commands:
  - `sh -n sysroot/strat-indexer-boot.sh`
  - `sh -n scripts/phase7/prepare-minimal-rootfs.sh`
- Result: both passed.
- Rationale: verify boot/rootfs shell paths remain syntactically valid.

### 2026-04-14 15:08 UCT
- Action: Ran post-change Rust build validation attempts.
- Commands:
  - `cd stratterm && cargo build --bin strat-settings`
  - `cd stratterm && cargo build --bins`
- Result: both failed (`cargo: command not found`) in current sandbox.
- Rationale: attempted compile validation after adding settings app and backend save framework.

### 2026-04-14 15:12 UCT
- Action: Generated visual PNG mockup showing Strat Terminal and Strat Settings side-by-side.
- Paths: `stratterm/mockups/stratterm-and-settings-mockup.png`.
- Rationale: user requested a drawable PNG preview of the current terminal + settings app UX while runtime build tooling is unavailable in sandbox.

### 2026-04-14 15:12 UCT
- Action: Ran environment capability checks before rendering mockup.
- Commands:
  - `command -v python3`
  - Python import probe for `PIL`
  - `command -v magick`
  - `command -v convert`
  - `command -v ffmpeg`
  - `command -v cairo-renderer`
  - `command -v inkscape`
- Result:
  - `python3` available
  - `PIL` missing
  - image conversion/render CLI tools not found in sandbox PATH
- Rationale: determined no off-the-shelf image pipeline was available; switched to pure-Python PNG writer.

### 2026-04-14 15:12 UCT
- Action: Rendered PNG via pure-Python raster + custom PNG encoding script (no external dependencies).
- Paths: `stratterm/mockups/stratterm-and-settings-mockup.png`.
- Rationale: ensured deterministic image generation in restricted sandbox without Pillow/ImageMagick.

### 2026-04-14 15:22 UCT
- Action: Reworked `strat-settings` into icon-nested navigation model (main settings page -> terminal panel).
- Paths: `stratterm/src/bin/strat-settings.rs`.
- Rationale:
  - Added a top-level icon-grid settings home (`System Settings`) with category-style layout.
  - Added `Terminal` icon tile that opens nested terminal/indexer settings panel.
  - Added `Show All` back navigation in terminal panel to mirror classic icon-indexed settings UX.
  - Kept existing backend controls and save/reload behavior intact inside nested terminal panel.

### 2026-04-14 15:22 UCT
- Action: Updated docs for nested icon IA in settings app.
- Paths: `stratterm/README.md`.
- Rationale: documented that `strat-settings` now uses icon-based main page and terminal icon opens the terminal/indexer panel.

### 2026-04-14 15:22 UCT
- Action: Ran build validation attempt for updated settings app.
- Command: `cd stratterm && cargo build --bin strat-settings`
- Result: failed (`cargo: command not found`) in current sandbox.
- Rationale: compile sanity check after major settings-app UI architecture change.

### 2026-04-14 15:24 UCT
- Action: Added user-facing tooltips for terminal/indexer controls in nested StratTerm settings panel.
- Paths: `stratterm/src/bin/strat-settings.rs`.
- Rationale:
  - Replaced plain row wiring with `add_row_with_tooltip` helper so each setting label/input has a short explanation.
  - Added visible hint text in panel header to inform users they can hover for explanations.
  - Covers all existing settings fields (enable flags, boot toggle, frequency/batch, thresholds, roots/excludes, and UI pacing controls).

### 2026-04-14 15:24 UCT
- Action: Updated README to document tooltip behavior in settings app.
- Paths: `stratterm/README.md`.
- Rationale: make discoverability explicit for users and auditors.

### 2026-04-14 15:24 UCT
- Action: Ran post-change validation checks.
- Commands:
  - `grep -n "add_row_with_tooltip\|set_tooltip_text\|hover each setting" stratterm/src/bin/strat-settings.rs`
  - `cd stratterm && cargo build --bin strat-settings`
- Result:
  - tooltip wiring locations verified by grep
  - build failed (`cargo: command not found`) in current sandbox
- Rationale: attempted compile validation and quick source wiring verification.
