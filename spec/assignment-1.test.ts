// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { createElement } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";

// Assignment 1's spec (https://comp.anu.edu.au/courses/comp4020-agentic-coding-studio/assessments/assignment-1/)
// has one line that's mechanically checkable once there's an actual
// interaction to name: "a clear, testable interaction — visitor actions must
// change what's displayed." The interaction is a WASM ray tracer driven from
// a canvas + requestAnimationFrame loop: dragging the canvas orbits the
// camera, which restarts the progressive render. The real wasm build isn't
// exercised here (jsdom can't reliably instantiate it) — this test mocks the
// generated glue module and checks the React/DOM contract: a drag on the
// canvas visibly resets the sample-count readout.
// App.tsx sizes its internal render resolution off the viewport
// (`computeRenderSize`), matching the aspect ratio while keeping roughly the
// same pixel budget as the old fixed 480x300 canvas. Pinning the viewport to
// exactly a 480x300 aspect here reproduces exactly 480x300 (144,000px is the
// budget itself), so every hardcoded buffer/rect size below still matches.
Object.defineProperty(window, "innerWidth", { value: 480, configurable: true });
Object.defineProperty(window, "innerHeight", { value: 300, configurable: true });

vi.mock("../raytracer-wasm/pkg/raytracer_wasm.js", () => {
  class FakeScene {
    private count = 0;
    render_pass(n: number) {
      this.count += n;
    }
    pixels() {
      return new Uint8Array(4);
    }
    orbit_camera() {
      this.count = 0;
    }
    zoom() {
      this.count = 0;
    }
    set_sphere_material() {
      this.count = 0;
    }
    set_max_depth() {
      this.count = 0;
    }
    render_snapshot() {
      // App.tsx's WIDTH*HEIGHT*4 (RGBA) — hardcoded since vi.mock's factory
      // is hoisted above any top-level const declared in this file.
      return new Uint8Array(480 * 300 * 4);
    }
    render_snapshot_with_strategy() {
      return new Uint8Array(480 * 300 * 4);
    }
    // Scene::trace_pixel's real shape is [numPaths, terminationKind,
    // channel, len, x0,y0, ..., s0,s1,...] — one escaped (terminationKind=0),
    // neutral (channel=-1) two-point path, with both vertices tagged as not
    // PDF-sampled (source=-1), matching the plain single-polyline case.
    trace_pixel() {
      return new Float64Array([1, 0, -1, 2, 10, 20, 30, 40, -1, -1]);
    }
    // Scene::sample_directions's real shape is [originX, originY, count,
    // x0,y0, ...] — a two-tip sunburst fanned out from one origin point.
    sample_directions() {
      return new Float64Array([100, 100, 2, 110, 90, 120, 95]);
    }
    set_sampling_strategy() {
      this.count = 0;
    }
    set_defocus() {
      this.count = 0;
    }
    load_preset() {
      this.count = 0;
    }
    resize() {
      this.count = 0;
    }
    load_json(json: string) {
      let parsed: { objects?: unknown[] };
      try {
        parsed = JSON.parse(json);
      } catch {
        throw "invalid JSON";
      }
      if (!parsed.objects || parsed.objects.length === 0) {
        throw "scene must contain at least one object";
      }
      this.count = 0;
    }
    sample_count() {
      return this.count;
    }
  }
  return { default: async () => {}, Scene: FakeScene };
});

const { App } = await import("../App");

// jsdom has no canvas backend (no native "canvas" package): getContext()
// returns null by default, and there's no global ImageData either. Stub
// just enough of both for App.tsx's render loop to run end-to-end.
class FakeImageData {
  constructor(
    public data: Uint8ClampedArray,
    public width: number,
    public height: number,
  ) {}
}
vi.stubGlobal("ImageData", FakeImageData);
// A single shared stub object (rather than a fresh literal per getContext()
// call) so a test can spy on e.g. `ctxStub.arc` and see calls made through
// whichever canvas's context App.tsx actually drew into.
const ctxStub = {
  putImageData: vi.fn(),
  // Stubs for the traced-path overlay App.tsx draws on top of putImageData.
  save: vi.fn(),
  restore: vi.fn(),
  beginPath: vi.fn(),
  moveTo: vi.fn(),
  lineTo: vi.fn(),
  stroke: vi.fn(),
  arc: vi.fn(),
  fill: vi.fn(),
  // Stubs for the Monte Carlo π widget's dot-plot and sparkline canvases.
  clearRect: vi.fn(),
  setLineDash: vi.fn(),
};
HTMLCanvasElement.prototype.getContext = vi.fn(() => ctxStub) as unknown as typeof HTMLCanvasElement.prototype.getContext;
// jsdom doesn't implement the Pointer Capture API either.
HTMLElement.prototype.setPointerCapture = vi.fn();
HTMLElement.prototype.releasePointerCapture = vi.fn();
// jsdom's default getBoundingClientRect is all zeros, which would divide by
// zero when App.tsx converts a click's clientX/Y into pixel coordinates.
// App.tsx's WIDTH/HEIGHT (480x300) hardcoded here for the same reason the
// vi.mock factory above hardcodes its buffer size.
HTMLCanvasElement.prototype.getBoundingClientRect = vi.fn(
  () =>
    ({
      x: 0,
      y: 0,
      left: 0,
      top: 0,
      right: 480,
      bottom: 300,
      width: 480,
      height: 300,
      toJSON: () => {},
    }) as DOMRect,
);

describe("assignment-1: the interaction", () => {
  // jsdom + this test file don't get testing-library's automatic afterEach
  // cleanup (that requires vitest's `globals: true`, which this repo
  // doesn't set) — without it, a second test's render() leaves the first
  // test's <App/> (and its running rAF loop) mounted alongside the new one.
  afterEach(() => cleanup());

  it("resets the render when a visitor drags to orbit the camera", async () => {
    render(createElement(App));
    const status = await screen.findByTestId("sample-count");

    await vi.waitFor(() => expect(status.textContent).not.toBe("Samples per pixel: 0"));
    const before = status.textContent;

    const canvas = screen.getByRole("img", { name: /ray-traced scene/i });
    const user = userEvent.setup();
    await user.pointer([
      { keys: "[MouseLeft>]", target: canvas, coords: { x: 100, y: 100 } },
      { coords: { x: 150, y: 80 } },
      { keys: "[/MouseLeft]" },
    ]);

    await vi.waitFor(() => expect(status.textContent).not.toBe(before), { timeout: 2000 });
  });

  it("traces a ray's bounce path when a visitor clicks without dragging", async () => {
    render(createElement(App));
    await screen.findByTestId("sample-count");

    const canvas = screen.getByRole("img", { name: /drag to orbit the camera, or click/i });
    const user = userEvent.setup();
    await user.pointer([
      { keys: "[MouseLeft>]", target: canvas, coords: { x: 100, y: 100 } },
      { keys: "[/MouseLeft]" },
    ]);

    await vi.waitFor(() => expect(ctxStub.arc).toHaveBeenCalled());
  });

  it("shows the accumulate-rays toggle, defaulting to off", async () => {
    render(createElement(App));
    await screen.findByTestId("sample-count");

    const toggle = screen.getByLabelText(/accumulate every ray/i) as HTMLInputElement;
    expect(toggle.checked).toBe(false);
  });

  it("keeps tracing correctly across repeated clicks with accumulate mode on", async () => {
    render(createElement(App));
    await screen.findByTestId("sample-count");

    const user = userEvent.setup();
    await user.click(screen.getByLabelText(/accumulate every ray/i));

    const canvas = screen.getByRole("img", { name: /drag to orbit the camera, or click/i });
    for (let i = 0; i < 2; i++) {
      await user.pointer([
        { keys: "[MouseLeft>]", target: canvas, coords: { x: 100, y: 100 } },
        { keys: "[/MouseLeft]" },
      ]);
    }

    await vi.waitFor(() => expect(ctxStub.arc).toHaveBeenCalled());
  });

  it("resets the render when a visitor adjusts the max bounce depth", async () => {
    render(createElement(App));
    const status = await screen.findByTestId("sample-count");

    await vi.waitFor(() => expect(status.textContent).not.toBe("Samples per pixel: 0"));
    const before = status.textContent;

    const depthSlider = screen.getByLabelText(/max bounce depth/i);
    fireEvent.change(depthSlider, { target: { value: "3" } });

    await vi.waitFor(() => expect(status.textContent).not.toBe(before), { timeout: 2000 });
  });

  it("draws a 1-sample snapshot into all three strategy panels when a visitor clicks the comparison button", async () => {
    render(createElement(App));
    await screen.findByTestId("sample-count");

    // Three panels (naive / cosine / mixture), each its own canvas — all
    // match "single noisy sample" via their shared aria-label suffix. Every
    // canvas in this file shares the one prototype-level getContext mock
    // (returning the shared ctxStub), so rather than spy per-canvas — which
    // would mean nesting three vi.spyOn wraps on the same prototype method
    // and restoring them out of stack order would leave a stale mock behind
    // for every later test — just count calls on the shared stub.
    const snapshotCanvases = screen.getAllByRole("img", {
      name: /single noisy sample/i,
    }) as HTMLCanvasElement[];
    expect(snapshotCanvases).toHaveLength(3);

    // The main render loop's own requestAnimationFrame calls also draw into
    // this shared ctxStub every frame, so >= before + 3 (rather than an
    // exact count) is what's actually guaranteed by the click.
    const before = ctxStub.putImageData.mock.calls.length;
    const user = userEvent.setup();
    await user.click(screen.getByRole("button", { name: /refresh 1-sample comparison/i }));

    await vi.waitFor(() => expect(ctxStub.putImageData.mock.calls.length).toBeGreaterThanOrEqual(before + 3));
  });

  it("resets the render when a visitor changes the PDF sampling strategy", async () => {
    render(createElement(App));
    const status = await screen.findByTestId("sample-count");

    await vi.waitFor(() => expect(status.textContent).not.toBe("Samples per pixel: 0"));
    const before = status.textContent;

    const select = screen.getByLabelText(/pdf sampling strategy/i);
    fireEvent.change(select, { target: { value: "2" } }); // Mixture

    await vi.waitFor(() => expect(status.textContent).not.toBe(before), { timeout: 2000 });
  });

  it("only shows the sunburst distribution selector once the sunburst click mode is on", async () => {
    render(createElement(App));
    await screen.findByTestId("sample-count");

    expect(screen.queryByLabelText(/sunburst distribution/i)).toBeNull();

    const user = userEvent.setup();
    await user.click(screen.getByLabelText(/direction sample.*sunburst/i));

    expect(screen.getByLabelText(/sunburst distribution/i)).toBeTruthy();
  });

  it("draws a sunburst instead of a bounce path when a visitor clicks in sunburst mode", async () => {
    render(createElement(App));
    await screen.findByTestId("sample-count");

    const user = userEvent.setup();
    await user.click(screen.getByLabelText(/direction sample.*sunburst/i));

    const canvas = screen.getByRole("img", { name: /drag to orbit the camera, or click/i });
    await user.pointer([
      { keys: "[MouseLeft>]", target: canvas, coords: { x: 100, y: 100 } },
      { keys: "[/MouseLeft]" },
    ]);

    // drawSunburst strokes a fan of lines and a distinguishing origin dot —
    // the same primitives drawTracedPath uses, so this only confirms
    // something was drawn (the click was routed to sample_directions rather
    // than trace_pixel without throwing); FakeScene's stub shapes are what
    // actually exercise the two call sites differently.
    await vi.waitFor(() => expect(ctxStub.stroke).toHaveBeenCalled());
  });

  it("shows the material blurb matching the selected material kind", async () => {
    render(createElement(App));
    await screen.findByTestId("sample-count");

    const blurb = await screen.findByTestId("material-blurb-1");
    expect(blurb.textContent).toMatch(/diffuse scattering/i);

    const select = screen.getAllByLabelText(/left sphere material/i)[0];
    fireEvent.change(select, { target: { value: "1" } });

    expect((await screen.findByTestId("material-blurb-1")).textContent).toMatch(/specular reflection/i);
  });

  it("shows the click-to-trace explainer section inside the open control widget", async () => {
    render(createElement(App));
    await screen.findByTestId("sample-count");

    expect(screen.getByRole("heading", { name: /trace a ray yourself/i }).textContent).toMatch(
      /trace a ray yourself/i,
    );
  });

  it("collapses and re-expands the control widget", async () => {
    render(createElement(App));
    await screen.findByTestId("sample-count");

    expect(screen.getByLabelText(/max bounce depth/i)).toBeTruthy();

    const user = userEvent.setup();
    await user.click(screen.getByRole("button", { name: /collapse/i }));
    expect(screen.queryByLabelText(/max bounce depth/i)).toBeNull();

    await user.click(screen.getByRole("button", { name: /expand/i }));
    expect(screen.getByLabelText(/max bounce depth/i)).toBeTruthy();
  });

  it("hides the classic-scene material controls when a visitor picks a different scene", async () => {
    render(createElement(App));
    await screen.findByTestId("sample-count");

    expect(screen.getByText("Materials")).toBeTruthy();

    const sceneSelect = screen.getByLabelText(/choose a scene/i);
    fireEvent.change(sceneSelect, { target: { value: "1" } }); // Cornell box

    expect(screen.queryByText("Materials")).toBeNull();
  });

  it("loads the example custom scene with no error", async () => {
    render(createElement(App));
    await screen.findByTestId("sample-count");

    fireEvent.change(screen.getByLabelText(/choose a scene/i), { target: { value: "5" } }); // Custom (JSON)
    expect(await screen.findByLabelText(/scene json/i)).toBeTruthy();
    expect(screen.queryByTestId("json-scene-error")).toBeNull();

    const user = userEvent.setup();
    await user.click(screen.getByRole("button", { name: /load example/i }));

    expect(screen.queryByTestId("json-scene-error")).toBeNull();
  });

  it("runs the Monte Carlo pi estimator and updates its running estimate", async () => {
    render(createElement(App));
    await screen.findByTestId("sample-count");

    const estimate = await screen.findByTestId("mc-estimate");
    expect(estimate.textContent).toMatch(/π ≈ …/);

    await vi.waitFor(() => expect(estimate.textContent).toMatch(/π ≈ \d/), { timeout: 2000 });
  });

  it("resets the Monte Carlo estimate when a visitor clicks Reset", async () => {
    render(createElement(App));
    await screen.findByTestId("sample-count");

    const estimate = await screen.findByTestId("mc-estimate");
    await vi.waitFor(() => expect(estimate.textContent).toMatch(/π ≈ \d/), { timeout: 2000 });

    const user = userEvent.setup();
    await user.click(screen.getByRole("button", { name: /^reset$/i }));

    expect(estimate.textContent).toMatch(/π ≈ …/);
  });

  it("restarts the Monte Carlo estimate when a visitor toggles stratified sampling", async () => {
    render(createElement(App));
    await screen.findByTestId("sample-count");

    const estimate = await screen.findByTestId("mc-estimate");
    await vi.waitFor(() => expect(estimate.textContent).toMatch(/π ≈ \d/), { timeout: 2000 });

    const user = userEvent.setup();
    await user.click(screen.getByLabelText(/stratified sampling/i));

    expect(estimate.textContent).toMatch(/π ≈ …/);
  });

  it("shows an error message when the custom scene JSON is invalid", async () => {
    render(createElement(App));
    await screen.findByTestId("sample-count");

    fireEvent.change(screen.getByLabelText(/choose a scene/i), { target: { value: "5" } }); // Custom (JSON)
    const textarea = await screen.findByLabelText(/scene json/i);
    fireEvent.change(textarea, { target: { value: "{ not valid json" } });

    const user = userEvent.setup();
    await user.click(screen.getByRole("button", { name: /apply scene/i }));

    expect(await screen.findByTestId("json-scene-error")).toBeTruthy();
  });
});
