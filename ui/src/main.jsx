import { invoke } from "@tauri-apps/api/core";
import { useEffect, useRef, useState } from "react";
import { createRoot } from "react-dom/client";
import "./styles.css";

function localTime(seconds) {
  const date = new Date(Number(seconds) * 1000);
  return Number.isFinite(date.getTime()) ? date.toLocaleString() : seconds;
}

function Badge({ value }) {
  return <span className={`badge badge-${value}`}>{value}</span>;
}

function EventList({ events, total }) {
  return <>
    <p className="muted">{events.length} de {total} eventos{total > events.length ? " · últimos 200" : ""}</p>
    {events.length === 0 ? <p className="empty-state">No hay eventos registrados.</p> :
      <ol className="event-list">{events.map((event) => <li key={event.id}>
        <details>
          <summary><b>{event.event_type}</b> <span className="muted">#{event.id} · {localTime(event.created_at)}</span></summary>
          <code className="identifier">{event.run_id}</code>
          <pre>{JSON.stringify(event.payload, null, 2)}</pre>
        </details>
      </li>)}</ol>}
  </>;
}

function App() {
  const [snapshot, setSnapshot] = useState(null);
  const [error, setError] = useState(null);
  const [loading, setLoading] = useState(true);
  const [updated, setUpdated] = useState(null);
  const [selectedRun, setSelectedRun] = useState("");
  const [selectedTask, setSelectedTask] = useState("");
  const [statusFilter, setStatusFilter] = useState("all");
  const [activity, setActivity] = useState(null);
  const [eventError, setEventError] = useState(null);
  const [refresh, setRefresh] = useState(0);
  const refreshAction = useRef(null);

  useEffect(() => {
    let active = true;
    let pending = false;
    async function load() {
      if (pending) return;
      pending = true;
      setLoading(true);
      try {
        const value = await invoke("local_snapshot");
        if (!active) return;
        setSnapshot(value);
        setUpdated(new Date().toLocaleTimeString());
        setError(null);
        setSelectedTask((previous) => value.tasks.some((task) => task.selection_key === previous)
          ? previous : value.tasks[0]?.selection_key ?? "");
        setSelectedRun((previous) => value.runs.some((run) => run.run_id === previous)
          ? previous : value.runs[0]?.run_id ?? "");
      } catch (value) {
        if (active) setError(String(value));
      } finally {
        pending = false;
        if (active) setLoading(false);
      }
    }
    refreshAction.current = load;
    load();
    const timer = setInterval(load, 5000);
    return () => { active = false; clearInterval(timer); refreshAction.current = null; };
  }, []);

  useEffect(() => {
    let active = true;
    setActivity((previous) => previous?.run_id === selectedRun ? previous : null);
    setEventError(null);
    if (selectedRun) {
      invoke("local_run_events", { runId: selectedRun })
        .then((value) => { if (active) setActivity(value); })
        .catch((value) => { if (active) setEventError(String(value)); });
    }
    return () => { active = false; };
  }, [selectedRun, snapshot, refresh]);

  const tasks = snapshot?.tasks ?? [];
  const runs = snapshot?.runs ?? [];
  const approvals = snapshot?.approvals ?? [];
  const visibleTasks = tasks.filter((task) => statusFilter === "all" || task.status === statusFilter);
  const currentTask = tasks.find((task) => task.selection_key === selectedTask);
  const pendingApprovals = tasks.filter((task) => task.approval !== "not_required"
    && ["ready", "awaiting_approval"].includes(task.status)
    && !approvals.some((approval) => approval.task_id === task.id && approval.latest));

  return <main className="shell">
    <header className="topbar">
      <div><span className="eyebrow">LOCAL TASK DESK</span><h1>RouDex</h1></div>
      <span className="status-pill"><span className="status-dot" /> local-first · solo lectura</span>
    </header>
    <section className="desk-heading">
      <div><h2>Orquestación local</h2><p className="lede">Tasks, ejecuciones, aprobaciones y eventos de SQLite.</p></div>
      <button type="button" disabled={loading} onClick={() => { refreshAction.current?.(); setRefresh((n) => n + 1); }}>
        {loading ? "Actualizando…" : "Actualizar"}
      </button>
    </section>
    <p className="muted" role="status">{error ? "Sin conexión al estado local. Los datos anteriores pueden estar desactualizados."
      : loading ? "Leyendo estado local…"
      : snapshot?.initialized ? `Última lectura: ${updated} · actualización cada 5 segundos`
      : "Aún no existe estado local. Carga una especificación desde el CLI para comenzar."}</p>
    {error && <p className="error-banner" role="alert">No se pudo leer SQLite: {error}</p>}
    <section className="metrics" aria-label="Estado local">
      <article className="card metric-card"><span className="card-label">TASKS</span><strong>{snapshot ? snapshot.task_total : "—"}</strong><span className="muted">{snapshot?.status_counts.running ?? 0} en ejecución</span></article>
      <article className="card metric-card"><span className="card-label">EJECUCIONES</span><strong>{snapshot ? runs.length : "—"}</strong><span className="muted">persistidas localmente</span></article>
      <article className="card metric-card"><span className="card-label">APROBACIONES PENDIENTES</span><strong>{snapshot ? pendingApprovals.length : "—"}</strong><span className="muted">Las decisiones se registran desde el CLI.</span></article>
    </section>
    <section className="card panel section-panel" aria-labelledby="milestones-title">
      <h3 id="milestones-title">Progreso por hito</h3>
      {snapshot?.milestones.length ? <ul className="milestone-list">{snapshot.milestones.map((milestone) => {
        const completed = milestone.status_counts.completed ?? 0;
        return <li key={JSON.stringify([milestone.spec_id, milestone.milestone_id])}>
          <b>{milestone.spec_id} · {milestone.milestone_id ?? "Sin hito asignado"}</b>
          <span>{completed} / {milestone.task_total} completadas</span>
          <progress aria-label={`${milestone.spec_id} · ${milestone.milestone_id ?? "Sin hito asignado"}`} value={completed} max={milestone.task_total || 1} />
        </li>;
      })}</ul> : <p className="empty-state">{snapshot ? "No hay hitos con tasks registradas." : "Esperando estado local."}</p>}
    </section>
    <section className="card panel section-panel" aria-labelledby="tasks-title">
      <h3 id="tasks-title">Tasks</h3>
      <div className="task-filter"><label htmlFor="task-status">Estado de task</label>
        <select id="task-status" value={statusFilter} onChange={(event) => setStatusFilter(event.target.value)}>
          <option value="all">Todos ({snapshot?.task_total ?? 0})</option>
          {Object.entries(snapshot?.status_counts ?? {}).map(([status, count]) => <option key={status} value={status}>{status} ({count})</option>)}
          {statusFilter !== "all" && !(statusFilter in (snapshot?.status_counts ?? {})) && <option value={statusFilter}>{statusFilter} (0)</option>}
        </select>
        <span className="muted">{visibleTasks.length} de {snapshot?.task_total ?? 0} tasks</span>
      </div>
      {visibleTasks.length ? <div className="table-scroll"><table><thead><tr><th>Task / especificación</th><th>Estado</th><th>Modelo / esfuerzo</th><th>Dependencias</th><th>Aprobación</th></tr></thead><tbody>
        {visibleTasks.map((task) => <tr key={task.selection_key}><td><button type="button" className="task-button" aria-pressed={selectedTask === task.selection_key} onClick={() => setSelectedTask(task.selection_key)}>{task.id} · {task.title}</button><small>{task.spec_id} · {task.milestone_id ?? "Sin hito asignado"}</small></td><td><Badge value={task.status} /></td><td>{task.model}<small>{task.profile} · {task.effort}</small></td><td>{task.dependencies ?? "—"}</td><td>{task.approval === "not_required" ? "No requerida" : approvals.find((a) => a.task_id === task.id && a.latest)?.decision ?? "Pendiente"}</td></tr>)}
      </tbody></table></div> : <p className="empty-state">{!snapshot ? "Esperando estado local." : tasks.length ? "No hay tasks para este estado." : "No hay tasks registradas."}</p>}
      <div className="task-selection" aria-live="polite">
        {currentTask ? <><b>Task seleccionada: {currentTask.id} · {currentTask.title}</b><p><Badge value={currentTask.status} /> <span className="muted">{currentTask.spec_id} · {currentTask.milestone_id ?? "Sin hito asignado"}</span></p>
          {!visibleTasks.includes(currentTask) && <p className="muted">La task seleccionada está fuera del filtro actual.</p>}</>
          : <p className="muted">No hay task seleccionada.</p>}
      </div>
    </section>
    <div className="content-grid">
      <section className="card panel" aria-labelledby="runs-title"><h3 id="runs-title">Ejecuciones</h3>
        {runs.length ? <ul className="run-list">{runs.map((run) => <li key={run.run_id}>
          <button type="button" className="run-button" aria-pressed={selectedRun === run.run_id} onClick={() => setSelectedRun(run.run_id)}>
            <b>{run.task_id ?? "Entrada de mensaje"}</b> <Badge value={run.status} /><code className="identifier">{run.run_id}</code>
            <span className="muted">{run.model} · {run.effort} · {localTime(run.created_at)}</span>
          </button>
        </li>)}</ul> : <p className="empty-state">No hay ejecuciones registradas.</p>}
      </section>
      <section className="card panel" aria-labelledby="events-title"><h3 id="events-title">Eventos de la ejecución</h3>
        {selectedRun ? <><code className="identifier">{selectedRun}</code><p className="muted">Route: {runs.find((run) => run.run_id === selectedRun)?.route_id}</p>
          {eventError ? <p className="error-banner" role="alert">{eventError}</p>
            : activity?.run_id === selectedRun ? <EventList events={activity.events} total={activity.total} />
            : <p role="status">Leyendo eventos…</p>}</>
          : <p className="empty-state">Selecciona una ejecución para inspeccionar sus eventos.</p>}
      </section>
    </div>
    <section className="card panel section-panel" aria-labelledby="approvals-title"><h3 id="approvals-title">Historial de aprobaciones</h3>
      {approvals.length ? <div className="table-scroll"><table><thead><tr><th>Task</th><th>Decisión</th><th>Vigencia</th><th>Motivo</th><th>Fecha registrada</th></tr></thead><tbody>
        {approvals.map((approval) => <tr key={approval.id}><td>{approval.task_id}</td><td><Badge value={approval.decision} /></td><td>{approval.latest ? "Última decisión" : "Histórica"}</td><td>{approval.reason ?? "—"}</td><td>{localTime(approval.created_at)}</td></tr>)}
      </tbody></table></div> : <p className="empty-state">No hay decisiones registradas.</p>}
    </section>
    <section className="card panel section-panel" aria-labelledby="errors-title"><h3 id="errors-title">Errores y bloqueos registrados</h3>
      <EventList events={snapshot?.errors ?? []} total={snapshot?.error_count ?? 0} />
    </section>
    <footer className="card panel section-panel"><span className="card-label">ALMACENAMIENTO LOCAL</span>
      <code className="identifier">{snapshot?.state_path ?? "Sin lectura disponible"}</code>
      <p className="muted">SQLite · Tauri IPC · sin servicios remotos. Esta vista no aprueba ni ejecuta tasks.</p>
    </footer>
  </main>;
}

createRoot(document.getElementById("root")).render(<App />);
