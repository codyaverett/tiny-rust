# tiny-rust Ideas

Where we track what to build next, and every idea we have already considered so
nothing gets lost.

It started as a question -- how small can a Rust binary get -- and grew into 40
examples doing real work in tiny binaries. This folder is the plan for the next
100+.

## The docs

- **[roadmap.md](roadmap.md)** -- what to focus on up front. First milestone,
  the focus batch (~15 in build order), and 8 themed learning series.
- **[catalog.md](catalog.md)** -- the full backlog. All 248 candidate ideas,
  grouped by category, each tiered with a one-line hook. The master list.
- **[organization.md](organization.md)** -- how to organize, number, and present
  100+ examples without losing the "each one builds on the last" story.

## How ideas were picked

1. Fan-out brainstorm across 17 domains -- 253 raw ideas.
2. Deduped against the existing 40 examples.
3. Scored for build-worthiness, then tiered: `must-build` ★★★★★ ·
   `strong` ★★★★ · `nice` ★★★.
4. Tagged into learning series and ranked into a focus batch.

248 made the cut: **98 must-build**, 135 strong, 15 nice.

## If you are new here

Skim the [roadmap](roadmap.md) series cards -- pick the track that sounds fun
(Unix toolbox, build a tiny OS, AI from scratch, emulators, compression) and
follow it in order. Each example is small and self-contained.

## Status legend

Used across these docs and (eventually) the root README:

- 💡 idea -- in the catalog, not scheduled
- 📋 planned -- has a number and an implementation plan
- ✅ built -- shipped as a numbered example
