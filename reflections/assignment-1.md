# Assignment 1 reflection

**The breakthrough that moved the work forward** was realising book 3's PDF
sampling didn't need a mode-switch bolted onto the renderer — it needed to be
opt-in by default. Giving `Hittable` and `Material` trait methods that default
to "behave exactly as book 1/2 already did" (`is_specular` defaults to `true`,
`pdf_value` defaults to `0.0`) meant every existing scene and material kept
rendering bit-for-bit identically unless it explicitly opted into the new
machinery. That one design choice is what let me add next-event estimation,
a mixture PDF, and a Cornell box with a real light without ever worrying I'd
silently broken the classic three-sphere scene — and I didn't just trust the
design, I wrote `cargo test` cases that assert cosine/mixture sampling
converge to naive's radiance on light-less scenes, so the claim is checked,
not assumed.

**What this changed about who I want to be as a developer** came from the
opposite direction: discovering an entire feature arc had sat uncommitted for
a whole session, and having to reconstruct it into commits after the fact.
Getting the code right wasn't the failure — leaving no trail of *how* it got
right was. I've started treating commit hygiene as part of the deliverable
rather than administrative cleanup, and I'd rather bake that into the
project's standing instructions than rely on remembering to do better next
time. A process a stranger can follow is now a design constraint I hold
myself to, not an afterthought I tidy up before submitting.
