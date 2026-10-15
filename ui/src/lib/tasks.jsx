import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { createContext, useCallback, useContext, useEffect, useMemo, useState } from "react";
import { receiptLine } from "./format.js";
import { errorText } from "./i18n.js";
import { applyLiveEvent, useSession } from "./session.jsx";

const TaskContext = createContext(null);

/**
 * Live state of spec task runs, fed by `roudex://task` events: streamed turns
 * per task, pending approvals and what is running. It lives above the views
 * so a run keeps streaming while the user looks elsewhere. `revision` bumps
 * whenever stored task state may have changed.
 */
export function TaskProvider({ children }) {
  const { notify } = useSession();
  const [live, setLive] = useState({});
  const [turnsByTask, setTurnsByTask] = useState({});
  const [approvals, setApprovals] = useState([]);
  const [current, setCurrent] = useState(null);
  const [active, setActive] = useState(null);
  const [revision, setRevision] = useState(0);

  const refreshActive = useCallback(() => {
    invoke("task_active").then(setActive).catch(() => setActive(null));
  }, []);
  useEffect(refreshActive, [refreshActive]);

  useEffect(() => {
    const unlisten = listen("roudex://task", ({ payload }) => {
      const bump = () => setRevision((n) => n + 1);
      if (payload.type === "session") {
        const { taskId, event } = payload;
        setLive((all) => applyLiveEvent(all, event));
        switch (event.type) {
          case "turnStarted":
            setTurnsByTask((all) => ((all[taskId] ?? []).includes(event.turnId) ? all
              : { ...all, [taskId]: [...(all[taskId] ?? []), event.turnId] }));
            setCurrent(taskId);
            bump();
            break;
          case "approvalRequested":
            setApprovals((list) => [...list, { ...event, taskId }]);
            break;
          case "turnCompleted":
            setApprovals((list) => list.filter((item) => item.turnId !== event.turnId));
            bump();
            break;
          case "runtimeError":
            notify(`${taskId}: ${event.message}`);
            break;
          default:
            // `closed` arrives at the end of every run: each run has its own process.
            break;
        }
        return;
      }
      setCurrent((running) => (running === payload.taskId ? null : running));
      setApprovals((list) => list.filter((item) => item.taskId !== payload.taskId));
      refreshActive();
      bump();
      notify(payload.type === "failed" ? `${payload.taskId}: ${errorText(payload.error)}` : receiptLine(payload.taskId, payload.receipt));
    });
    return () => { unlisten.then((stop) => stop()); };
  }, [notify, refreshActive]);

  const resolveApproval = useCallback((item) => {
    setApprovals((list) => list.filter((entry) => entry !== item));
  }, []);

  const value = useMemo(() => ({
    live, turnsByTask, approvals, current, active, revision,
    resolveApproval,
    /** Call after a background run was accepted (dispatch, run all, resume). */
    started: (label) => setActive(label),
    bump: () => setRevision((n) => n + 1),
  }), [live, turnsByTask, approvals, current, active, revision, resolveApproval]);

  return <TaskContext.Provider value={value}>{children}</TaskContext.Provider>;
}

export function useTasks() {
  return useContext(TaskContext);
}
