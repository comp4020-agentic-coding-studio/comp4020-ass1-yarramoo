import { useEffect, useRef, useState } from "react";
import type { RefObject } from "react";
import { createPortal } from "react-dom";
import init, { Scene } from "./raytracer-wasm/pkg/raytracer_wasm.js";

// The nav/heading live as a static shell in index.html (so the built HTML
// carries them before any JS runs — see the invariants spec). This portals
// the one dynamic piece, the sample count, into that same static <p> rather
// than App rendering its own competing header. In tests, App mounts without
// that static shell present, so it falls back to rendering the paragraph
// inline — same testid, same text, no shell required.
function SampleCountReadout({ sampleCount }: { sampleCount: number }) {
  const content = <>Samples per pixel: {sampleCount}</>;
  const mount = typeof document !== "undefined" ? document.getElementById("sample-count-mount") : null;
  return mount ? createPortal(content, mount) : <p data-testid="sample-count">{content}</p>;
}

// The render itself fills the whole viewport rather than a fixed canvas
// size, but a full-screen *ray traced* resolution would tank convergence
// speed (this renderer is single-threaded CPU Monte Carlo). Instead the
// internal render resolution keeps roughly today's pixel count (480x300 =
// 144,000px) and only its *aspect ratio* tracks the viewport, so the CSS
// stretch to fill the screen is uniform rather than distorting, and the
// samples/sec rate never regresses regardless of window size.
const RENDER_PIXEL_BUDGET = 480 * 300;
const MIN_EDGE = 120;
const MAX_EDGE = 900;
function computeRenderSize(viewportWidth: number, viewportHeight: number) {
  const aspect = viewportWidth / viewportHeight;
  const width = Math.round(Math.sqrt(RENDER_PIXEL_BUDGET * aspect));
  const height = Math.round(Math.sqrt(RENDER_PIXEL_BUDGET / aspect));
  return {
    width: Math.min(MAX_EDGE, Math.max(MIN_EDGE, width)),
    height: Math.min(MAX_EDGE, Math.max(MIN_EDGE, height)),
  };
}

const DEFAULT_SAMPLES_PER_PASS = 2;
const DEFAULT_MAX_DEPTH = 8;
// Below this much total pointer movement (in CSS px), a press+release counts
// as a click-to-trace rather than a drag-to-orbit.
const CLICK_MOVEMENT_THRESHOLD = 5;
// Caps how many traced-ray segments accumulate for a single pixel before the
// oldest are dropped — otherwise repeatedly clicking the same spot in
// accumulate mode grows the per-pixel history (and the redraw cost) without
// bound.
const MAX_RAYS_PER_PIXEL = 40;

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

// Mirrors Scene::load_preset's id list. `CUSTOM_SCENE_ID` isn't a preset id
// at all — selecting it just reveals the JSON textarea below.
const PRESETS = [
  { id: 0, label: "Classic spheres" },
  { id: 1, label: "Cornell box" },
  { id: 2, label: "Foggy room" },
  { id: 3, label: "Dispersive prism" },
  { id: 4, label: "Depth of field" },
] as const;
const CUSTOM_SCENE_ID = 5;

// Mirrors each preset's `defocus_angle` in raytracer-wasm/src/presets.rs —
// used only to sync the DOF slider's displayed value after a preset load,
// since `Scene` has no getter for the angle it just set internally.
const PRESET_DEFAULT_DOF: Record<number, number> = { 0: 0, 1: 0, 2: 0, 3: 0, 4: 1.2 };

// Mirrors Scene::set_sampling_strategy's mode encoding.
const SAMPLING_STRATEGIES = [
  { value: 0, label: "Naive (approximate diffuse)" },
  { value: 1, label: "Cosine-weighted importance sampling" },
  { value: 2, label: "Mixture (cosine + direct light sampling)" },
] as const;

// Mirrors Scene::sample_directions's mode encoding.
const SUNBURST_MODES = [
  { value: 0, label: "Uniform hemisphere" },
  { value: 1, label: "Cosine-weighted" },
  { value: 2, label: "Aimed at the light (NEE)" },
] as const;

// How many single-bounce sample directions the sunburst view fires per
// click — enough to show cosine's clustering near the normal without
// crowding the canvas into an unreadable tangle of lines.
const SUNBURST_SAMPLE_COUNT = 28;

// The classic 4-sphere scene expressed in the custom-JSON schema — a real,
// working starting point for "author your own scene, like the ray tracing
// project from the beginning" rather than a blank textarea.
const EXAMPLE_JSON = `{
  "objects": [
    { "type": "sphere", "centre": [0, -100.5, -1], "radius": 100, "material": { "kind": "lambertian", "albedo": [0.5, 0.5, 0.5] } },
    { "type": "sphere", "centre": [0, 0, -1.2], "radius": 0.5, "material": { "kind": "lambertian", "albedo": [0.1, 0.2, 0.5] } },
    { "type": "sphere", "centre": [-1, 0, -1], "radius": 0.5, "material": { "kind": "dielectric", "ior": 1.5 } },
    { "type": "sphere", "centre": [1, 0, -1], "radius": 0.5, "material": { "kind": "metal", "albedo": [0.8, 0.6, 0.2], "fuzz": 0 } }
  ]
}`;

// One bounce path out of Scene::trace_pixel's multi-path bundle: `points` is
// flat [x0, y0, x1, y1, ...] pixel-space coordinates, `termination` is
// 0=escaped/1=absorbed/2=emitted, `channel` is -1=neutral or 0/1/2=R/G/B
// (only set by dispersive glass), and `sources` is one PDF-source tag per
// point (-1=not PDF-sampled, 0=cosine-weighted, 1=aimed at a light) — only
// meaningful when `channel` is -1, since a dispersive path's bounces are
// deterministic refractions, not PDF samples.
type TracedPathSegment = {
  termination: number;
  channel: number;
  points: number[];
  sources: number[];
};

// Unpacks Scene::trace_pixel's flat
// `[numPaths, terminationKind, channel, len, x0,y0, ..., s0,s1, ...]` buffer.
// A plain click on a non-dispersive, non-defocus scene yields exactly one
// segment — the same shape the old single-polyline API returned, plus the
// trailing per-vertex sources tail.
function parseTracedPaths(raw: number[]): TracedPathSegment[] {
  if (raw.length === 0) return [];
  const numPaths = raw[0];
  const paths: TracedPathSegment[] = [];
  let offset = 1;
  for (let p = 0; p < numPaths; p++) {
    const termination = raw[offset];
    const channel = raw[offset + 1];
    const len = raw[offset + 2];
    offset += 3;
    const points = raw.slice(offset, offset + len * 2);
    offset += len * 2;
    const sources = raw.slice(offset, offset + len);
    offset += len;
    paths.push({ termination, channel, points, sources });
  }
  return paths;
}

// Unpacks Scene::sample_directions's flat
// `[originX, originY, count, x0,y0, ..., x(count-1),y(count-1)]` buffer into
// the sunburst fan it describes, or null if the click missed everything.
type SunburstSample = {
  originX: number;
  originY: number;
  tips: number[];
};

function parseSampleDirections(raw: number[]): SunburstSample | null {
  if (raw.length === 0) return null;
  const [originX, originY, count] = raw;
  return { originX, originY, tips: raw.slice(3, 3 + count * 2) };
}

// Keys the per-pixel ray history hashmap — pixel-space integer coordinates,
// same ones passed to Scene::trace_pixel.
function pixelKey(px: number, py: number): string {
  return `${px}:${py}`;
}

// Colour-codes a path by what happened to it. A dispersion channel (0/1/2)
// always wins with its pure colour, since separating those channels visually
// *is* the point; a neutral path (-1) is tinted by how it ended — escaped
// keeps the original red, absorbed fades to grey, and hitting a light glows
// gold so that outcome reads as different without any label.
function pathColour(termination: number, channel: number): string {
  if (channel === 0) return "#ff3030";
  if (channel === 1) return "#30c060";
  if (channel === 2) return "#3080ff";
  if (termination === 2) return "#ffcc33";
  if (termination === 1) return "#8a8a8a";
  return "#ff3b30";
}

// Colour for one bounce *segment* by which PDF actually produced it — used
// only under the Mixture strategy, and only for non-dispersive paths (a
// dispersive path's bounces are deterministic refractions, not PDF samples,
// so pathColour's channel-based colouring keeps priority there). Gold
// matches pathColour's existing "hit a light" tint deliberately, so a
// mixture path visibly snaps to the same colour once next-event estimation
// actually lands a bounce on the light.
function pdfSourceColour(source: number): string {
  if (source === 1) return "#ffcc33";
  if (source === 0) return "#33ccaa";
  return "#8a8a8a";
}

// Drawn fresh every frame since the progressive render's putImageData wipes
// anything drawn on a prior frame. Multiple segments (a dispersion split, or
// a depth-of-field lens bundle) are drawn at reduced alpha so the overlaps
// read as density rather than clutter. `colourBySource` (true only under the
// Mixture strategy) switches non-dispersive paths from one uniform stroke
// colour to colouring each bounce segment individually by its PDF-source
// tag — dots stay outcome-coloured (via pathColour) either way, since that
// meaning ("where did this ray end up") doesn't change.
function drawTracedPath(ctx: CanvasRenderingContext2D, paths: TracedPathSegment[] | null, colourBySource: boolean) {
  if (!paths || paths.length === 0) return;
  ctx.save();
  // A single traced ray (or a small dispersion/DOF bundle) reads fine at
  // 0.55 alpha; accumulate mode can pile up dozens of overlapping rays for
  // one pixel, where that same 0.55 would just paint a solid blob. Scaling
  // down with the count keeps the overlay legible as density builds up.
  ctx.globalAlpha = paths.length > 1 ? Math.max(0.12, 1 / Math.sqrt(paths.length)) : 1;
  for (const path of paths) {
    if (path.points.length < 2) continue;
    const dotColour = pathColour(path.termination, path.channel);
    ctx.lineWidth = 2;
    if (colourBySource && path.channel < 0) {
      for (let k = 2; k < path.points.length; k += 2) {
        ctx.strokeStyle = pdfSourceColour(path.sources[k / 2] ?? -1);
        ctx.shadowBlur = 0;
        ctx.beginPath();
        ctx.moveTo(path.points[k - 2], path.points[k - 1]);
        ctx.lineTo(path.points[k], path.points[k + 1]);
        ctx.stroke();
      }
    } else {
      ctx.shadowBlur = path.termination === 2 && path.channel < 0 ? 8 : 0;
      ctx.shadowColor = dotColour;
      ctx.strokeStyle = dotColour;
      ctx.beginPath();
      ctx.moveTo(path.points[0], path.points[1]);
      for (let k = 2; k < path.points.length; k += 2) {
        ctx.lineTo(path.points[k], path.points[k + 1]);
      }
      ctx.stroke();
    }
    ctx.fillStyle = dotColour;
    for (let k = 0; k < path.points.length; k += 2) {
      ctx.beginPath();
      ctx.arc(path.points[k], path.points[k + 1], 3, 0, Math.PI * 2);
      ctx.fill();
    }
  }
  ctx.restore();
}

// Colour for a sunburst fan by which mode produced it — teal/gold match
// pdfSourceColour's cosine/light tags above so the "which PDF" cue stays
// consistent between the two visualizations; uniform-hemisphere gets its
// own hue since pdfSourceColour has no tag for it (it never appears in a
// real render, only here).
function sunburstColour(mode: number): string {
  if (mode === 2) return "#ffcc33";
  if (mode === 0) return "#8866ff";
  return "#33ccaa";
}

// Draws Scene::sample_directions's single-bounce fan: one short line per
// sampled direction from the hit point, plus a white dot marking the origin
// itself (distinct from every bounce-path dot colour, so the two overlays
// never look ambiguous when both happen to be visible).
function drawSunburst(ctx: CanvasRenderingContext2D, sample: SunburstSample | null, mode: number) {
  if (!sample || sample.tips.length === 0) return;
  ctx.save();
  ctx.strokeStyle = sunburstColour(mode);
  ctx.lineWidth = 1.5;
  ctx.globalAlpha = 0.7;
  for (let k = 0; k < sample.tips.length; k += 2) {
    ctx.beginPath();
    ctx.moveTo(sample.originX, sample.originY);
    ctx.lineTo(sample.tips[k], sample.tips[k + 1]);
    ctx.stroke();
  }
  ctx.globalAlpha = 1;
  ctx.fillStyle = "#fff";
  ctx.beginPath();
  ctx.arc(sample.originX, sample.originY, 4, 0, Math.PI * 2);
  ctx.fill();
  ctx.restore();
}

// Monte Carlo π estimator demo (book 3, ch. 2) — self-contained constants,
// no wasm involvement. MC_GRID_SIZE is the stratification grid used when
// jittering is on: each new point lands in the next cell of this fixed
// MC_GRID_SIZE x MC_GRID_SIZE grid, cycling back to cell 0 once all cells
// have been visited once.
const MC_CANVAS_SIZE = 200;
const MC_POINTS_PER_TICK = 4;
const MC_GRID_SIZE = 8;
const MC_HISTORY_LIMIT = 120;

// Draws the running π-estimate history as a simple line chart with a dashed
// reference line at the true value of π, so the "settles down as samples
// grow" convergence is directly visible rather than just a jumping number.
function drawSparkline(ctx: CanvasRenderingContext2D, history: number[], width: number, height: number) {
  ctx.clearRect(0, 0, width, height);
  if (history.length < 2) return;
  const min = 2;
  const max = 4.5;
  const yFor = (v: number) => {
    const clamped = Math.min(max, Math.max(min, v));
    return height - ((clamped - min) / (max - min)) * height;
  };
  ctx.strokeStyle = "#999";
  ctx.setLineDash([3, 3]);
  ctx.lineWidth = 1;
  ctx.beginPath();
  ctx.moveTo(0, yFor(Math.PI));
  ctx.lineTo(width, yFor(Math.PI));
  ctx.stroke();
  ctx.setLineDash([]);

  ctx.strokeStyle = "#6af";
  ctx.lineWidth = 1.5;
  ctx.beginPath();
  ctx.moveTo(0, yFor(history[0]));
  for (let i = 1; i < history.length; i++) {
    ctx.lineTo((i / (history.length - 1)) * width, yFor(history[i]));
  }
  ctx.stroke();
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
  // The naive/cosine/mixture comparison grid's three panels — one canvas
  // per strategy, captured together by showOneSample below.
  const snapshotNaiveRef = useRef<HTMLCanvasElement | null>(null);
  const snapshotCosineRef = useRef<HTMLCanvasElement | null>(null);
  const snapshotMixtureRef = useRef<HTMLCanvasElement | null>(null);
  const sceneRef = useRef<Scene | null>(null);
  // Mirrors `sceneKind` state for the scroll-driven narrative effect below,
  // which subscribes once via [ready] rather than re-subscribing on every
  // scene change — same pattern as maxDepthRef. Also lets that effect skip a
  // redundant load_preset call when the visitor scrolls past a section's
  // threshold more than once without the scene actually changing.
  const sceneKindRef = useRef(0);
  // One ref per narrative section that "owns" a scene — scrolling one of
  // these into view pins the shared background render to the scene that
  // section is discussing (see the IntersectionObserver effect below).
  const materialsSectionRef = useRef<HTMLElement | null>(null);
  const dofSectionRef = useRef<HTMLElement | null>(null);
  const dispersionSectionRef = useRef<HTMLElement | null>(null);
  const lightSamplingSectionRef = useRef<HTMLElement | null>(null);
  // The last traced click's bounce path(s), or null when there's nothing to
  // draw. Lives in a ref (not state) because the render loop redraws it
  // every frame alongside the progressive render, not on its own React
  // render cycle.
  const tracedPathRef = useRef<TracedPathSegment[] | null>(null);
  // The last sunburst click's sample fan, or null — the sunburst view's
  // analogue of tracedPathRef. Only one of the two is ever populated at a
  // time (see clickMode below), and neither persists any per-pixel history:
  // a sunburst click always replaces the previous one outright.
  const sunburstRef = useRef<SunburstSample | null>(null);
  // Every ray ever traced, keyed by the pixel it was sent from — the record
  // the "accumulate" mode below replays. Lives in a ref, same reasoning as
  // tracedPathRef: mutated from the pointer-events effect, read from the
  // render loop, never itself drives a React render.
  const rayHistoryRef = useRef<Map<string, TracedPathSegment[]>>(new Map());
  // Which pixel was last clicked, so toggling the accumulate setting can
  // immediately redraw that pixel's full history without waiting for
  // another click.
  const lastClickedPixelRef = useRef<{ px: number; py: number } | null>(null);
  // Mirrors `accumulateRays` state for the pointer-events effect below (only
  // re-subscribes on [ready]) — same pattern as maxDepthRef. Defaults on: a
  // click's default job is to reveal the saved history at that pixel, not
  // to bury it under a fresh replacement — "cast a new ray" (below) is the
  // opt-in.
  const accumulateRaysRef = useRef(true);
  const [accumulateRays, setAccumulateRays] = useState(true);
  // Mirrors `samplingStrategy` state for the render loop and pointer-events
  // effect (both only re-subscribe on [ready]) — same pattern as
  // maxDepthRef. Drives both the real progressive render (via
  // set_sampling_strategy) and whether drawTracedPath colours bounce
  // segments by PDF source.
  const samplingStrategyRef = useRef(0);
  const [samplingStrategy, setSamplingStrategy] = useState(0);
  // Which visualization a click-without-drag produces: the existing
  // multi-bounce path trace, or the new single-bounce direction sunburst.
  const clickModeRef = useRef<"trace" | "sunburst">("trace");
  const [clickMode, setClickMode] = useState<"trace" | "sunburst">("trace");
  // Which PDF the sunburst view samples from when clickMode is "sunburst".
  const sunburstModeRef = useRef(1);
  const [sunburstMode, setSunburstMode] = useState(1);
  // The internal render resolution. Mirrored into a ref so the rAF loop and
  // the pointer handlers (subscribed once via [ready]) always read the
  // latest size instead of one captured at mount/subscribe time.
  const dimsRef = useRef(computeRenderSize(window.innerWidth, window.innerHeight));
  const [dims, setDims] = useState(dimsRef.current);
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
  // 0-4 select one of PRESETS; 5 (CUSTOM_SCENE_ID) shows the JSON textarea
  // instead of loading a builtin. Drives which fieldset below is visible.
  const [sceneKind, setSceneKind] = useState<number>(0);
  const [dofAngle, setDofAngle] = useState(0);
  const [jsonText, setJsonText] = useState(EXAMPLE_JSON);
  const [jsonError, setJsonError] = useState<string | null>(null);
  // The floating control widget defaults open — every slider/select stays
  // reachable without first discovering a hidden toggle. Collapsing is for
  // someone who wants an unobstructed view of the render, not the default.
  const [widgetOpen, setWidgetOpen] = useState(true);

  // Monte Carlo π estimator demo state (see MC_* constants above). Refs
  // mirror the checkbox state for the rAF loop below, same reasoning as
  // accumulateRaysRef: the loop reads these every frame without
  // re-subscribing to the effect on every toggle.
  const mcCanvasRef = useRef<HTMLCanvasElement | null>(null);
  const mcSparklineRef = useRef<HTMLCanvasElement | null>(null);
  const mcRunningRef = useRef(true);
  const [mcRunning, setMcRunning] = useState(true);
  const mcJitterRef = useRef(false);
  const [mcJitter, setMcJitter] = useState(false);
  const mcInsideRef = useRef(0);
  const mcTotalRef = useRef(0);
  const mcCellRef = useRef(0);
  const mcHistoryRef = useRef<number[]>([]);
  const [mcEstimate, setMcEstimate] = useState<number | null>(null);
  const [mcTotal, setMcTotal] = useState(0);

  // Any traced ray's points and the pixel history keyed by them describe
  // geometry captured at a specific camera/scene/resolution — once any of
  // those change, stale rays would draw at positions that no longer mean
  // anything, so every mutation below clears both the current overlay and
  // the whole per-pixel history rather than just the former.
  const clearTracedRays = () => {
    tracedPathRef.current = null;
    sunburstRef.current = null;
    rayHistoryRef.current.clear();
    lastClickedPixelRef.current = null;
  };

  useEffect(() => {
    let cancelled = false;
    init().then(() => {
      if (cancelled) return;
      sceneRef.current = new Scene(dimsRef.current.width, dimsRef.current.height);
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
      const { width, height } = dimsRef.current;
      const imageData = new ImageData(new Uint8ClampedArray(scene.pixels()), width, height);
      ctx.putImageData(imageData, 0, 0);
      drawTracedPath(ctx, tracedPathRef.current, samplingStrategyRef.current === 2);
      drawSunburst(ctx, sunburstRef.current, sunburstModeRef.current);
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
      if (!dragConfirmed) {
        // The camera's about to move — any previously traced ray (and its
        // whole per-pixel history) now points at geometry that no longer
        // exists. Cleared once, on the frame the drag is first confirmed,
        // rather than on every subsequent move event.
        dragConfirmed = true;
        clearTracedRays();
      }
      scene.orbit_camera((e.clientY - lastY) * 0.005, -(e.clientX - lastX) * 0.005);
      lastX = e.clientX;
      lastY = e.clientY;
    };
    const onUp = (e: PointerEvent) => {
      dragging = false;
      if (dragConfirmed) return;

      const { width, height } = dimsRef.current;
      const rect = canvas.getBoundingClientRect();
      const px = Math.min(width - 1, Math.max(0, Math.round(((e.clientX - rect.left) / rect.width) * width)));
      const py = Math.min(height - 1, Math.max(0, Math.round(((e.clientY - rect.top) / rect.height) * height)));

      if (clickModeRef.current === "sunburst") {
        const raw = Array.from(scene.sample_directions(px, py, sunburstModeRef.current, SUNBURST_SAMPLE_COUNT));
        sunburstRef.current = parseSampleDirections(raw);
        tracedPathRef.current = null;
        return;
      }

      const raw = Array.from(scene.trace_pixel(px, py, maxDepthRef.current));
      const paths = parseTracedPaths(raw);

      lastClickedPixelRef.current = { px, py };
      const key = pixelKey(px, py);
      const history = [...(rayHistoryRef.current.get(key) ?? []), ...paths].slice(-MAX_RAYS_PER_PIXEL);
      rayHistoryRef.current.set(key, history);

      sunburstRef.current = null;
      tracedPathRef.current = accumulateRaysRef.current ? history : paths.length > 0 ? paths : null;
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

  // Debounced so a desktop window drag-resize doesn't reallocate the
  // accumulation buffer on every intermediate pixel — only once resizing
  // settles.
  useEffect(() => {
    let timeout: ReturnType<typeof setTimeout> | undefined;
    const onResize = () => {
      clearTimeout(timeout);
      timeout = setTimeout(() => {
        const next = computeRenderSize(window.innerWidth, window.innerHeight);
        const current = dimsRef.current;
        if (next.width === current.width && next.height === current.height) return;
        dimsRef.current = next;
        // Updated imperatively (not just via the `dims` state below) so the
        // canvas's backing store matches before the render loop's very next
        // frame draws into it, rather than waiting a React commit behind.
        if (canvasRef.current) {
          canvasRef.current.width = next.width;
          canvasRef.current.height = next.height;
        }
        setDims(next);
        sceneRef.current?.resize(next.width, next.height);
        clearTracedRays();
      }, 200);
    };
    window.addEventListener("resize", onResize);
    return () => {
      window.removeEventListener("resize", onResize);
      clearTimeout(timeout);
    };
  }, []);

  // The narrative's whole premise is "the render in the background matches
  // whatever concept you're currently reading about" — this is what makes
  // that true. IntersectionObserver (not scroll-position math) so it works
  // the same scrolling up as scrolling down. Guarded against jsdom, which
  // has no IntersectionObserver global — tests drive scene selection
  // manually via the Playground picker instead, unaffected by this effect
  // simply not running.
  useEffect(() => {
    if (!ready || typeof IntersectionObserver === "undefined") return;
    const sections: [number, HTMLElement | null][] = [
      [0, materialsSectionRef.current],
      [4, dofSectionRef.current],
      [3, dispersionSectionRef.current],
      [1, lightSamplingSectionRef.current],
    ];
    const idsByElement = new Map(sections.filter(([, el]) => el).map(([id, el]) => [el as HTMLElement, id]));

    // rootMargin shrinks the observer's root to a single horizontal line at
    // the viewport's vertical centre, so a section is "intersecting" exactly
    // when it's crossing that centreline — independent of the section's own
    // height. A plain intersectionRatio threshold breaks once a section (with
    // its prose wrapped narrower) grows taller than ~2x the viewport, since
    // the visible fraction of it then never reaches that threshold at all.
    const observer = new IntersectionObserver(
      (entries) => {
        for (const entry of entries) {
          if (!entry.isIntersecting) continue;
          const id = idsByElement.get(entry.target as HTMLElement);
          if (id === undefined || sceneKindRef.current === id) continue;
          sceneKindRef.current = id;
          setSceneKind(id);
          setJsonError(null);
          clearTracedRays();
          sceneRef.current?.load_preset(id);
          setDofAngle(PRESET_DEFAULT_DOF[id] ?? 0);
          if (id === 0) setControls(HERO_SPHERES.map((s) => ({ ...s.initial })));
        }
      },
      { rootMargin: "-50% 0px -50% 0px", threshold: 0 },
    );
    for (const [, el] of sections) {
      if (el) observer.observe(el);
    }
    return () => observer.disconnect();
  }, [ready]);

  // The Monte Carlo canvas lives inside the collapsible control-widget body,
  // which unmounts entirely when the widget is collapsed — so this effect
  // re-subscribes on [widgetOpen] rather than mounting once, picking up a
  // fresh canvas element each time the section reappears instead of drawing
  // into a detached one.
  useEffect(() => {
    if (!widgetOpen) return;
    const canvas = mcCanvasRef.current;
    const ctx = canvas?.getContext("2d");
    const sparkCanvas = mcSparklineRef.current;
    const sparkCtx = sparkCanvas?.getContext("2d");
    if (!canvas || !ctx) return;

    let frame: number;
    let lastHistoryPush = 0;
    const loop = (t: number) => {
      if (mcRunningRef.current) {
        const size = canvas.width;
        for (let k = 0; k < MC_POINTS_PER_TICK; k++) {
          let x: number;
          let y: number;
          if (mcJitterRef.current) {
            const cell = mcCellRef.current % (MC_GRID_SIZE * MC_GRID_SIZE);
            mcCellRef.current += 1;
            x = ((cell % MC_GRID_SIZE) + Math.random()) / MC_GRID_SIZE;
            y = (Math.floor(cell / MC_GRID_SIZE) + Math.random()) / MC_GRID_SIZE;
          } else {
            x = Math.random();
            y = Math.random();
          }
          const inside = x * x + y * y <= 1;
          mcTotalRef.current += 1;
          if (inside) mcInsideRef.current += 1;
          ctx.fillStyle = inside ? "#30c060" : "#ff3b30";
          ctx.beginPath();
          ctx.arc(x * size, (1 - y) * size, 1.5, 0, Math.PI * 2);
          ctx.fill();
        }
        if (t - lastHistoryPush > 150) {
          lastHistoryPush = t;
          const estimate = (4 * mcInsideRef.current) / mcTotalRef.current;
          mcHistoryRef.current = [...mcHistoryRef.current, estimate].slice(-MC_HISTORY_LIMIT);
          setMcEstimate(estimate);
          setMcTotal(mcTotalRef.current);
          if (sparkCanvas && sparkCtx) {
            drawSparkline(sparkCtx, mcHistoryRef.current, sparkCanvas.width, sparkCanvas.height);
          }
        }
      }
      frame = requestAnimationFrame(loop);
    };
    frame = requestAnimationFrame(loop);
    return () => cancelAnimationFrame(frame);
  }, [widgetOpen]);

  const updateMcRunning = (on: boolean) => {
    mcRunningRef.current = on;
    setMcRunning(on);
  };

  // Mixing freely-random and stratified points in the same running average
  // would muddy the very comparison this toggle exists to show, so flipping
  // it restarts the demo from scratch.
  const updateMcJitter = (on: boolean) => {
    mcJitterRef.current = on;
    setMcJitter(on);
    resetMonteCarlo();
  };

  const resetMonteCarlo = () => {
    mcInsideRef.current = 0;
    mcTotalRef.current = 0;
    mcCellRef.current = 0;
    mcHistoryRef.current = [];
    setMcEstimate(null);
    setMcTotal(0);
    const canvas = mcCanvasRef.current;
    const ctx = canvas?.getContext("2d");
    if (canvas && ctx) ctx.clearRect(0, 0, canvas.width, canvas.height);
    const spark = mcSparklineRef.current;
    const sparkCtx = spark?.getContext("2d");
    if (spark && sparkCtx) sparkCtx.clearRect(0, 0, spark.width, spark.height);
  };

  const updateSamplesPerPass = (n: number) => {
    samplesPerPassRef.current = n;
    setSamplesPerPass(n);
  };

  const updateMaxDepth = (n: number) => {
    maxDepthRef.current = n;
    setMaxDepth(n);
    sceneRef.current?.set_max_depth(n);
    clearTracedRays();
  };

  const updateAccumulateRays = (on: boolean) => {
    accumulateRaysRef.current = on;
    setAccumulateRays(on);
    // Switch what's currently displayed immediately, without waiting for
    // another click, if there's a last-clicked pixel to redraw from.
    const last = lastClickedPixelRef.current;
    if (!last) return;
    const history = rayHistoryRef.current.get(pixelKey(last.px, last.py));
    if (!history || history.length === 0) return;
    tracedPathRef.current = on ? history : history.slice(-1);
  };

  const updateSamplingStrategy = (mode: number) => {
    samplingStrategyRef.current = mode;
    setSamplingStrategy(mode);
    sceneRef.current?.set_sampling_strategy(mode);
    clearTracedRays();
  };

  // Switching what a click produces makes any currently-displayed overlay
  // (whichever kind it is) stale, so this clears both rather than picking
  // one — same reasoning as every other render-affecting control.
  const updateClickMode = (mode: "trace" | "sunburst") => {
    clickModeRef.current = mode;
    setClickMode(mode);
    clearTracedRays();
  };

  const updateSunburstMode = (mode: number) => {
    sunburstModeRef.current = mode;
    setSunburstMode(mode);
    sunburstRef.current = null;
  };

  const showOneSample = () => {
    const scene = sceneRef.current;
    if (!scene) return;
    const { width, height } = dimsRef.current;
    // render_snapshot_with_strategy never touches self.sampling_strategy or
    // the live accumulator (unlike set_sampling_strategy) — capturing all
    // three panels here doesn't reset the progressive render in progress.
    const panels: [number, RefObject<HTMLCanvasElement | null>][] = [
      [0, snapshotNaiveRef],
      [1, snapshotCosineRef],
      [2, snapshotMixtureRef],
    ];
    for (const [mode, canvasRef] of panels) {
      const canvas = canvasRef.current;
      const ctx = canvas?.getContext("2d");
      if (!canvas || !ctx) continue;
      const imageData = new ImageData(new Uint8ClampedArray(scene.render_snapshot_with_strategy(mode, 1)), width, height);
      ctx.putImageData(imageData, 0, 0);
    }
  };

  // Without this, the three panels sit on their empty #111 background until
  // a visitor finds and clicks "Show 1 sample" — easy to mistake for broken.
  // Render an initial sample as soon as there's a scene to sample from.
  useEffect(() => {
    if (ready) showOneSample();
  }, [ready]);

  const updateMaterial = (i: number, next: Partial<SphereControlState>) => {
    setControls((prev) => {
      const updated = prev.map((c, idx) => (idx === i ? { ...c, ...next } : c));
      const state = updated[i];
      const [r, g, b] = hexToRgb01(state.colour);
      sceneRef.current?.set_sphere_material(HERO_SPHERES[i].index, state.kind, r, g, b, state.param);
      clearTracedRays();
      return updated;
    });
  };

  const zoom = (delta: number) => {
    sceneRef.current?.zoom(delta);
    clearTracedRays();
  };

  const applyJson = (json: string) => {
    const scene = sceneRef.current;
    if (!scene) return;
    try {
      scene.load_json(json);
      setJsonError(null);
      setDofAngle(0);
      clearTracedRays();
    } catch (err) {
      setJsonError(err instanceof Error ? err.message : String(err));
    }
  };

  const selectScene = (id: number) => {
    sceneKindRef.current = id;
    setSceneKind(id);
    setJsonError(null);
    clearTracedRays();
    if (id === CUSTOM_SCENE_ID) {
      applyJson(jsonText);
      return;
    }
    sceneRef.current?.load_preset(id);
    setDofAngle(PRESET_DEFAULT_DOF[id] ?? 0);
    // The classic scene rebuilds its default spheres on every load_preset —
    // resync the material controls so they don't lie about a prior tweak.
    if (id === 0) setControls(HERO_SPHERES.map((s) => ({ ...s.initial })));
  };

  const loadExampleJson = () => {
    setJsonText(EXAMPLE_JSON);
    applyJson(EXAMPLE_JSON);
  };

  const updateDof = (n: number) => {
    setDofAngle(n);
    sceneRef.current?.set_defocus(n);
    clearTracedRays();
  };

  return (
    <>
      <canvas
        ref={canvasRef}
        width={dims.width}
        height={dims.height}
        className="raytracer-canvas"
        role="img"
        aria-label="Ray-traced scene. Drag to orbit the camera, or click to trace a ray."
      />
      <SampleCountReadout sampleCount={sampleCount} />
      <div className="click-mode-panel">
        <p className="click-mode-panel-title">Click mode</p>
        <label className="render-setting-checkbox">
          <input
            type="checkbox"
            checked={clickMode === "sunburst"}
            onChange={(e) => updateClickMode(e.target.checked ? "sunburst" : "trace")}
          />
          Click shows a direction sample "sunburst" instead of a full bounce path
        </label>
        {clickMode === "sunburst" && (
          <label>
            Sunburst distribution
            <select value={sunburstMode} onChange={(e) => updateSunburstMode(Number(e.target.value))}>
              {SUNBURST_MODES.map((m) => (
                <option key={m.value} value={m.value}>
                  {m.label}
                </option>
              ))}
            </select>
          </label>
        )}
        {clickMode === "trace" && (
          <label className="render-setting-checkbox">
            <input
              type="checkbox"
              checked={!accumulateRays}
              onChange={(e) => updateAccumulateRays(!e.target.checked)}
            />
            Cast a new ray each click
          </label>
        )}
      </div>
      <div className="story">
        <section className="story-hero">
          <h1>One ray at a time</h1>
          <p>
            Everything on this page is the same single-threaded, CPU path tracer, rendering live
            in your browser behind this text. Drag it to orbit, click it to trace exactly one
            ray and watch what happened to it. Scroll down — each section below hands the render
            a new concept to show off, building from "guess randomly and average" up to real
            light sources and importance sampling. Nothing here is pre-rendered.
          </p>
        </section>

        <section id="mc" className="mc-explainer">
          <h2>Monte Carlo π estimator</h2>
          <p>
            Every render on this page is Monte Carlo integration in disguise — averaging random
            samples to approximate an answer you can't (or don't want to) compute exactly.
            Here's that same idea stripped down to its simplest form, no ray tracing involved
            yet: scatter random points into a unit square, and the fraction landing inside the
            inscribed quarter circle approximates the circle's area, so{" "}
            <code>4 × inside / total</code> converges toward π.
          </p>
          <div className="mc-canvas-row">
            <canvas
              ref={mcCanvasRef}
              width={MC_CANVAS_SIZE}
              height={MC_CANVAS_SIZE}
              className="mc-canvas"
              role="img"
              aria-label="Monte Carlo pi estimator: random points in a unit square, coloured by whether they fall inside the quarter circle."
            />
            <div className="mc-side">
              <p className="mc-estimate" data-testid="mc-estimate">
                {mcEstimate === null ? "π ≈ …" : `π ≈ ${mcEstimate.toFixed(4)}`}
                <br />
                <span className="mc-total">{mcTotal.toLocaleString()} points</span>
              </p>
              <canvas
                ref={mcSparklineRef}
                width={140}
                height={48}
                className="mc-sparkline"
                role="img"
                aria-label="History of the running pi estimate, with a dashed reference line at the true value of pi."
              />
            </div>
          </div>
          <div className="mc-controls">
            <label className="render-setting-checkbox">
              <input type="checkbox" checked={mcRunning} onChange={(e) => updateMcRunning(e.target.checked)} />
              Running
            </label>
            <label className="render-setting-checkbox">
              <input type="checkbox" checked={mcJitter} onChange={(e) => updateMcJitter(e.target.checked)} />
              Stratified sampling (jittered grid)
            </label>
            <button type="button" onClick={resetMonteCarlo}>
              Reset
            </button>
          </div>
          <p className="render-setting-blurb">
            Off, points land anywhere in the square — some patches get clumped with several
            points while others sit empty by chance, the way pure random pixel sampling does.
            On, an {MC_GRID_SIZE}×{MC_GRID_SIZE} grid divides the square into cells and each new
            point is placed randomly within the next cell in turn, cycling back to the first
            cell once all of them have one — the same total randomness, but spread out instead
            of clumped, which is why the estimate above settles down faster with it on.
          </p>
        </section>

        <section id="materials" className="story-section" ref={materialsSectionRef}>
          <h2>Trace a ray yourself</h2>
          <p>
            Drag to orbit the camera. Click without dragging to trace a single ray: each dot is
            a bounce, the line is the path it took, and a gold glowing dot means it hit a light.
          </p>
          <p className="render-setting-blurb">
            <strong>Known quirk:</strong> a bounce that reflects back past the camera has its
            final line segment omitted — the dot's colour is still correct.
          </p>
          {sceneKind === 0 && (
          <fieldset className="materials">
            <legend>Materials</legend>
            {HERO_SPHERES.map((sphere, i) => {
              const state = controls[i];
              const activeMaterial = MATERIAL_KINDS.find((k) => k.value === state.kind);
              return (
                <div className="material-control" key={sphere.index}>
                  <label>
                    {sphere.label} material
                    <select value={state.kind} onChange={(e) => updateMaterial(i, { kind: Number(e.target.value) })}>
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
          )}
        </section>

        <section id="dof" className="story-section" ref={dofSectionRef}>
          <h2>A camera is a lens</h2>
          <p>
            Every ray so far has come from a single pinhole, so everything is in perfect focus.
            Real lenses aren't pinholes — they have an aperture, so each ray instead samples a
            point across a small lens disc. Widen it and anything away from the focus distance
            blurs, while whatever sits at the focus distance stays sharp: this row of spheres
            receding into the distance is built to show that off.
          </p>
          <div className="render-setting">
            <label>
              Aperture (depth of field): {dofAngle.toFixed(1)}°
              <input
                type="range"
                min={0}
                max={3}
                step={0.1}
                value={dofAngle}
                onChange={(e) => updateDof(Number(e.target.value))}
              />
            </label>
            <p className="render-setting-blurb">
              Above 0°, each ray samples a point on a lens instead of a single pinhole, so
              anything away from the focus distance blurs while whatever sits at it stays sharp.
            </p>
          </div>
        </section>

        <section id="dispersion" className="story-section" ref={dispersionSectionRef}>
          <h2>Colour depends on wavelength</h2>
          <p>
            Refraction isn't one number — glass bends red, green and blue light by slightly
            different amounts. Click through this prism and each channel is traced and coloured
            separately (red, green, blue), so the split is visible ray by ray instead of only as
            a blur in the final image.
          </p>
        </section>

        <section id="light-sampling" className="story-section" ref={lightSamplingSectionRef}>
          <h2>Aiming at what actually matters</h2>
          <p>
            So far every bounce has picked its next direction more or less randomly and hoped it
            found a light. This Cornell box has a real light source, small and easy to miss — so
            instead of hoping, a mixture strategy also aims some rays straight at it (next-event
            estimation), mixed with proper cosine-weighted sampling. All three strategies below
            converge to the same final image; the difference is how noisy a single sample looks.
          </p>
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
              How many rays each pixel fires before the canvas updates again. This light source
              is small, so single samples are very noisy — cranking this up visibly clears the
              grain.
            </p>
          </div>
          <div className="render-setting">
            <p className="render-setting-blurb">
              By default, every click here saves its bounce path rather than replacing the last
              one — repeated clicks on the same wall grow into a scatter plot of a random
              process, filling in a cosine-shaped fan. Check "Cast a new ray each click" in the
              top-right panel to see only the newest bounce instead.
            </p>
          </div>
          <div className="render-setting">
            <label>
              PDF sampling strategy
              <select value={samplingStrategy} onChange={(e) => updateSamplingStrategy(Number(e.target.value))}>
                {SAMPLING_STRATEGIES.map((s) => (
                  <option key={s.value} value={s.value}>
                    {s.label}
                  </option>
                ))}
              </select>
            </label>
            <p className="render-setting-blurb">
              Naive (a rough approximation of diffuse scattering), cosine (proper importance sampling for a
              matte surface's brightness), or mixture, which also aims some rays straight at the
              light. Under mixture, bounce segments below are coloured by which PDF produced them
              (teal = cosine, gold = light-aimed).
            </p>
          </div>
          <div className="render-setting">
            <p className="render-setting-blurb">
              The click-mode panel in the top-right corner can swap what a click produces: fires{" "}
              {SUNBURST_SAMPLE_COUNT} single-bounce samples from one clicked point and fans them
              out instead of tracing a full path, showing a strategy's shape directly in one
              click. Cosine clusters visibly toward the surface normal; aimed-at-the-light snaps
              every sample straight at the ceiling.
            </p>
          </div>
          <p className="comparison-blurb">
            One sample per pixel is noisy — these three panels compare that noise across
            strategies, against the converged render's {sampleCount} samples per pixel.
          </p>
          <button type="button" onClick={showOneSample}>
            Refresh 1-sample comparison
          </button>
          <div className="snapshot-grid">
            <div className="snapshot-panel">
              <p className="snapshot-label">Naive</p>
              <canvas
                ref={snapshotNaiveRef}
                width={dims.width}
                height={dims.height}
                className="snapshot-canvas"
                role="img"
                aria-label="A single noisy sample per pixel under naive Lambertian sampling, for comparison against the converged render above."
              />
            </div>
            <div className="snapshot-panel">
              <p className="snapshot-label">Cosine</p>
              <canvas
                ref={snapshotCosineRef}
                width={dims.width}
                height={dims.height}
                className="snapshot-canvas"
                role="img"
                aria-label="A single noisy sample per pixel under cosine-weighted importance sampling, for comparison against the converged render above."
              />
            </div>
            <div className="snapshot-panel">
              <p className="snapshot-label">Mixture</p>
              <canvas
                ref={snapshotMixtureRef}
                width={dims.width}
                height={dims.height}
                className="snapshot-canvas"
                role="img"
                aria-label="A single noisy sample per pixel under mixture direct-light sampling, for comparison against the converged render above."
              />
            </div>
          </div>
        </section>

        <main id="playground" className="control-widget">
          <div className="control-widget-header">
            <span className="control-widget-title">Playground — put it all together</span>
            <button
              type="button"
              aria-expanded={widgetOpen}
              aria-controls="control-widget-body"
              onClick={() => setWidgetOpen((v) => !v)}
            >
              {widgetOpen ? "Collapse" : "Expand"}
            </button>
          </div>
          {widgetOpen && (
            <div id="control-widget-body" className="control-widget-body">
              <p>
                Pick any scene, or write your own, and combine every setting above freely — the
                render above is shared across the whole page, so whatever you set here also
                applies if you scroll back up.
              </p>
              <div className="controls" role="group" aria-label="Zoom">
                <button type="button" onClick={() => zoom(-0.5)}>
                  Zoom in
                </button>
                <button type="button" onClick={() => zoom(0.5)}>
                  Zoom out
                </button>
              </div>
              <fieldset className="scene-picker">
                <legend>Scene</legend>
                <label>
                  Choose a scene
                  <select value={sceneKind} onChange={(e) => selectScene(Number(e.target.value))}>
                    {PRESETS.map((p) => (
                      <option key={p.id} value={p.id}>
                        {p.label}
                      </option>
                    ))}
                    <option value={CUSTOM_SCENE_ID}>Custom (JSON)</option>
                  </select>
                </label>
                {sceneKind === CUSTOM_SCENE_ID && (
                  <div className="json-scene">
                    <label>
                      Scene JSON
                      <textarea
                        value={jsonText}
                        onChange={(e) => setJsonText(e.target.value)}
                        rows={10}
                        spellCheck={false}
                      />
                    </label>
                    <div className="json-scene-actions">
                      <button type="button" onClick={() => applyJson(jsonText)}>
                        Apply scene
                      </button>
                      <button type="button" onClick={loadExampleJson}>
                        Load example
                      </button>
                    </div>
                    {jsonError && (
                      <p className="json-error" role="alert" data-testid="json-scene-error">
                        {jsonError}
                      </p>
                    )}
                  </div>
                )}
              </fieldset>
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
                  A path that hasn't hit a light or escaped after this many bounces gets cut off
                  and counted as black — cheap, but it throws away indirect light. It matters
                  most in the <strong>Cornell box</strong> and <strong>Foggy room</strong>, where
                  every wall and the fog itself are lit only by bounced light.
                </p>
              </div>
            </div>
          )}
        </main>
      </div>
    </>
  );
}
