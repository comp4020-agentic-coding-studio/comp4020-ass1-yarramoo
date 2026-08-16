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
render.** Writing a UI blurb for the dielectric material meant checking
`reflectance()`'s Schlick approximation against the book's formula line by
line — `.powi(2)` bound tighter than the surrounding division, silently
computing `(1-n)/(1+n)^2` instead of `((1-n)/(1+n))^2`. The render still
looked plausible at a glance, which is why it had gone unnoticed; only
justifying the formula in words caught it, not a passing check.
[`15c350f`](https://github.com/comp4020-agentic-coding-studio/comp4020-ass1-yarramoo/commit/15c350fe7f3a7e3df3bd36f5fc8eb62cff8c5e5d)

**Making book 3 backward-compatible by construction, not by branching.**
Rather than an `if` at every call site for which sampling mode is active, I
gave `Hittable`/`Material` defaulted trait methods (`pdf_value`/`random`
default to "not a light"; `is_specular`/`scattering_pdf` default to "behave
exactly as before"), so every existing material and light-less scene renders
bit-for-bit identical to book 1/2 unless it explicitly opts in — only
`Lambertian` and `Quad` override anything. Verified with `cargo test` cases
asserting cosine and mixture sampling converge to the same radiance as the
naive path on light-less scenes, not just assumed.
[`b5ecb00...30181e3`](https://github.com/comp4020-agentic-coding-studio/comp4020-ass1-yarramoo/compare/b5ecb00...30181e3)

**A phone bug invisible from the desktop view I'd been testing in.** The
scrollytelling redesign narrowed the narrative column so the render stayed
visible beside it on desktop — but the phone-width fallback widens that same
column back to full-bleed, and its CSS grid box (gaps included) sat in front
of the canvas the whole scroll. On desktop the narrow column left most of
the viewport clickable, so the bug stayed silent; on phone, the full-width
column covered every pixel, so click-to-trace/drag-to-orbit — this project's
whole reason for existing — was unreachable. Caught by scripting
`elementFromPoint` across the full scroll range at 390×844 rather than
trusting a few screenshots, fixed by letting `.story`'s own box pass pointer
events through, catching them again only on the card children.
[`447f088`](https://github.com/comp4020-agentic-coding-studio/comp4020-ass1-yarramoo/commit/447f088)

**Fixing the harness, not just the symptom, when a whole feature arc went
uncommitted.** When adding in book 3 features, the agent implemented ~2400 lines
of code without committing any changes. I asked it to reconstruct some seperate 
commits, but importantly added a note to `CLAUDE.md` to always commit incrementally. 
[`b5ecb00...98b3ae3`](https://github.com/comp4020-agentic-coding-studio/comp4020-ass1-yarramoo/compare/b5ecb00...98b3ae3),
[`767d518`](https://github.com/comp4020-agentic-coding-studio/comp4020-ass1-yarramoo/commit/767d518)
