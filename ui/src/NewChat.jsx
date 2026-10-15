import { invoke } from "@tauri-apps/api/core";
import { useState } from "react";
import AsciiField from "./AsciiField.jsx";
import Composer from "./Composer.jsx";
import { basename } from "./lib/format.js";
import { errorText, t } from "./lib/i18n.js";
import { useSession } from "./lib/session.jsx";

/** New-chat screen: ASCII field in the back, project picker and composer. */
export default function NewChat({ project, projects, config, onProject, onStarted }) {
  const session = useSession();
  const [menuOpen, setMenuOpen] = useState(false);
  const [picking, setPicking] = useState(false);
  const [path, setPath] = useState("");
  const [error, setError] = useState(null);

  async function choosePath(event) {
    event.preventDefault();
    try {
      await onProject(path.trim());
      setPicking(false);
      setMenuOpen(false);
      setPath("");
      setError(null);
    } catch (reason) {
      setError(errorText(reason));
    }
  }

  async function send(prompt, decision, access) {
    const sent = await invoke("chat_send", { project, threadId: null, prompt, decision, access });
    session.markStarted(sent.turnId);
    session.bump();
    onStarted(sent.threadId);
  }

  return <div className="new-chat">
    <AsciiField />
    <div className="new-chat-dock">
      <span className="hint">{project ? t("newChat.clickBackground") : t("newChat.pickProject")}</span>
      <div className="dock">
        {menuOpen && <div className="menu" role="menu" aria-label={t("newChat.projectMenu")}>
          {projects.map((item) => <button key={item} type="button" role="menuitemradio" aria-checked={item === project}
            className="menu-item" title={item} onClick={() => { onProject(item); setMenuOpen(false); }}>
            <span className="muted">▣</span><span className="grow ellipsis">{basename(item)}</span><span className="accent">{item === project ? "✓" : ""}</span>
          </button>)}
          {projects.length > 0 && <div className="menu-sep" />}
          {picking ? <form className="menu-path" onSubmit={choosePath}>
            <label className="visually-hidden" htmlFor="project-path">{t("newChat.folder")}</label>
            <input id="project-path" autoFocus placeholder={t("newChat.folderPlaceholder")} value={path} onChange={(event) => setPath(event.target.value)} />
            <button type="submit" className="ghost">{t("newChat.open")}</button>
          </form> : <button type="button" className="menu-item muted" onClick={() => setPicking(true)}>{t("newChat.chooseFolder")}</button>}
          {error && <p className="error-line">{error}</p>}
        </div>}
        <Composer project={project} config={config} onSend={send} compact chooseAccess placeholder={t("newChat.placeholder")}>
          <button type="button" className="project-button" aria-haspopup="menu" aria-expanded={menuOpen} onClick={() => setMenuOpen(!menuOpen)}>
            ▣ {project ? basename(project) : t("newChat.chooseProject")} <span className="muted">▾</span>
          </button>
          <span className="muted small">{t("newChat.routerAuto")}</span>
        </Composer>
      </div>
    </div>
  </div>;
}
