import { clockTime, routeReason, tokens } from "./lib/format.js";

export const TURN_STATUS_LABEL = { running: "en curso", completed: "completado", interrupted: "interrumpido", failed: "falló" };
const COMMAND_STATUS = { failed: "falló", declined: "rechazado" };
export const APPROVAL_OPTIONS = [["accept", "aceptar"], ["acceptForSession", "aceptar en la sesión"], ["decline", "rechazar"], ["cancel", "cancelar turno"]];

/**
 * One turn of a transcript: prompt, router decision, streamed answer and the
 * commands it ran. `onEscalate` adds the "re-run with fuerte" action; `extra`
 * renders after the commands (a task's verification, for example).
 */
export function TurnLog({ turn, live, onEscalate, disabled, extra }) {
  const running = turn.status === "running" && !(live?.done);
  const text = live?.text || turn.response;
  const commands = live?.commands?.length ? live.commands : turn.commands;
  return <div className="log-turn">
    <span className="log-time">{clockTime(turn.created_at)}</span>
    <span className="log-prompt"><span className="accent">❯</span> {turn.prompt}</span>
    <span />
    <span className="log-router">router  <span className={`tier-text tier-${turn.tier}`}>{turn.tier}</span> · {turn.model} {turn.effort} · {routeReason(turn)}</span>
    <span />
    <span className="log-answer">{text || (running ? "" : "sin respuesta")}{running && <span className="caret" aria-hidden="true">▍</span>}</span>
    {commands?.map((command) => <CommandLine key={command.id} command={command} />)}
    {turn.error && <><span /><span className="log-error">{turn.error}</span></>}
    {extra}
    {!running && <><span />
      <span className="log-foot">─ turno {TURN_STATUS_LABEL[turn.status] ?? turn.status}{turn.tokens ? ` · ${tokens(turn.tokens.total)} tokens` : ""}
        {onEscalate && turn.tier !== "fuerte" && <button type="button" className="link small" disabled={disabled} onClick={() => onEscalate(turn)}>re-ejecutar con fuerte</button>}
      </span></>}
  </div>;
}

export function CommandLine({ command }) {
  const ok = command.exitCode === 0 || command.status === "completed";
  const failed = command.status === "failed" || (command.exitCode !== null && command.exitCode !== undefined && command.exitCode !== 0) || command.status === "declined";
  return <>
    <span />
    <span className="log-command">$ {command.command ?? command.type}
      <span className={failed ? "bad" : ok ? "good" : "accent"}>   {command.status === "en curso" ? "en curso" : failed ? `✗ ${COMMAND_STATUS[command.status] ?? command.status}` : "✓"}</span>
      {command.output && <pre>{command.output}</pre>}
    </span>
  </>;
}

/** A pending approval request from Codex, answered inline. */
export function ApprovalCard({ item, onAnswer }) {
  return <div className="approval" role="alert">
    <span className="approval-title">aprobación requerida · {item.kind === "fileChange" ? "cambio de archivos" : "comando"}</span>
    {item.command && <code>{item.command}</code>}
    {item.reason && <span className="approval-reason">{item.reason}</span>}
    <div className="row-wrap">
      {APPROVAL_OPTIONS.map(([decision, label]) => <button key={decision} type="button"
        className={decision === "accept" ? "primary" : decision === "acceptForSession" ? "amber-ghost" : "ghost"}
        onClick={() => onAnswer(item, decision)}>{label}</button>)}
    </div>
  </div>;
}
