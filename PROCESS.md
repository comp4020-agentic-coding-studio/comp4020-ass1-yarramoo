# Process overview

## What I built

Last year I implemented Shirley's [*Ray Tracing*](https://raytracing.github.io/)
series, going through in Rust rather than the usual C++. I always wanted to
build a wrapper around the project to play with scenes interactively. This
project does that and focuses learning about ray tracing through one
mechanic: click a pixel, see
what happened to its ray. That mechanic carries book 1/2 features (materials,
depth of field, chromatic dispersion, custom JSON scenes, a Cornell box and
foggy room) and book 3's actual subject — orthonormal bases, PDFs, and
next-event estimation against real light sources.

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

**A phone bug invisible from the desktop view I'd been testing in.** The
scrollytelling redesign narrowed the narrative column so the render stayed
visible beside it on desktop — but the phone-width fallback widens that same
column back to full-bleed for legibility, and I hadn't noticed its CSS grid
box (gaps between cards included) sat in front of the canvas the whole
scroll, at every width. On desktop the narrow column left most of the
viewport clickable regardless, so the render was still reachable and the bug
stayed silent; on phone, the full-width column covered every pixel of the
scrollable page, so the click-to-trace/drag-to-orbit interaction — this
project's whole reason for existing — was completely unreachable there.
Caught by scripting `elementFromPoint` across the full scroll range at
390×844 rather than trusting a few screenshots, and fixed by letting
`.story`'s own box pass pointer events through, catching them again only on
the actual card children.
[`447f088`](https://github.com/comp4020-agentic-coding-studio/comp4020-ass1-yarramoo/commit/447f088)

**Fixing the harness, not just the symptom, when a whole feature arc went
uncommitted.** When adding in book 3 features, the agent implemented ~2400 lines
of code without committing any changes. I asked it to reconstruct some seperate 
commits, but importantly added a note to `CLAUDE.md` to always commit incrementally. 
[`b5ecb00...98b3ae3`](https://github.com/comp4020-agentic-coding-studio/comp4020-ass1-yarramoo/compare/b5ecb00...98b3ae3),
[`767d518`](https://github.com/comp4020-agentic-coding-studio/comp4020-ass1-yarramoo/commit/767d518)
