import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";
import { createRoot } from "react-dom/client";
import "./styles.css";

function App() {
  const [status, setStatus] = useState(null);
  const [error, setError] = useState(null);

  useEffect(() => {
    invoke("local_status")
      .then(setStatus)
      .catch((value) => setError(String(value)));
  }, []);

  return (
    <main className="shell">
      <header className="topbar">
        <div>
          <span className="eyebrow">LOCAL TASK DESK</span>
          <h1>RouDex</h1>
        </div>
        <span className="status-pill"><span className="status-dot" /> local-first</span>
      </header>

      <section className="hero">
        <div>
          <p className="eyebrow">HITO 1</p>
          <h2>Mensajes, ejecuciones y eventos en un solo lugar.</h2>
          <p className="lede">
            Supervisa el enrutamiento local, sus aprobaciones y la continuidad de
            cada ejecución sin depender de servicios remotos.
          </p>
        </div>
        <div className="hero-mark" aria-hidden="true">RD</div>
      </section>

      <section className="metrics" aria-label="Estado local">
        <article className="card metric-card">
          <span className="card-label">EJECUCIONES</span>
          <strong>{status?.run_count ?? "—"}</strong>
          <span className="muted">persistidas localmente</span>
        </article>
        <article className="card metric-card">
          <span className="card-label">POLÍTICA</span>
          <strong>Aprobación manual</strong>
          <span className="muted">acciones sensibles requieren confirmación</span>
        </article>
        <article className="card metric-card">
          <span className="card-label">ESTADO</span>
          <strong>{error ? "Revisar" : "Disponible"}</strong>
          <span className="muted">{error ?? "runtime local conectado"}</span>
        </article>
      </section>

      <section className="content-grid">
        <article className="card panel">
          <div className="panel-heading">
            <div>
              <span className="card-label">ACTIVIDAD</span>
              <h3>Últimos eventos</h3>
            </div>
            <button type="button" disabled>Actualizar</button>
          </div>
          <div className="empty-state">
            <div className="empty-icon">+</div>
            <p>Aún no hay eventos para mostrar.</p>
            <span>Los mensajes enrutados aparecerán aquí.</span>
          </div>
        </article>
        <aside className="card panel side-panel">
          <span className="card-label">ALMACENAMIENTO</span>
          <h3>Estado local</h3>
          <code>{status?.state_path ?? "Inicializando…"}</code>
          <div className="line-item"><span>Persistencia</span><b>SQLite</b></div>
          <div className="line-item"><span>Transporte</span><b>stdio</b></div>
        </aside>
      </section>
    </main>
  );
}

createRoot(document.getElementById("root")).render(<App />);
