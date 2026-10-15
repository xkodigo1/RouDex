import { clockTime, routeReason, tokens } from "./lib/format.js";
import { t } from "./lib/i18n.js";

export const APPROVAL_OPTIONS = ["accept", "acceptForSession", "decline", "cancel"];

/** A turn status in words ("completado", "completed"…). */
export function turnStatusLabel(status) {
  return t(`turn.status.${status}`);
}

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
    <span className="log-answer">{text || (running ? "" : t("turn.noAnswer"))}{running && <span className="caret" aria-hidden="true">▍</span>}</span>
    {commands?.map((command) => <CommandLine key={command.id} command={command} />)}
    {turn.error && <><span /><span className="log-error">{turn.error}</span></>}
    {extra}
    {!running && <><span />
      <span className="log-foot">{t("turn.footer", { status: turnStatusLabel(turn.status) })}{turn.tokens ? t("turn.tokens", { count: tokens(turn.tokens.total) }) : ""}
        {onEscalate && turn.tier !== "fuerte" && <button type="button" className="link small" disabled={disabled} onClick={() => onEscalate(turn)}>{t("turn.escalate")}</button>}
      </span></>}
  </div>;
}

/** A command a turn ran; `status` is `inProgress` while it runs. */
export function CommandLine({ command }) {
  const ok = command.exitCode === 0 || command.status === "completed";
  const failed = command.status === "failed" || (command.exitCode !== null && command.exitCode !== undefined && command.exitCode !== 0) || command.status === "declined";
  const label = command.status === "inProgress" ? t("command.running")
    : failed ? `✗ ${command.status === "declined" ? t("command.declined") : t("command.failed")}` : "✓";
  return <>
    <span />
    <span className="log-command">$ {command.command ?? command.type}
      <span className={failed ? "bad" : ok ? "good" : "accent"}>   {label}</span>
      {command.output && <pre>{command.output}</pre>}
    </span>
  </>;
}

/** A pending approval request from Codex, answered inline. */
export function ApprovalCard({ item, onAnswer }) {
  const kind = item.kind === "fileChange" ? t("approval.kind.fileChange") : t("approval.kind.command");
  return <div className="approval" role="alert">
    <span className="approval-title">{t("approval.title", { kind })}</span>
    {item.command && <code>{item.command}</code>}
    {item.reason && <span className="approval-reason">{item.reason}</span>}
    <div className="row-wrap">
      {APPROVAL_OPTIONS.map((decision) => <button key={decision} type="button"
        className={decision === "accept" ? "primary" : decision === "acceptForSession" ? "amber-ghost" : "ghost"}
        onClick={() => onAnswer(item, decision)}>{t(`approval.${decision}`)}</button>)}
    </div>
  </div>;
}
