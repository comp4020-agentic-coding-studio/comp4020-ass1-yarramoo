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
    trace_pixel() {
      return new Float64Array([10, 20, 30, 40]);
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
  beginPath: vi.fn(),
  moveTo: vi.fn(),
  lineTo: vi.fn(),
  stroke: vi.fn(),
  arc: vi.fn(),
  fill: vi.fn(),
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

  it("resets the render when a visitor adjusts the max bounce depth", async () => {
    render(createElement(App));
    const status = await screen.findByTestId("sample-count");

    await vi.waitFor(() => expect(status.textContent).not.toBe("Samples per pixel: 0"));
    const before = status.textContent;

    const depthSlider = screen.getByLabelText(/max bounce depth/i);
    fireEvent.change(depthSlider, { target: { value: "3" } });

    await vi.waitFor(() => expect(status.textContent).not.toBe(before), { timeout: 2000 });
  });

  it("draws a 1-sample snapshot when a visitor clicks the comparison button", async () => {
    render(createElement(App));
    await screen.findByTestId("sample-count");

    const snapshotCanvas = screen.getByRole("img", {
      name: /single noisy sample/i,
    }) as HTMLCanvasElement;
    const putImageData = vi.fn();
    vi.spyOn(snapshotCanvas, "getContext").mockReturnValue({
      putImageData,
    } as unknown as CanvasRenderingContext2D);

    const user = userEvent.setup();
    await user.click(screen.getByRole("button", { name: /show 1 sample/i }));

    expect(putImageData).toHaveBeenCalled();
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
});
