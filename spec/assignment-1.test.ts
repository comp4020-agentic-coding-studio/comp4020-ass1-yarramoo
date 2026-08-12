import { describe, expect, it } from "vitest";

// Assignment 1's spec (https://comp.anu.edu.au/courses/comp4020-agentic-coding-studio/assessments/assignment-1/)
// has one line that's mechanically checkable once there's an actual
// interaction to name: "a clear, testable interaction — visitor actions must
// change what's displayed." The rest of the spec is judged at the crit
// (single well-scoped idea, works at both marking viewports) or already
// covered elsewhere (deploy + invariants via CI, process evidence via
// `pnpm check:evidence`).
//
// This starts red on purpose: there's no prototype yet. Replace the selectors
// below with the real interactive control and the real effect it has, once
// the idea is built — that's the point where this test earns its keep.
describe("assignment-1: the interaction", () => {
  it("changes what's displayed when a visitor acts on it", () => {
    expect(
      false,
      "No interaction wired up yet. Point this test at the real control " +
        "(e.g. a button, slider, or input) and assert the DOM it affects " +
        "actually changes in response.",
    ).toBe(true);
  });
});
