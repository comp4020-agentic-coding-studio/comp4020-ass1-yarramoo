# Process overview

## What I built

An interactive WASM ray tracer for teaching Shirley's *Ray Tracing* series,
built around one mechanic reused rather than duplicated: click a pixel, see
what happened to its ray. That mechanic carries book 1/2 features (materials,
depth of field, chromatic dispersion, custom JSON scenes, a Cornell box and
foggy room) and book 3's actual subject — orthonormal bases, PDFs, and
next-event estimation against real light sources — pointed at the same
click-a-pixel interaction rather than bolted on as a separate demo.

## The moments that mattered

**A precedence bug found by writing the explainer, not by testing the
render.** Writing a UI blurb for the dielectric material meant reading
`reflectance()`'s Schlick approximation line by line against the book's
formula, and `.powi(2)` turned out to bind tighter than the surrounding
division — silently computing `(1-n)/(1+n)^2` instead of `((1-n)/(1+n))^2`.
The render still looked plausible at a glance, which is exactly why it had
gone unnoticed; I only caught it because the blurb forced me to justify the
formula in words, not because a check failed.
[`f6bf55e`](https://github.com/comp4020-agentic-coding-studio/comp4020-ass1-yarramoo/commit/f6bf55e11e6adfc46b796a8daac632423cf466c3)

**Making book 3 backward-compatible by construction, not by branching.**
The obvious way to add PDF-based sampling is an `if` at every call site
checking which mode is active. Instead I gave `Hittable`/`Material` defaulted
trait methods (`pdf_value`/`random` default to "not a light"; `is_specular`/
`scattering_pdf` default to "behave exactly as before") so every existing
material and every light-less scene renders bit-for-bit identical to book 1/2
unless it explicitly opts in — only `Lambertian` and `Quad` override anything.
I didn't just trust the design: I added `cargo test` cases asserting cosine
and mixture sampling converge to the *same* total radiance as the naive
path on scenes with no lights, so "unaffected unless opted in" is a checked
claim, not an assumption.
[`b5ecb00...30181e3`](https://github.com/comp4020-agentic-coding-studio/comp4020-ass1-yarramoo/compare/b5ecb00...30181e3)

**A UX bug diagnosed by reading the code, not guessing.** The user reported
the naive/cosine/mixture comparison panels as "black and empty." Rather than
assume a rendering bug, I traced it to `render_snapshot_with_strategy` only
ever firing on a manual button click — the panels were empty because nothing
had rendered into them yet, not because rendering was broken. The fix was an
auto-render-on-ready effect plus a layout fix (the three panels were laid out
side-by-side in flex-wrap, overflowing the ~260-340px-wide control widget).
Verified by actually opening the page and watching the panels populate,
per the project's own "the rendered page is the truth" rule.
[`98b3ae3`](https://github.com/comp4020-agentic-coding-studio/comp4020-ass1-yarramoo/commit/98b3ae3)

**Fixing the harness, not just the symptom, when a whole feature arc went
uncommitted.** I discovered the entire book 3 arc — ~2400 lines across 19
files — had never been committed. I reconstructed it into three
build-verified commits using a stash-based technique (stage the target
files, stash everything else with `--keep-index`, run the checks against
that partial state, commit, pop), rather than one indiscriminate dump.
That fixed the symptom but not the habit, so afterwards I added a standing
rule to `CLAUDE.md` to commit incrementally as work lands, not at session
end — the skilled fix, per this course's own rubric, is the one that changes
what the agent works against.
[`b5ecb00...98b3ae3`](https://github.com/comp4020-agentic-coding-studio/comp4020-ass1-yarramoo/compare/b5ecb00...98b3ae3),
[`767d518`](https://github.com/comp4020-agentic-coding-studio/comp4020-ass1-yarramoo/commit/767d518)
