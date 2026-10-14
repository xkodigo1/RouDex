import { listen } from "@tauri-apps/api/event";
import { createContext, useCallback, useContext, useEffect, useMemo, useState } from "react";

const SessionContext = createContext(null);

const emptyTurn = () => ({ text: "", commands: [], done: false });

/**
 * Live state of every app-server session, fed by `roudex://session` events:
 * streaming text and commands per turn, pending approvals, rate limits and
 * notices. `revision` bumps when a turn ends so views reload stored data.
 */
export function SessionProvider({ children }) {
  const [live, setLive] = useState({});
  const [approvals, setApprovals] = useState([]);
  const [notices, setNotices] = useState([]);
  const [rateLimits, setRateLimits] = useState(null);
  const [revision, setRevision] = useState(0);

  const notify = useCallback((text) => {
    setNotices((list) => [...list.slice(-3), { id: Date.now() + Math.random(), text }]);
  }, []);
  const dismiss = useCallback((id) => setNotices((list) => list.filter((n) => n.id !== id)), []);
  const markStarted = useCallback((turnId) => {
    setLive((all) => (all[turnId] ? all : { ...all, [turnId]: emptyTurn() }));
  }, []);
  const resolveApproval = useCallback((item) => {
    setApprovals((list) => list.filter((entry) => entry !== item));
  }, []);

  useEffect(() => {
    const unlisten = listen("roudex://session", ({ payload: event }) => {
      switch (event.type) {
        case "turnStarted":
          setLive((all) => ({ ...all, [event.turnId]: all[event.turnId] ?? emptyTurn() }));
          break;
        case "agentDelta":
          setLive((all) => {
            const turn = all[event.turnId] ?? emptyTurn();
            return { ...all, [event.turnId]: { ...turn, text: turn.text + event.delta } };
          });
          break;
        case "itemStarted":
        case "itemCompleted": {
          const item = event.item ?? {};
          if (item.type !== "commandExecution" && item.type !== "fileChange") break;
          setLive((all) => {
            const turn = all[event.turnId] ?? emptyTurn();
            const previous = turn.commands.find((c) => c.id === item.id);
            const entry = {
              id: item.id,
              type: item.type,
              command: item.command ?? (item.changes ? item.changes.map((c) => c.path).join(", ") : previous?.command),
              status: event.type === "itemCompleted" ? (item.status ?? "completado") : "en curso",
              exitCode: item.exitCode ?? previous?.exitCode ?? null,
              output: previous?.output ?? "",
            };
            const commands = [...turn.commands.filter((c) => c.id !== item.id), entry];
            return { ...all, [event.turnId]: { ...turn, commands } };
          });
          break;
        }
        case "commandOutput":
          setLive((all) => {
            const turn = all[event.turnId];
            if (!turn) return all;
            const commands = turn.commands.map((c) => (c.id === event.itemId
              ? { ...c, output: (c.output + event.delta).slice(-2000) } : c));
            return { ...all, [event.turnId]: { ...turn, commands } };
          });
          break;
        case "approvalRequested":
          setApprovals((list) => [...list, event]);
          break;
        case "rateLimits":
          setRateLimits((current) => ({ ...(current ?? {}), ...event.snapshot }));
          break;
        case "turnCompleted":
          setLive((all) => (all[event.turnId] ? { ...all, [event.turnId]: { ...all[event.turnId], done: true } } : all));
          setApprovals((list) => list.filter((item) => item.turnId !== event.turnId));
          setRevision((n) => n + 1);
          break;
        case "runtimeError":
          notify(event.message);
          break;
        case "closed":
          notify(`La sesión con Codex se cerró: ${event.reason}`);
          break;
        default:
          break;
      }
    });
    return () => { unlisten.then((stop) => stop()); };
  }, [notify]);

  const value = useMemo(() => ({
    live, approvals, notices, rateLimits, revision,
    notify, dismiss, markStarted, resolveApproval, setRateLimits,
    bump: () => setRevision((n) => n + 1),
  }), [live, approvals, notices, rateLimits, revision, notify, dismiss, markStarted, resolveApproval]);

  return <SessionContext.Provider value={value}>{children}</SessionContext.Provider>;
}

export function useSession() {
  return useContext(SessionContext);
}
