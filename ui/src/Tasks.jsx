import { invoke } from "@tauri-apps/api/core";
import { useEffect, useMemo, useRef, useState } from "react";
import { ApprovalCard, CommandLine, TurnLog } from "./Transcript.jsx";
import { TASK_STATUS_LABEL, TIERS, basename, percent, readStored, reasonText, receiptLine, routeReason, writeStored } from "./lib/format.js";
import { useSession } from "./lib/session.jsx";
import { useTasks } from "./lib/tasks.jsx";

const STATUS_MARK = { planned: "○", ready: "○", awaiting_approval: "?", running: "●", completed: "✓", failed: "✗", blocked: "■", interrupted: "‖" };
// Statuses a dispatch can start from (the coordinator checks the rest).
const OPEN = ["planned", "ready", "blocked", "awaiting_approval"];
const DECISION_LABEL = { approved: "aprobada", denied: "denegada" };
const SANDBOX_LABEL = { workspaceWrite: "escritura en el proyecto", readOnly: "solo lectura" };
// Stored run events worth a line in the transcript.
const EVENT_NOTE = {
  "turn.error": "el turno se cortó",
  "turn.invalid": "respuesta inválida del runtime",
  "turn.incomplete": "el turno no terminó",
  "thread.start.failed": "no se pudo abrir el hilo",
  "thread.resume.failed": "no se pudo reanudar el hilo",
  "thread.resumed": "hilo reanudado",
  "task.blocked": "bloqueada",
  "task.recovered": "marcada como interrumpida",
};

function StatusMark({ status }) {
  return <span className={`status-mark status-${status}`} aria-hidden="true">{STATUS_MARK[status] ?? "·"}</span>;
}

const needsApproval = (task) => task.approval !== "not_required";

/** Why a task cannot be dispatched now, or null if it can. */
function blocker(card, cards) {
  if (!card.dependenciesCompleted) {
    const pending = card.task.depends_on.filter((id) => cards.find((c) => c.task.id === id)?.task.status !== "completed");
    return `faltan dependencias: ${pending.join(", ")}`;
  }
  if (needsApproval(card.task) && card.approval?.decision !== "approved") {
    return card.approval?.decision === "denied" ? "la aprobación fue denegada" : "necesita aprobación";
  }
  return card.routeError ?? null;
}

const runnable = (card, cards) => OPEN.includes(card.task.status) && !card.running && !blocker(card, cards);

function routeBase(route) {
  return { tier: route?.tier, model: route?.model, effort: route?.effort, source: route?.source ?? "spec", reason: route?.reason ?? "" };
}

function verification(payload, failed, fallback) {
  const result = payload.result ?? {};
  const output = [result.stdout, result.stderr].filter(Boolean).join("\n").trim();
  return { id: "verify", command: payload.command ?? fallback, status: failed ? "failed" : "completed",
    exitCode: result.exitCode ?? null, output: output.slice(-2000),
    // With an exit code the failure speaks for itself.
    reason: failed && result.exitCode === undefined && payload.reason ? reasonText(payload.reason) : null };
}

/** A stored run rebuilt from its evidence: turns and incidents in order, and the verification. */
function runLog(events, task) {
  const started = events.find((event) => event.event_type === "task.started")?.payload ?? {};
  const base = { ...routeBase(started.route), tier: started.tier ?? started.route?.tier,
    model: started.model ?? started.route?.model, effort: started.effort ?? started.route?.effort };
  const turns = [];
  const entries = [];
  let verify = null;
  let turn = null;
  for (const { event_type: type, payload = {} } of events) {
    if (type === "turn/started") {
      const id = payload.turn?.id;
      turn = turns.find((t) => t.turn_id === id);
      if (!turn) {
        turn = { ...base, turn_id: id, prompt: task.title, status: "running", response: "", commands: [], error: null };
        turns.push(turn);
        entries.push({ turn });
      }
    } else if (type === "item/agentMessage/delta" && turn) {
      turn.response += payload.delta ?? "";
    } else if (type === "item/completed" && turn) {
      const item = payload.item ?? {};
      if (item.type === "commandExecution" || item.type === "fileChange") {
        turn.commands.push({ id: item.id, type: item.type, command: item.command ?? item.changes?.map((c) => c.path).join(", "),
          status: item.status, exitCode: item.exitCode ?? null, output: (item.aggregatedOutput ?? "").slice(-2000) });
      }
    } else if (type === "turn/completed" || type === "turn.completed") {
      const id = payload.turnId ?? payload.turn?.id;
      const done = turns.find((t) => t.turn_id === id) ?? turn;
      if (done) {
        done.status = payload.status ?? payload.turn?.status ?? done.status;
        if (!done.response && payload.text) done.response = payload.text;
      }
    } else if (type === "task.verified" || type === "verification.failed") {
      verify = verification(payload, type === "verification.failed", task.verify);
    } else if (EVENT_NOTE[type]) {
      entries.push({ note: { type, text: `${EVENT_NOTE[type]}${payload.reason ? `: ${reasonText(payload.reason)}` : ""}` } });
    }
  }
  return { turns, entries, verify };
}

function Verification({ verify, running, command }) {
  if (!verify && !running) return null;
  const line = verify ?? { id: "verify", command, status: "en curso", exitCode: null };
  return <>
    <span />
    <span className="log-router">verificación{verify?.exitCode !== null && verify?.exitCode !== undefined ? ` · salida ${verify.exitCode}` : ""}{verify?.reason ? ` · ${verify.reason}` : ""}</span>
    <CommandLine command={line} />
  </>;
}

function RunTranscript({ card, events, onAnswer }) {
  const tasks = useTasks();
  const { task } = card;
  const log = useMemo(() => runLog(events, task), [events, task]);
  const liveIds = (tasks.turnsByTask[task.id] ?? [])
    .filter((id) => !log.turns.some((turn) => turn.turn_id === id) && !tasks.live[id]?.done);
  const entries = [...log.entries, ...liveIds.map((id) => ({ turn: { ...routeBase(card.route), turn_id: id, prompt: task.title,
    status: "running", response: "", commands: [], error: null } }))];
  // A turn stored as running belongs to a run that is no longer active.
  const ended = card.latestRun?.status && card.latestRun.status !== "running" ? card.latestRun.status : "interrupted";
  const shown = entries.map((entry) => (entry.turn?.status === "running" && !card.running && !tasks.live[entry.turn.turn_id]
    ? { turn: { ...entry.turn, status: ended } } : entry));
  const turns = shown.filter((entry) => entry.turn).map((entry) => entry.turn);
  const lastTurn = turns.at(-1);
  const turnsDone = turns.every((turn) => turn.status !== "running" || tasks.live[turn.turn_id]?.done);
  const verifying = card.running && turns.length > 0 && turnsDone && !log.verify;
  const approvals = tasks.approvals.filter((item) => item.taskId === task.id);
  const run = card.latestRun;

  if (!shown.length && !approvals.length) {
    return <div className="placeholder"><h2>Sin ejecuciones</h2>
      <p className="muted">{card.running ? "Arrancando Codex…" : "Ejecuta la tarea para ver aquí su transcripción."}</p></div>;
  }
  return <>
    {run && <p className="run-head muted small">ejecución {run.run_id.slice(0, 8)} · {TASK_STATUS_LABEL[run.status] ?? run.status} · {run.model} {run.effort}</p>}
    {shown.map(({ turn, note }, index) => (turn
      ? <TurnLog key={turn.turn_id ?? index} turn={turn} live={tasks.live[turn.turn_id]}
        extra={turn === lastTurn && <Verification verify={log.verify} running={verifying} command={task.verify} />} />
      : <div key={index} className="log-turn"><span />
        <span className={note.type === "thread.resumed" ? "muted" : "log-error"}>{note.text}</span></div>))}
    {approvals.map((item) => <ApprovalCard key={JSON.stringify(item.requestId)} item={item} onAnswer={onAnswer} />)}
  </>;
}

function TaskDetail({ card, cards, onSelect }) {
  const { task, route } = card;
  const roots = task.sandbox.writable_roots?.join(", ");
  return <div className="task-detail">
    <dl>
      <dt>prompt</dt><dd><pre>{task.prompt}</pre></dd>
      <dt>aceptación</dt><dd><ul>{task.acceptance.map((item, index) => <li key={index}>{item}</li>)}</ul></dd>
      <dt>verificación</dt><dd><code>$ {task.verify}</code></dd>
      <dt>dependencias</dt><dd className="row-wrap">{task.depends_on.length ? task.depends_on.map((id) => {
        const dependency = cards.find((c) => c.task.id === id);
        return <button key={id} type="button" className="dep-link" onClick={() => onSelect(id)}>
          <StatusMark status={dependency?.task.status} /> {id}</button>;
      }) : <span className="muted">ninguna</span>}</dd>
      <dt>tier</dt><dd>{task.tier} en la spec{task.profile ? ` (perfil ${task.profile})` : ""}{card.tierOverride ? ` · elegido a mano: ${card.tierOverride}` : ""}</dd>
      <dt>ruta</dt><dd>{route ? <><span className={`tier-text tier-${route.tier}`}>{route.tier}</span> · {route.model} {route.effort} · {routeReason(route)}</>
        : <span className="error-line">{card.routeError}</span>}</dd>
      {route?.risk?.length > 0 && <><dt>riesgo</dt><dd className="warn-text">{route.risk.map((match) => match.value).join(", ")}</dd></>}
      <dt>aprobación</dt><dd>{needsApproval(task) ? DECISION_LABEL[card.approval?.decision] ?? "pendiente" : "no requerida"}</dd>
      <dt>alcance</dt><dd>{task.scope.join(", ")}</dd>
      <dt>sandbox</dt><dd>{SANDBOX_LABEL[task.sandbox.type] ?? task.sandbox.type}{roots ? ` · ${roots}` : ""}</dd>
      <dt>carpeta</dt><dd>{task.cwd}</dd>
    </dl>
  </div>;
}

/** Specs of the project and the loaded ones; picking an unloaded one loads it. */
function SpecPicker({ specs, files, specId, project, onPick }) {
  const unloaded = files.filter((file) => !file.loaded && !file.error);
  return <label className="field">spec
    <select value={specId} onChange={(event) => onPick(event.target.value)}>
      {!specs.some((spec) => spec.specId === specId) && <option value="">elige una spec</option>}
      {specs.length > 0 && <optgroup label="cargadas">
        {specs.map((spec) => <option key={spec.specId} value={spec.specId}>{spec.specId} · {spec.completed}/{spec.tasks}</option>)}
      </optgroup>}
      {unloaded.length > 0 && <optgroup label={`sin cargar · ${basename(project)}`}>
        {unloaded.map((file) => <option key={file.path} value={`file:${file.path}`}>{file.specId} · {file.tasks} tareas</option>)}
      </optgroup>}
    </select>
  </label>;
}

export default function Tasks({ project }) {
  const session = useSession();
  const tasks = useTasks();
  const [files, setFiles] = useState([]);
  const [specs, setSpecs] = useState(null);
  const [specId, setSpecId] = useState(() => readStored("roudex.spec", ""));
  const [board, setBoard] = useState(null);
  const [selected, setSelected] = useState(null);
  const [tab, setTab] = useState("detalle");
  const [events, setEvents] = useState([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState(null);
  const endRef = useRef(null);

  useEffect(() => {
    invoke("spec_list").then(setSpecs).catch((reason) => setError(String(reason)));
    if (!project) { setFiles([]); return; }
    invoke("spec_files", { project }).then(setFiles).catch((reason) => setError(String(reason)));
  }, [project, tasks.revision]);

  const choose = (id) => {
    setSpecId(id);
    writeStored("roudex.spec", id);
  };

  // Without a valid choice, show the most recently loaded spec.
  useEffect(() => {
    if (specs?.length && !specs.some((spec) => spec.specId === specId)) choose(specs[0].specId);
  }, [specs]);

  useEffect(() => {
    if (!specId) { setBoard(null); return undefined; }
    let current = true;
    invoke("task_board", { specId }).then((value) => { if (current) setBoard(value); })
      .catch((reason) => { if (current) { setBoard(null); setError(String(reason)); } });
    return () => { current = false; };
  }, [specId, tasks.revision]);

  const cards = board?.tasks ?? [];
  const card = cards.find((item) => item.task.id === selected) ?? null;
  const runId = card?.latestRun?.run_id;

  const select = (id, preferRun = false) => {
    setSelected(id);
    const next = cards.find((item) => item.task.id === id);
    setTab(preferRun || next?.running || next?.latestRun ? "ejecucion" : "detalle");
  };

  useEffect(() => {
    if (board && !card && cards.length) select((cards.find((item) => item.running) ?? cards[0]).task.id);
  }, [board]);

  // Follow the task that starts streaming (a single run or "run all ready").
  useEffect(() => {
    if (tasks.current && cards.some((item) => item.task.id === tasks.current)) select(tasks.current, true);
  }, [tasks.current]);

  useEffect(() => {
    if (!runId) { setEvents([]); return undefined; }
    let current = true;
    invoke("task_run_events", { runId }).then((value) => { if (current) setEvents(value); })
      .catch((reason) => { if (current) setError(String(reason)); });
    return () => { current = false; };
  }, [runId, tasks.revision]);

  useEffect(() => {
    if (tab === "ejecucion") endRef.current?.scrollIntoView({ block: "end" });
  }, [tab, events, tasks.live, tasks.approvals]);

  async function act(command, args, after) {
    setBusy(true);
    setError(null);
    try {
      const result = await invoke(command, args);
      await after?.(result);
      tasks.bump();
    } catch (reason) {
      setError(String(reason));
    } finally {
      setBusy(false);
    }
  }

  async function pick(value) {
    if (!value.startsWith("file:")) { choose(value); return; }
    await act("spec_load", { path: value.slice(5) }, (spec) => {
      choose(spec.specId);
      const count = spec.warnings.length;
      if (count) session.notify(`${spec.specId} cargada con ${count} ${count === 1 ? "aviso" : "avisos"}`);
    });
  }

  const taskId = card?.task.id;
  const dispatch = () => act("task_dispatch", { taskId }, () => { tasks.started(taskId); setTab("ejecucion"); });
  const actions = {
    run: dispatch,
    runReady: () => act("task_dispatch_ready", { specId }, () => tasks.started(`spec ${specId}`)),
    stop: () => act("task_interrupt", { taskId }),
    resume: () => act("task_resume", { runId }, () => { tasks.started(taskId); setTab("ejecucion"); }),
    // A retry makes the task ready again and starts a new run right away.
    retry: () => act("task_retry", { taskId }, async (receipt) => {
      if (receipt.status !== "ready") throw new Error(receiptLine(taskId, receipt));
      await invoke("task_dispatch", { taskId });
      tasks.started(taskId);
      setTab("ejecucion");
    }),
    recover: () => act("task_recover", { taskId }, (receipt) => session.notify(receiptLine(taskId, receipt))),
    decide: (approved) => act("task_decide", { taskId, approved }),
    tier: (tier) => act("task_set_tier", { taskId, tier }),
    reload: () => act("spec_load", { path: board.spec.path }, () => session.notify(`${specId} recargada desde el archivo`)),
  };

  async function answer(item, decision) {
    try {
      await invoke("task_approval", { taskId: item.taskId, requestId: item.requestId, decision });
      tasks.resolveApproval(item);
    } catch (reason) { setError(String(reason)); }
  }

  const groups = useMemo(() => {
    if (!board) return [];
    const byId = new Map(cards.map((item) => [item.task.id, item]));
    const listed = new Set();
    const result = board.spec.milestones.map((milestone) => {
      const members = milestone.task_ids.map((id) => byId.get(id)).filter(Boolean);
      members.forEach((item) => listed.add(item.task.id));
      return { id: milestone.id, title: milestone.title, cards: members };
    });
    const rest = cards.filter((item) => !listed.has(item.task.id));
    if (rest.length) result.push({ id: "", title: "sin hito", cards: rest });
    return result;
  }, [board]);

  const ready = cards.filter((item) => runnable(item, cards));
  const broken = files.filter((file) => file.error);
  const busyApp = Boolean(tasks.active);
  const waiting = new Set(tasks.approvals.map((item) => item.taskId));

  return <div className="threads tasks">
    <section className="thread-list" aria-label="Specs y tareas">
      <div className="thread-list-head">
        <div className="row-between"><h1>Tareas</h1>
          {board && <button type="button" className="ghost" disabled={!ready.length || busyApp || busy} onClick={actions.runReady}
            title="Ejecuta una tras otra, en orden de dependencias, las tareas que pueden correr ahora">▶ ejecutar listas · {ready.length}</button>}
        </div>
        <SpecPicker specs={specs ?? []} files={files} specId={specId} project={project} onPick={pick} />
        {board && <div className="spec-summary">
          <div className="row-between small">
            <span className="muted ellipsis" title={board.spec.title}>{board.spec.title}</span>
            {board.spec.path && <button type="button" className="link small" disabled={busy} onClick={actions.reload}>recargar</button>}
          </div>
          <div className="bar"><div className="bar-fill ok" style={{ width: percent(board.spec.completed, board.spec.tasks) }} /></div>
          <span className="muted small">{board.spec.completed}/{board.spec.tasks} completadas{tasks.active ? ` · en curso: ${tasks.active}` : ""}</span>
          {board.spec.warnings.length > 0 && <details className="warn-text small">
            <summary>{board.spec.warnings.length} {board.spec.warnings.length === 1 ? "aviso" : "avisos"} al cargar</summary>
            {board.spec.warnings.map((warning, index) => <p key={index}>{warning}</p>)}
          </details>}
        </div>}
        {broken.map((file) => <p key={file.path} className="error-line small">{basename(file.path.replace(/[\\/]tasks\.json$/, ""))}: {file.error}</p>)}
      </div>
      <div className="thread-items">
        {groups.map((group) => <div key={group.id || "rest"} className="milestone" role="group" aria-label={group.title}>
          <div className="milestone-head"><span className="ellipsis">{group.id ? `${group.id} · ` : ""}{group.title}</span>
            <span>{group.cards.filter((item) => item.task.status === "completed").length}/{group.cards.length}</span></div>
          {group.cards.map((item) => <button key={item.task.id} type="button" className="thread-item" aria-pressed={item.task.id === selected}
            onClick={() => select(item.task.id)}>
            <span className="row-start"><StatusMark status={item.task.status} /><span className="muted">{item.task.id}</span><span className="ellipsis">{item.task.title}</span></span>
            <span className="row-between muted small">
              <span>{waiting.has(item.task.id) ? "esperando tu respuesta" : TASK_STATUS_LABEL[item.task.status] ?? item.task.status}
                {needsApproval(item.task) && OPEN.includes(item.task.status) && item.approval?.decision !== "approved" ? " · pide aprobación" : ""}</span>
              {item.route && <span className={`tier-text tier-${item.route.tier}`}>{item.route.tier}{item.tierOverride ? " ·m" : ""}</span>}
            </span>
          </button>)}
        </div>)}
        {!board && <p className="muted pad">{specs === null ? "Cargando…" : project
          ? "Elige una spec del proyecto para cargarla."
          : "Elige un proyecto en «proyectos» para ver sus specs."}</p>}
      </div>
    </section>

    <main className="thread-main">
      {!card ? <div className="placeholder"><h2>Sin tarea</h2><p className="muted">Carga una spec y elige una tarea.</p></div> : <>
        <div className="thread-head">
          <div className="thread-title">
            <h2>{card.task.title}</h2>
            <span className="muted small">{card.task.id}{card.task.milestone ? ` · ${card.task.milestone}` : ""} · <StatusMark status={card.task.status} /> {TASK_STATUS_LABEL[card.task.status] ?? card.task.status}</span>
          </div>
          {card.route && <span className={`tier-chip tier-${card.route.tier}`} title={card.route.reason}>{card.route.tier} · {card.route.model} {card.route.effort}</span>}
        </div>
        <TaskActions card={card} cards={cards} busy={busy || busyApp} actions={actions} />
        <div className="chips task-tabs" role="tablist" aria-label="Vista de la tarea">
          {[["detalle", "detalle"], ["ejecucion", "ejecución"]].map(([id, label]) => <button key={id} type="button" role="tab" className="chip"
            aria-selected={tab === id} aria-pressed={tab === id} onClick={() => setTab(id)}>{label}</button>)}
        </div>
        {tab === "detalle" ? <TaskDetail card={card} cards={cards} onSelect={(id) => select(id)} />
          : <div className="transcript" aria-label="Transcripción de la tarea">
            <RunTranscript card={card} events={events} onAnswer={answer} />
            <div ref={endRef} />
          </div>}
        {error && <p className="error-line pad" role="alert">{error}</p>}
      </>}
    </main>
  </div>;
}

function TaskActions({ card, cards, busy, actions }) {
  const { task, running } = card;
  const open = OPEN.includes(task.status);
  const reason = open ? blocker(card, cards) : null;
  const stale = task.status === "running" && !running;
  const tierEditable = !running && !["running", "completed"].includes(task.status);
  const loweredRisk = card.route?.risk?.length > 0 && card.route.tier !== "fuerte";
  // A completed task has nothing left to do.
  if (!running && !open && !stale && !tierEditable) return null;
  return <div className="task-actions">
    {running && <button type="button" className="ghost" onClick={actions.stop}>■ detener</button>}
    {open && <button type="button" className="primary" disabled={busy || Boolean(reason)} title={reason ?? undefined} onClick={actions.run}>▶ ejecutar</button>}
    {task.status === "interrupted" && card.latestRun?.status === "interrupted" && <button type="button" className="primary" disabled={busy}
      title="Sigue en el mismo hilo sin repetir lo completado" onClick={actions.resume}>reanudar</button>}
    {(task.status === "interrupted" || task.status === "failed") && <button type="button" className="ghost" disabled={busy}
      title="Empieza una ejecución nueva" onClick={actions.retry}>reintentar</button>}
    {stale && <button type="button" className="amber-ghost" disabled={busy}
      title="La app se cerró durante la ejecución: márcala como interrumpida" onClick={actions.recover}>recuperar</button>}
    {needsApproval(task) && open && <>
      <button type="button" className="ghost" disabled={card.approval?.decision === "approved"} onClick={() => actions.decide(true)}>aprobar</button>
      <button type="button" className="ghost" disabled={card.approval?.decision === "denied"} onClick={() => actions.decide(false)}>denegar</button>
    </>}
    {reason && <span className="muted small">{reason}</span>}
    <span className="grow" />
    {tierEditable && <div className="segmented" role="group" aria-label="Tier de la tarea">
      <button type="button" aria-pressed={!card.tierOverride} title="El tier de la spec, subido a fuerte si hay riesgo"
        onClick={() => card.tierOverride && actions.tier(null)}>spec</button>
      {TIERS.map((tier) => <button key={tier} type="button" className={`tier-text tier-${tier}`} aria-pressed={card.tierOverride === tier}
        onClick={() => card.tierOverride !== tier && actions.tier(tier)}>{tier}</button>)}
    </div>}
    {loweredRisk && <p className="error-line task-warning">Esta tarea tiene riesgo y la bajaste por debajo de fuerte.</p>}
  </div>;
}
