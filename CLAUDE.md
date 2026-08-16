# COMP4020 prototype

## This week: React, and a known red

Switched the template's stack to React + Vite (`@vitejs/plugin-react`) for
Assignment 1. `main.tsx` mounts `<App />` into `#root` in `index.html`.

Known gap, left red on purpose: `spec/invariants.test.ts`'s "has a navigation
landmark" and "has exactly one top-level heading" checks read the **built**
`dist/index.html` as static markup — no JS runs. A pure client-rendered SPA
ships an empty shell (`<div id="root"></div>`) before hydration, so those two
invariants fail even though the rendered page (post-JS) has both. Two ways to
close this, not yet chosen: (a) move `<nav>`/`<h1>` into the static HTML shell
and have React mount only the interactive region, or (b) pre-render/SSG the
shell. Fix before shipping — a red invariant at the crit sweep costs marks.

`spec/assignment-1.test.ts` now wires the "testable interaction" line of the
brief to the real control: dragging the ray-tracer canvas orbits the camera
and resets the progressive render. See "Interactive ray tracer: wasm
architecture" below for how that's built.

This is your starter repo for a COMP4020 prototype: a static site written in
HTML/CSS/TypeScript that builds to plain HTML/CSS/JS and deploys to GitHub
Pages. The **deployed site is what gets marked** --- not this repo, and not "it
works on my machine". It's marked live in Chrome against the deployed URL at two
viewports --- 1920×1080 (desktop) and 390×844 (phone) --- and both count in
full, so make that artefact good at both and use the checks below to know
whether it is.

What you're building this week — the spec — is published on the course website,
and this repo's name tells you which deliverable it is. Run the course plugin's
**start** skill at the start of each week: it pulls the right spec from the
course API, carries your harness forward from last week, and helps you turn the
spec's checkable lines into tests of your own. Read the spec before you build,
and see `spec/README.md` for how the checks in this repo relate to it.

## Interactive ray tracer: wasm architecture

The interaction is a Rust ray tracer (vendored from
`ray_tracing_in_one_weekend`) compiled to WebAssembly and driven from a
canvas. Two Rust crates in a Cargo workspace at the repo root:

- `raytracer/` — the vendored tracer, mostly untouched. Its native CLI
  (`cargo run -p raytracer`) still works; that's the check that the vendoring
  didn't break anything.
- `raytracer-wasm/` — a thin `wasm-bindgen` binding crate exposing a `Scene`
  class: `render_pass(n)` traces `n` more samples per pixel into a running
  accumulator, `pixels()` returns gamma-corrected RGBA bytes, and
  `orbit_camera`/`zoom`/`set_sphere_material`/`set_max_depth`/`set_defocus`/
  `load_preset`/`load_json`/`resize` mutate state and reset the accumulator so
  refinement restarts. All wasm/JS-facing dependencies (`wasm-bindgen`,
  `getrandom`'s `wasm_js` backend, `console_error_panic_hook`, `serde`/
  `serde_json` for custom-scene JSON) live here, not in `raytracer`.

**One mechanic, deepened rather than duplicated.** The whole interaction is
"click a pixel, see what happened to its ray" — `trace_pixel(i, j,
max_depth)` returns every bounce path for that click as one flat buffer,
`[numPaths, terminationKind, channel, len, x0,y0, ..., s0,s1,...]` per path
(`terminationKind`: escaped/absorbed/emitted; `channel`: -1 neutral or 0/1/2
for a dispersive-glass split; the trailing `s0,s1,...` tags each vertex with
which PDF produced the bounce into it: -1 not-PDF-sampled/specular, 0
cosine-weighted, 1 aimed at a light). Everything added after the classic
3-sphere scene — `load_preset`'s four extra scenes (Cornell box, foggy room,
dispersive prism, depth of field), `load_json`'s custom-scene textarea, and
`set_defocus`'s lens-bundle sampling — is more things to point that one
mechanic at, not a second feature. `raytracer-wasm/src/presets.rs` and
`scene_json.rs` both build a `SceneSetup` (world + camera framing) that
`Scene::apply_setup` swaps in, so orbit/zoom/reset behave identically
regardless of which scene is loaded.

**Book 3's PDF machinery ("Ray Tracing: The Rest of Your Life"), same
mechanic pointed at a new question.** `raytracer/src/onb.rs` and `pdf.rs`
add an orthonormal-basis type and a `Pdf` trait (`CosinePdf`,
`UniformHemispherePdf`, `HittablePdf` for next-event estimation against a
scene's lights, `MixturePdf` combining the two). `Scene::set_sampling_strategy(mode)`
(0 naive/book-1, 1 cosine-importance, 2 mixture/NEE) switches which PDF
`Camera::ray_colour` and `trace_pixel` sample from for non-specular
materials — mode 0 reproduces the original book-1 approximation exactly, so
every scene's converged image is unaffected by which mode is selected, only
its noise at a given sample count. Two more read-only wasm entry points ride
alongside `trace_pixel`: `sample_directions(i, j, mode, n)` fires `n`
single-bounce sample directions from one clicked point (a "sunburst" —
separate from `trace_pixel` since it has no recursion/termination
bookkeeping), and `render_snapshot_with_strategy(mode, samples)` renders a
stateless snapshot under an explicit strategy without touching
`self.sampling_strategy` or the live accumulator (unlike
`set_sampling_strategy`, which resets accumulation as a side effect) — this
is what lets the frontend's naive/cosine/mixture 3-panel comparison grid
capture all three at once without disturbing the progressive render in
progress. `App.tsx` colour-codes bounce-path segments by PDF source when the
strategy is mixture, and has a click-mode toggle switching a click between
tracing a full path and firing a sunburst.

**Accumulate mode is client-side, not another wasm call.** `trace_pixel`
itself is unchanged and still ephemeral — `App.tsx` is what keeps state now:
every returned path is appended into `rayHistoryRef`, a `Map<pixelKey,
TracedPathSegment[]>` capped at `MAX_RAYS_PER_PIXEL` per key. The "accumulate"
toggle only changes what gets *drawn* — the newest click's paths, or the
pixel's whole recorded history — so clicking the same pixel repeatedly with
it on visibly builds up the same set of independent samples the converged
render's colour for that pixel is actually the average of. Anything that
invalidates previously traced geometry (camera orbit, zoom, material/scene/
JSON change, resize) must clear the whole history, not just the current
overlay — that's `clearTracedRays()`, called everywhere `tracedPathRef` used
to be nulled directly.

**Scrollytelling over one fixed canvas, fixed pixel budget.** The canvas is a
fixed full-viewport background layer and a slim topbar holds nav/heading/
sample-count, both unscrolled — but `.story` (`App.tsx`) is a normal-flow
column of narrative `<section>`s the visitor scrolls past, each one loading a
different preset via an `IntersectionObserver` keyed to the viewport's
vertical centreline (`rootMargin: "-50% 0px -50% 0px", threshold: 0`, checking
only `entry.isIntersecting` — robust to a section being taller than the
viewport, which a naive `intersectionRatio` threshold isn't). The last
section, `.control-widget` (`#playground`), holds every manual control
(scene, materials, render settings, JSON, folded-in explainer/comparison) and
is the one point the whole page's earlier per-concept presets funnel into —
"now drive it yourself." Since the render sits *behind* readable text for the
whole scroll, `.story`'s own grid box is `pointer-events: none` with
`pointer-events: auto` restored only on the card children, so a tap in the
gap between cards still reaches the canvas underneath instead of the empty
grid area swallowing it. Below ~700px there's no spare width to keep text and
canvas side by side, so the narrative falls back to a centred, near-opaque
column — legibility over render-visibility once the two can't coexist. Since
this renderer is single-threaded CPU Monte Carlo, `computeRenderSize` keeps
the *internal* render resolution at roughly the old fixed 480×300 pixel count
and only tracks the viewport's *aspect ratio* on resize (via `Scene::resize`,
debounced ~200ms) — this keeps the convergence rate constant regardless of
window size; the CSS stretch to fill the screen is uniform, not distorting,
since the aspect always matches.

**Single-threaded, deliberately.** Multithreaded wasm needs
`SharedArrayBuffer`, which needs COOP/COEP response headers — GitHub Pages
can't set those. Progressive per-frame refinement (watch the image sharpen)
is the interaction's "wow", not raw speed.

**`pnpm build:wasm` is auto-chained** into `dev`, `build`, and `typecheck`
(see `scripts/build-wasm.sh`) so every entry point is self-sufficient on a
fresh checkout — critically, the deploy job's bare `pnpm build`. It's
incremental, so the redundant rebuild inside `pnpm check` costs seconds.

**wasm-bindgen version pin, the gotcha that bites silently:** the
`wasm-bindgen` crate version (`raytracer-wasm/Cargo.toml`) and the
`wasm-bindgen-cli` version (installed locally, and in
`.github/workflows/checks.yml`) must match *exactly*. A mismatch doesn't fail
the build — it fails at runtime in the browser with an opaque
schema-version-mismatch error. Currently pinned to `0.2.127` in both places;
if you ever bump the crate version, update the CLI install and the CI step
together.

## How to work in here

- Keep the dev server running (`pnpm dev`) so you see changes as you make them.
- Before you push, run `pnpm check`. It runs most of what CI runs --- build,
  lint, and the spec --- so you catch those in seconds instead of waiting for
  the pipeline. The links check, the evidence check, the secrets scan, and the
  deploy itself only run in CI; run `pnpm dlx linkinator ./dist --silent`
  locally against a fresh `pnpm build` for the links check without waiting for
  CI.
- To see what the page actually looks like rather than what you assume it looks
  like, open it in a browser (the `agent-browser` CLI, documented on
  [the course site](https://comp.anu.edu.au/courses/comp4020-agentic-coding-studio/topics/backpressure/#agent-browser-the-rendered-page-as-ground-truth),
  works well for this). The rendered page is the truth; your mental model of it
  isn't.
- When a check fails, read its output before changing anything. Each check below
  names what it measures, and the failure message is the instruction: it tells
  you the file, the line, or the contract. Treat a red check as authoritative
  --- the page is wrong until the check is green, not until you decide it should
  be.
- Commit when the checks pass. Never commit a red state.
- Commit sensibly and incrementally *as you go*, not as one dump at the end of
  a session. Once a logically-scoped piece of work is green (a module, a
  wired-up feature, a UI pass), commit it with a message explaining why before
  moving to the next piece — don't let unrelated work pile up uncommitted in
  between. This isn't hypothetical: an entire feature arc (book 3's PDF-
  sampling machinery — `onb.rs`, `pdf.rs`, the mixture-sampling wiring, the
  preset scenes that depend on it, and the control-widget UI for all of it)
  sat uncommitted across ~2400 lines and 19 files for the whole session it was
  built in, and had to be reconstructed into three retroactive commits
  afterwards by re-deriving which files belonged together and re-running the
  checks against partial state. That reconstruction cost real effort and
  still lost the true chronology — the commit history reads as three
  checkpoints, not as the session that produced them. Commit as each piece
  lands instead.

## The checks (your sensors)

CI runs these on every push once your repo is public. GitHub's checks UI shows
two jobs, `check` and `deploy` --- not one status per sensor below --- and
within `check` the steps run in sequence (`pnpm check` chains typecheck, build,
lint, and the spec with `&&`), so an early failure like a broken build stops the
later sensors from running for that push; fix it and push again to see the rest.
While the repo is private (all week, until you ship) the CI jobs stay skipped
--- `pnpm check` is the same roster on your machine, and it's the faster loop
anyway. They aren't hoops. Each is a different way of finding out something true
about the site that you can't reliably see by looking at it.

They also carry a mark at a crit: the sweep runs fifteen minutes after your
cutoff, and green checks there are worth half that week's shipped mark. Still
running counts as not green, so ship with time for CI to finish.

- **typecheck** --- `tsc --noEmit` runs first in `pnpm check`, so a type error
  stops the roster before the build even starts. The types are extra
  backpressure: a red here is the compiler telling you a claim in the code is
  false.
- **build** --- the site must build (`pnpm build`). A build failure means the
  deployed site is broken or stale, so nothing else matters until this is green.
- **deploy / online** --- the live GitHub Pages URL must load and return the
  page you expect. An asset that 404s on the deployed URL counts as broken even
  if it loads locally.
- **spec** --- `spec/invariants.test.ts` asserts what's true of any good
  website, whatever the week's brief asks; the tests you write for the week's
  own spec run alongside it (any `spec/*.test.ts`). A failure names the contract
  you haven't met yet.
- **lint** --- `stylelint` for CSS, `oxlint` for TypeScript. Flags code that's
  wrong, fragile, or non-idiomatic. Read the rule it names.
- **tests** --- any other tests you write, wherever you put them (co-located
  with your source is fine, not just `spec/`), must pass. Vitest picks up both
  this and the spec suite in one `vitest run`, the last step of `pnpm check`. A
  failing test is a claim about the site that's no longer true.
- **evidence** (`pnpm check:evidence`) --- checks your process evidence:
  `PROCESS.md`'s citations resolve to real commits, the current deliverable's
  exact reflection is in `reflections/` (worked out from this repo's name
  against the public course API), and your `CLAUDE.md` is present. Evidence
  gates the deploy --- `deploy` needs `check` to pass, so failing evidence
  blocks the deploy alongside everything else. See
  [Your process is part of the mark](#your-process-is-part-of-the-mark) below,
  and the course website's
  [assessment page](https://comp.anu.edu.au/courses/comp4020-agentic-coding-studio/topics/assessment/#what-you-submit)
  for what counts as evidence.
- **links** --- internal links must resolve. A broken link is a dead end you
  didn't mean to ship.
- **secrets** --- the repo is scanned for committed credentials. Never put a
  key, token, or password in a tracked file. If one leaks, rotate it. A local
  pre-commit hook (`.githooks/pre-commit`, installed by `pnpm install`) also
  blocks any commit containing something shaped like an API key --- by the time
  CI sees a key it's already pushed, so the hook is the sensor that matters.

Nothing here measures **accessibility** or **performance** --- wiring those
sensors (`axe-core`, Lighthouse, or whatever you choose) is your work, and later
in the course the spec will ask you to show how you tested both. When you do,
read a green performance result honestly: it's a lab estimate from one run on a
CI machine, not proof the site is fast for real users.

## The stack is swappable

Out of the box this is plain HTML/CSS/TypeScript on Vite, and every `.html` file
in the repo is a page: add pages, link them, and the build picks them up with no
config. That's a default, not a rule (unless the week's spec says otherwise).
You can swap in Astro or any other static generator, because nothing in CI names
a tool --- the whole contract is:

- `pnpm build` emits the complete site into `dist/`
- the `package.json` scripts (`check`, `check:evidence`, `build`) keep working
- whatever lands in `dist/` still passes the invariants in `spec/`

Two things bite in a swap. The deployed site lives under a path
(`…github.io/<repo>/`), so configure your generator's base path --- this
template's Vite config uses relative asset URLs to sidestep that, but most
generators (Astro included) need `base` set explicitly, and getting it wrong
looks fine locally while every asset 404s on the live URL. And commit the
updated `pnpm-lock.yaml`: CI installs with `--frozen-lockfile`.

## Your process is part of the mark

The deployed page is only half of it. How you got there is marked too: your
commit history, your agent files, and the decisions visible across them. The
checks above can't see any of that, so a person reads it directly --- which
means building legibly is part of building well.

- **Commit as you go.** Small, frequent commits are the record of how the work
  came together, and that record is read, not just the final state. A trail that
  grew alongside the code is the strongest evidence of your process; a single
  dump the night before is the weakest.
- **Keep a process overview** (`PROCESS.md`). A short reading-guide, not an
  essay: what you built, the moments that mattered --- each pointing at a
  commit, a `CLAUDE.md` change, or a prompt and the commit it produced --- and
  where to look in the history. It points a marker at the evidence; it doesn't
  stand in for it, and claims the history doesn't back don't count. The
  `PROCESS.md` in this repo is a template showing the shape and the citation
  format (link text the commit hash or range, target the commit or compare URL);
  `pnpm check:evidence` verifies your citations resolve to real commits before
  you ship. Markers follow those citations and don't trawl the repo for evidence
  you didn't cite.
- **Write your reflection in `reflections/`** --- a short markdown file in this
  repo, named for the deliverable it answers, so the number in the filename is
  the number in this repo's name (`crit-1.md` in `comp4020-crit1-<you>`,
  `assignment-1.md` in `comp4020-ass1-<you>`); `reflections/README.md` has the
  full rule. `pnpm check:evidence` checks the exact current name against the
  course API, not merely the presence of any well-named file. It answers the two
  standing prompts: the breakthrough that moved the work forward, and what this
  work changed about the developer you want to be. It stays out of the deployed
  site. It's due at the cutoff, and if it isn't in the repo by then the week
  doesn't count as shipped, however good the prototype is.
- **This file is process evidence.** The harness you build to direct the agent,
  this `CLAUDE.md` and any `AGENTS.md`, is itself read as part of how you
  worked. Keep it honest and current (see below).

You don't need a name, a student number, or any identity file in the repo: we
know whose repo it is. Spend the effort on the work.

## This file is yours

This CLAUDE.md is a starting point, not a fixed rulebook. As you learn what your
prototype needs --- a convention to hold the agent to, a sensor that keeps
catching you out, a fact about the stack the agent keeps getting wrong --- write
it down here. Growing this file is the work of harness engineering, and the gap
between this boilerplate and your own version is part of what your prototype
says about the developer you're becoming.
