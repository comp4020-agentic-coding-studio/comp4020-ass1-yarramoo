import { useEffect, useRef, useState } from "react";
import init, { Scene } from "./raytracer-wasm/pkg/raytracer_wasm.js";

const WIDTH = 480;
const HEIGHT = 300;
const SAMPLES_PER_PASS = 2;

const MATERIAL_KINDS = [
  { value: 0, label: "Lambertian (matte)" },
  { value: 1, label: "Metal" },
  { value: 2, label: "Dialectric (glass)" },
] as const;

type SphereControlState = {
  kind: number;
  colour: string;
  param: number;
};

// Indices into the fixed 4-sphere scene the wasm side builds (index 0 is the
// large ground sphere — left as-is, not exposed as a control). Initial
// values mirror Scene::new's defaults so the controls don't lie on mount.
const HERO_SPHERES: { index: number; label: string; initial: SphereControlState }[] = [
  { index: 1, label: "Left sphere", initial: { kind: 0, colour: "#1a3380", param: 0 } },
  { index: 2, label: "Middle sphere", initial: { kind: 2, colour: "#ffffff", param: 1.5 } },
  { index: 3, label: "Right sphere", initial: { kind: 1, colour: "#cc9933", param: 0 } },
];

function hexToRgb01(hex: string): [number, number, number] {
  const clean = hex.replace("#", "");
  const r = parseInt(clean.slice(0, 2), 16) / 255;
  const g = parseInt(clean.slice(2, 4), 16) / 255;
  const b = parseInt(clean.slice(4, 6), 16) / 255;
  return [r, g, b];
}

export function App() {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const sceneRef = useRef<Scene | null>(null);
  const [ready, setReady] = useState(false);
  const [sampleCount, setSampleCount] = useState(0);
  const [controls, setControls] = useState(() => HERO_SPHERES.map((s) => ({ ...s.initial })));

  useEffect(() => {
    let cancelled = false;
    init().then(() => {
      if (cancelled) return;
      sceneRef.current = new Scene(WIDTH, HEIGHT);
      setReady(true);
    });
    return () => {
      cancelled = true;
    };
  }, []);

  // The render loop lives entirely in refs — sample count is throttled to
  // ~2/sec purely to drive the status line, so React never re-renders 60x/sec.
  useEffect(() => {
    if (!ready) return;
    const canvas = canvasRef.current;
    const scene = sceneRef.current;
    const ctx = canvas?.getContext("2d");
    if (!canvas || !scene || !ctx) return;

    let frame: number;
    let lastStatusUpdate = 0;
    const loop = (t: number) => {
      scene.render_pass(SAMPLES_PER_PASS);
      const imageData = new ImageData(new Uint8ClampedArray(scene.pixels()), WIDTH, HEIGHT);
      ctx.putImageData(imageData, 0, 0);
      if (t - lastStatusUpdate > 500) {
        setSampleCount(scene.sample_count());
        lastStatusUpdate = t;
      }
      frame = requestAnimationFrame(loop);
    };
    frame = requestAnimationFrame(loop);
    return () => cancelAnimationFrame(frame);
  }, [ready]);

  // Pointer Events unify mouse and touch, so drag-to-orbit works at both
  // marking viewports with one implementation.
  useEffect(() => {
    const canvas = canvasRef.current;
    const scene = sceneRef.current;
    if (!ready || !canvas || !scene) return;

    let dragging = false;
    let lastX = 0;
    let lastY = 0;
    const onDown = (e: PointerEvent) => {
      dragging = true;
      lastX = e.clientX;
      lastY = e.clientY;
      canvas.setPointerCapture(e.pointerId);
    };
    const onMove = (e: PointerEvent) => {
      if (!dragging) return;
      scene.orbit_camera((e.clientY - lastY) * 0.005, -(e.clientX - lastX) * 0.005);
      lastX = e.clientX;
      lastY = e.clientY;
    };
    const onUp = () => {
      dragging = false;
    };

    canvas.addEventListener("pointerdown", onDown);
    canvas.addEventListener("pointermove", onMove);
    canvas.addEventListener("pointerup", onUp);
    return () => {
      canvas.removeEventListener("pointerdown", onDown);
      canvas.removeEventListener("pointermove", onMove);
      canvas.removeEventListener("pointerup", onUp);
    };
  }, [ready]);

  const updateMaterial = (i: number, next: Partial<SphereControlState>) => {
    setControls((prev) => {
      const updated = prev.map((c, idx) => (idx === i ? { ...c, ...next } : c));
      const state = updated[i];
      const [r, g, b] = hexToRgb01(state.colour);
      sceneRef.current?.set_sphere_material(HERO_SPHERES[i].index, state.kind, r, g, b, state.param);
      return updated;
    });
  };

  return (
    <>
      <header>
        <nav aria-label="Primary">
          <a href="./">Home</a>
        </nav>
      </header>
      <main>
        <h1>Interactive ray tracer</h1>
        <p>
          Drag the image to orbit the camera. The render sharpens progressively — every
          action restarts it, so give it a moment to settle.
        </p>
        <canvas
          ref={canvasRef}
          width={WIDTH}
          height={HEIGHT}
          className="raytracer-canvas"
          role="img"
          aria-label="Ray-traced scene of three spheres. Drag to orbit the camera."
        />
        <p data-testid="sample-count">Samples per pixel: {sampleCount}</p>
        <div className="controls" role="group" aria-label="Zoom">
          <button type="button" onClick={() => sceneRef.current?.zoom(-0.5)}>
            Zoom in
          </button>
          <button type="button" onClick={() => sceneRef.current?.zoom(0.5)}>
            Zoom out
          </button>
        </div>
        <fieldset className="materials">
          <legend>Materials</legend>
          {HERO_SPHERES.map((sphere, i) => {
            const state = controls[i];
            return (
              <div className="material-control" key={sphere.index}>
                <label>
                  {sphere.label} material
                  <select
                    value={state.kind}
                    onChange={(e) => updateMaterial(i, { kind: Number(e.target.value) })}
                  >
                    {MATERIAL_KINDS.map((k) => (
                      <option key={k.value} value={k.value}>
                        {k.label}
                      </option>
                    ))}
                  </select>
                </label>
                {state.kind !== 2 && (
                  <label>
                    Colour
                    <input
                      type="color"
                      value={state.colour}
                      onChange={(e) => updateMaterial(i, { colour: e.target.value })}
                    />
                  </label>
                )}
                {state.kind === 1 && (
                  <label>
                    Fuzz
                    <input
                      type="range"
                      min={0}
                      max={1}
                      step={0.05}
                      value={state.param}
                      onChange={(e) => updateMaterial(i, { param: Number(e.target.value) })}
                    />
                  </label>
                )}
                {state.kind === 2 && (
                  <label>
                    Refraction index
                    <input
                      type="range"
                      min={1}
                      max={2.5}
                      step={0.05}
                      value={state.param}
                      onChange={(e) => updateMaterial(i, { param: Number(e.target.value) })}
                    />
                  </label>
                )}
              </div>
            );
          })}
        </fieldset>
      </main>
    </>
  );
}
