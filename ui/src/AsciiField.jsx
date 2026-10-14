import { useEffect, useRef } from "react";
import { AsciiEngine } from "./lib/asciiEngine.js";

/** Animated ASCII background; clicks drop a pumpkin at the pointer. */
export default function AsciiField({ figure = "calabaza", background = "murcielagos" }) {
  const canvasRef = useRef(null);
  const engineRef = useRef(null);

  useEffect(() => {
    const engine = new AsciiEngine(canvasRef.current, { figure, background, glyphs: "simbolos" });
    engineRef.current = engine;
    engine.start();
    return () => engine.stop();
  }, [figure, background]);

  function splash(event) {
    const box = canvasRef.current.getBoundingClientRect();
    engineRef.current?.splash(event.clientX - box.left, event.clientY - box.top);
  }

  return <canvas ref={canvasRef} className="ascii-field" aria-hidden="true" onClick={splash} />;
}
