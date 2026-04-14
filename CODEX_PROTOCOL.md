# Codex Protocol — StratOS

This file governs how Codex operates in this repo. Read it at the start of every session.

---

## The one non-negotiable rule

**Update TALKING.md before you move on from any file touch.**

Not after. Not at the end of the session. Before you start the next task.

Claude cannot audit what it doesn't know about. If you push a file without logging it, Claude will detect the change by timestamp and audit it cold — without your context. That means slower feedback, missed intent, and re-work.

---

## What to post in TALKING.md

Every entry needs:
- Date + `(Codex)`
- File(s) touched
- What changed and why (one or two lines is fine)
- Build/validation status

Example:
```
- 2026-04-12 (Codex): `stratboot/src/partition.c` — added strat_find_partition_by_name().
  Uses HARDDRIVE_DEVICE_PATH traversal instead of EFI_PARTITION_INFO_PROTOCOL (not available in GNU-EFI here).
  Build status: make clean all passes, no warnings.
```

If the build failed or something is blocked, say so. Claude needs to know.

---

## When to post

| Event | Post to TALKING.md? |
|---|---|
| Created a new file | Yes, immediately |
| Edited an existing file | Yes, immediately |
| Built an artifact (EFI, image, etc.) | Yes, one line with validation result |
| Ran a script that produced output | Yes, one line with result |
| Hit a blocker | Yes — describe it, don't silently skip |
| Read a file but made no changes | No |

---

## Deliberation

If you're about to do something that:
- contradicts Claude's last audit finding
- changes a design decision
- is outside the current phase scope

**Stop and post in TALKING.md first.** Wait for Claude to respond before proceeding. That's what the log is for.

---

## Phase discipline

- Complete the current phase before starting the next.
- If a phase gate is blocked, document the blocker in TALKING.md and get explicit sign-off before skipping ahead.
- Do not touch files Claude has marked CLEAN unless you have a reason — if you do, log it.

---

## Session start checklist

1. Read the last 50 lines of TALKING.md to get current state.
2. Read this file.
3. Post a session-start note in TALKING.md: current phase, current task, any blockers.
4. Work. Log every file touch. Ask before deviating.
