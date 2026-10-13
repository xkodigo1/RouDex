# Specification 001 publication handoff

Prepared on 2026-10-13 by the routed T-009 worker. This document is a
publication checklist until the parent workflow records the final remote
evidence below.
The routed worker completed the code and acceptance audit. The enclosing
workflow owns Git metadata and GitHub publication; no worker guard should be
cleared and no Git metadata should be moved to evade policy.

## Review before publication

- Read [acceptance-evidence.md](acceptance-evidence.md), especially the T-001
  historical assignment mismatch and missing `task.verified` event. Preserve
  historical receipts; resolve the discrepancy explicitly in the task record.
- Record a native UI smoke result, or retain the unverified limitation in the
  release decision and notes. Browser fixture evidence is not native testing.
- Collect T-009's eventual terminal event and independent verification result.
  Its immutable verify command tests code but cannot prove a GitHub publication.
  Do not label the whole milestone complete solely because that command passes.
- Review only the scoped changes listed below. At the start of this audit the
  worktree was clean. No repository settings or credentials were modified.
- Secret/local-state heuristic scan: **46 tracked files and 149 reachable
  historical blobs** at the audited baseline, no findings. Recheck the final
  staged tree and diff. A final scan of **49 publication files**, including the
  three new audit documents, also had no findings; the nine-task manifest is
  acyclic and relative document links resolve. Build artifacts and the temporary signature trust file
  are ignored under build directories and must not be staged.
- Audit signatures using the configured public key. All 22 commits from the
  specification's introduction through the baseline passed locally. Remote
  signature verification was not available and remains required.

## Intended commit units

1. `test: verify independent invalid manifest acceptance cases`
   - `crates/router-cli/tests/hook_protocol.rs`
   - Three independent A7 cases, no runtime behavior changes.
2. `docs: record specification 001 acceptance audit`
   - `specs/001-task-orchestration/spec.md`
   - `specs/001-task-orchestration/tasks.md`
   - `specs/001-task-orchestration/tasks.json`
   - `specs/001-task-orchestration/acceptance-evidence.md`
   - `specs/001-task-orchestration/publication-handoff.md`
   - `specs/001-task-orchestration/release-notes.md`

Use explicit paths with `git add`, inspect `git diff --cached --check` and the
staged diff, then `git commit -S` for each coherent unit. Verify each new commit
and push it separately without force. Update task/commit references from actual
results; keep publication status pending until the release exists.

## Tag and release

The observed branch is `kodigo/prompt-routing`; the highest local tag is
`v0.4.15`. Remote branch/tag/release state could not be refreshed. Select an
unused milestone version only after inspecting the remote; no version is
reserved by this handoff. Do not overwrite or move existing tags.

After the audit gates are closed and the intended signed commit is pushed:

1. Create a **signed annotated tag** at that exact commit with `git tag -s`.
   Verify it with `git verify-tag`, then push that exact tag.
2. Publish with `gh release create <tag> --verify-tag --title <title>
   --notes-file specs/001-task-orchestration/release-notes.md`. Remove the draft
   notice only when the documented gates have actually been resolved. Keep
   any remaining validation limits in the public notes.
3. Verify that the remote tag peels to the intended commit and that the release
   reports the same tag, is published, and has the intended notes. Verify
   remote commit signatures and retain the release URL.
4. Record final task/spec status and publication evidence in a signed follow-up
   audit commit if needed. Do not amend published commits or retag the release.

## Final publication receipt

The parent workflow completed the publication without amending or moving the
published commit:

- Audit commit: `d80fb7441d8289ee902c0f5ae6d6fc630523f4da`, signed and pushed
  to `kodigo/prompt-routing`.
- Tag: signed annotated `v0.5.0`, object
  `e296695c7c1a0be3b229683923490da0b1df2fd1`, peeling to the audit commit.
- Release: [v0.5.0 — Local task orchestration](https://github.com/xkodigo1/RouDex/releases/tag/v0.5.0),
  published and not draft.
- Remote refs verified: the branch and peeled tag resolved to
  `d80fb7441d8289ee902c0f5ae6d6fc630523f4da` at publication time.
- T-009 terminal and the independent `cargo test --workspace` verification
  were already recorded by the coordinator before publication.

The historical T-001 assignment discrepancy and the unverified native Tauri
smoke remain explicitly documented limitations. Existing package versions
remain unchanged, and no installer or binary artifact was uploaded.
