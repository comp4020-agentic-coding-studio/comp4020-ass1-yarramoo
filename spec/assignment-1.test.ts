// @vitest-environment jsdom
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { createElement } from "react";
import { describe, expect, it, vi } from "vitest";

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
HTMLCanvasElement.prototype.getContext = vi.fn(() => ({
  putImageData: () => {},
})) as unknown as typeof HTMLCanvasElement.prototype.getContext;
// jsdom doesn't implement the Pointer Capture API either.
HTMLElement.prototype.setPointerCapture = vi.fn();
HTMLElement.prototype.releasePointerCapture = vi.fn();

describe("assignment-1: the interaction", () => {
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
});
