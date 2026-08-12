import { useEffect, useRef, useState } from "react";
import init, { Scene } from "./raytracer-wasm/pkg/raytracer_wasm.js";

const WIDTH = 480;
const HEIGHT = 300;
const DEFAULT_SAMPLES_PER_PASS = 2;
const DEFAULT_MAX_DEPTH = 8;
// Below this much total pointer movement (in CSS px), a press+release counts
// as a click-to-trace rather than a drag-to-orbit.
const CLICK_MOVEMENT_THRESHOLD = 5;

const MATERIAL_KINDS = [
  {
    value: 0,
    label: "Lambertian (matte)",
    blurb:
      "Diffuse scattering: each ray bounces off in a random direction biased toward the surface normal, not a mirror reflection — that's what makes matte surfaces look flat and directionless.",
  },
  {
    value: 1,
    label: "Metal",
    blurb:
      "Specular reflection: the ray bounces at a mirror angle (angle in = angle out). Fuzz randomly perturbs that reflected direction, blurring a sharp mirror into a brushed-metal look.",
  },
  {
    value: 2,
    label: "Dialectric (glass)",
    blurb:
      "Refraction via Snell's law, with an angle-dependent chance of reflecting instead (Schlick's approximation) — this is why glass edges look mirror-bright at grazing angles even though the centre lets light straight through.",
  },
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

// `path` is flat [x0, y0, x1, y1, ...] pixel-space coordinates from
// Scene::trace_pixel. Drawn fresh every frame since the progressive render's
// putImageData wipes anything drawn on a prior frame.
function drawTracedPath(ctx: CanvasRenderingContext2D, path: number[] | null) {
  if (!path || path.length < 2) return;
  ctx.strokeStyle = "#ff3b30";
  ctx.lineWidth = 2;
  ctx.beginPath();
  ctx.moveTo(path[0], path[1]);
  for (let k = 2; k < path.length; k += 2) {
    ctx.lineTo(path[k], path[k + 1]);
  }
  ctx.stroke();
  ctx.fillStyle = "#ff3b30";
  for (let k = 0; k < path.length; k += 2) {
    ctx.beginPath();
    ctx.arc(path[k], path[k + 1], 3, 0, Math.PI * 2);
    ctx.fill();
  }
}

function hexToRgb01(hex: string): [number, number, number] {
  const clean = hex.replace("#", "");
  const r = parseInt(clean.slice(0, 2), 16) / 255;
  const g = parseInt(clean.slice(2, 4), 16) / 255;
  const b = parseInt(clean.slice(4, 6), 16) / 255;
  return [r, g, b];
}

export function App() {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const snapshotCanvasRef = useRef<HTMLCanvasElement | null>(null);
  const sceneRef = useRef<Scene | null>(null);
  // Flat [x0, y0, x1, y1, ...] pixel-space coordinates of the last traced
  // ray's bounce path, or null when there's nothing to draw. Lives in a ref
  // (not state) because the render loop redraws it every frame alongside
  // the progressive render, not on its own React render cycle.
  const tracedPathRef = useRef<number[] | null>(null);
  const [ready, setReady] = useState(false);
  const [sampleCount, setSampleCount] = useState(0);
  const [controls, setControls] = useState(() => HERO_SPHERES.map((s) => ({ ...s.initial })));
  const [samplesPerPass, setSamplesPerPass] = useState(DEFAULT_SAMPLES_PER_PASS);
  const samplesPerPassRef = useRef(DEFAULT_SAMPLES_PER_PASS);
  const [maxDepth, setMaxDepth] = useState(DEFAULT_MAX_DEPTH);
  // Mirrors `maxDepth` state for the pointer-events effect below, which only
  // re-subscribes on `[ready]` — without this, a click-to-trace fired after
  // adjusting the slider would trace with a stale, mount-time depth value.
  const maxDepthRef = useRef(DEFAULT_MAX_DEPTH);

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
      scene.render_pass(samplesPerPassRef.current);
      const imageData = new ImageData(new Uint8ClampedArray(scene.pixels()), WIDTH, HEIGHT);
      ctx.putImageData(imageData, 0, 0);
      drawTracedPath(ctx, tracedPathRef.current);
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
  // marking viewports with one implementation. The same listeners also
  // resolve a no-movement press+release as a click-to-trace.
  useEffect(() => {
    const canvas = canvasRef.current;
    const scene = sceneRef.current;
    if (!ready || !canvas || !scene) return;

    let dragging = false;
    let dragConfirmed = false;
    let lastX = 0;
    let lastY = 0;
    let totalMovement = 0;
    const onDown = (e: PointerEvent) => {
      dragging = true;
      dragConfirmed = false;
      lastX = e.clientX;
      lastY = e.clientY;
      totalMovement = 0;
      canvas.setPointerCapture(e.pointerId);
    };
    const onMove = (e: PointerEvent) => {
      if (!dragging) return;
      totalMovement += Math.hypot(e.clientX - lastX, e.clientY - lastY);
      // Below the threshold, don't orbit yet — a tap's inevitable few
      // pixels of jitter (worse on touch than mouse) would otherwise nudge
      // the camera and reset the render before onUp even decides whether
      // this is a click or a drag.
      if (!dragConfirmed && totalMovement < CLICK_MOVEMENT_THRESHOLD) {
        lastX = e.clientX;
        lastY = e.clientY;
        return;
      }
      dragConfirmed = true;
      scene.orbit_camera((e.clientY - lastY) * 0.005, -(e.clientX - lastX) * 0.005);
      lastX = e.clientX;
      lastY = e.clientY;
      // The camera just moved — any previously traced path now points at
      // rays that no longer exist.
      tracedPathRef.current = null;
    };
    const onUp = (e: PointerEvent) => {
      dragging = false;
      if (dragConfirmed) return;

      const rect = canvas.getBoundingClientRect();
      const px = Math.min(WIDTH - 1, Math.max(0, Math.round(((e.clientX - rect.left) / rect.width) * WIDTH)));
      const py = Math.min(HEIGHT - 1, Math.max(0, Math.round(((e.clientY - rect.top) / rect.height) * HEIGHT)));
      const path = Array.from(scene.trace_pixel(px, py, maxDepthRef.current));
      tracedPathRef.current = path.length >= 2 ? path : null;
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

  const updateSamplesPerPass = (n: number) => {
    samplesPerPassRef.current = n;
    setSamplesPerPass(n);
  };

  const updateMaxDepth = (n: number) => {
    maxDepthRef.current = n;
    setMaxDepth(n);
    sceneRef.current?.set_max_depth(n);
    tracedPathRef.current = null;
  };

  const showOneSample = () => {
    const scene = sceneRef.current;
    const canvas = snapshotCanvasRef.current;
    const ctx = canvas?.getContext("2d");
    if (!scene || !canvas || !ctx) return;
    const imageData = new ImageData(new Uint8ClampedArray(scene.render_snapshot(1)), WIDTH, HEIGHT);
    ctx.putImageData(imageData, 0, 0);
  };

  const updateMaterial = (i: number, next: Partial<SphereControlState>) => {
    setControls((prev) => {
      const updated = prev.map((c, idx) => (idx === i ? { ...c, ...next } : c));
      const state = updated[i];
      const [r, g, b] = hexToRgb01(state.colour);
      sceneRef.current?.set_sphere_material(HERO_SPHERES[i].index, state.kind, r, g, b, state.param);
      tracedPathRef.current = null;
      return updated;
    });
  };

  const zoom = (delta: number) => {
    sceneRef.current?.zoom(delta);
    tracedPathRef.current = null;
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
          action restarts it, so give it a moment to settle. Click without dragging to
          trace a single ray: each red dot is a bounce, and the line shows the path light
          took through the scene before it was absorbed or escaped.
        </p>
        <canvas
          ref={canvasRef}
          width={WIDTH}
          height={HEIGHT}
          className="raytracer-canvas"
          role="img"
          aria-label="Ray-traced scene of three spheres. Drag to orbit the camera, or click to trace a ray."
        />
        <p data-testid="sample-count">Samples per pixel: {sampleCount}</p>
        <div className="controls" role="group" aria-label="Zoom">
          <button type="button" onClick={() => zoom(-0.5)}>
            Zoom in
          </button>
          <button type="button" onClick={() => zoom(0.5)}>
            Zoom out
          </button>
        </div>
        <fieldset className="render-settings">
          <legend>Render settings</legend>
          <div className="render-setting">
            <label>
              Max bounce depth: {maxDepth}
              <input
                type="range"
                min={1}
                max={16}
                step={1}
                value={maxDepth}
                onChange={(e) => updateMaxDepth(Number(e.target.value))}
              />
            </label>
            <p className="render-setting-blurb">
              Each bounce traces one more ray segment. Indirect light — reflected in the
              metal sphere or refracted through the glass one — needs several bounces to
              resolve, so a shallow depth truncates that light early and darkens or
              blackens reflective and refractive surfaces.
            </p>
          </div>
          <div className="render-setting">
            <label>
              Samples per frame: {samplesPerPass}
              <input
                type="range"
                min={1}
                max={8}
                step={1}
                value={samplesPerPass}
                onChange={(e) => updateSamplesPerPass(Number(e.target.value))}
              />
            </label>
            <p className="render-setting-blurb">
              Each sample is one more random ray per pixel averaged into the running
              estimate — this is Monte Carlo integration. More samples per frame settle
              the image faster but trace more rays before the next frame paints, so very
              high values can make the canvas feel less responsive.
            </p>
          </div>
        </fieldset>
        <fieldset className="comparison">
          <legend>1 sample vs. converged</legend>
          <p className="comparison-blurb">
            Ray tracing is Monte Carlo integration: each pixel averages many random
            samples together. A single sample per pixel is noisy — this is what every
            pixel above started from. Compare it against the converged render, which has
            averaged {sampleCount} samples per pixel.
          </p>
          <button type="button" onClick={showOneSample}>
            Show 1 sample
          </button>
          <canvas
            ref={snapshotCanvasRef}
            width={WIDTH}
            height={HEIGHT}
            className="raytracer-canvas snapshot-canvas"
            role="img"
            aria-label="A single noisy sample per pixel, for comparison against the converged render above."
          />
        </fieldset>
        <fieldset className="materials">
          <legend>Materials</legend>
          {HERO_SPHERES.map((sphere, i) => {
            const state = controls[i];
            const activeMaterial = MATERIAL_KINDS.find((k) => k.value === state.kind);
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
                {activeMaterial && (
                  <p className="material-blurb" data-testid={`material-blurb-${sphere.index}`}>
                    {activeMaterial.blurb}
                  </p>
                )}
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
