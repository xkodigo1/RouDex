# Orchestrator example

A tiny project with one spec, `specs/001-notes`, to try the orchestrator end to
end with very little quota: two `economico` tasks that write a `NOTES.md` file.

- `HELLO-T1` creates `NOTES.md`.
- `HELLO-T2` depends on it and needs your approval before it runs. It appends
  the list of router tiers.

Each task is verified with `git grep`, so Git must be on your `PATH`. Codex
runs with write access to this folder only and without network.

## Run it

Copy the folder somewhere outside the repository first, so the run does not
change your RouDex checkout:

```powershell
Copy-Item -Recurse examples\hello-orchestrator $env:TEMP\hello-orchestrator
```

**In the app:**

1. In **proyectos**, open the copied folder.
2. In **tareas**, pick `hello-001-notes` under "sin cargar".
3. Press **▶ ejecutar listas**: `HELLO-T1` runs and is verified, and
   `HELLO-T2` stops at "espera aprobación".
4. Select `HELLO-T2`, press **aprobar**, then **▶ ejecutar**.

**From the CLI** (Codex approval requests are declined there):

```powershell
cd $env:TEMP\hello-orchestrator
router-cli spec-load specs\001-notes\tasks.json
router-cli task-dispatch hello-001-notes
router-cli task-approve HELLO-T2
router-cli task-dispatch hello-001-notes
```

See [docs/orchestrator.md](../../docs/orchestrator.md).
