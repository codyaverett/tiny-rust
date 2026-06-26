# Organizing 100+ Examples

Proposal for scaling the repo from 40 examples to 100+ without losing the
"each example builds on the last" identity. Produced by the roadmap workflow.

> Status: proposal. Nothing here is migrated yet. The numbering migration (section 1) is the
> one piece worth doing soon, while only 40 dirs exist.

## Goal
Preserve the repo's identity - a single monotonic 'each example builds on the last' journey - while scaling to 100+ entries. Keep directories FLAT and chronological; layer category/series navigation through metadata and a generated README, not through nested folders (nesting would break the workspace and the numbered-progression story).

## 1. Numbering (one-time migration, do it now while only 40 exist)
- Switch every dir to **3-digit zero-padded**: `001-release-opts` ... `040-tiny-kafka-cluster`, then `041-tiny-grep` onward. Reason: with mixed 2- and 3-digit names, lexical sort breaks (e.g. `041-` sorts BEFORE `40-`). Padding to 3 digits guarantees `ls` order == build order forever (up to 999).
- Do it with a scripted `git mv` loop plus a `docs/RENUMBER.md` redirect table so old links resolve. Cheap at 40 dirs, miserable at 140.
- Numbers are **build order / chronological and never reused**. A number is assigned only when an idea moves to `planned`. The category an example belongs to has zero effect on its number.

## 2. Naming
- Directory: `NNN-tiny-<thing>` (e.g. `045-tiny-redis`).
- Cargo package + binary name: `tiny-<thing>` (drop the number - keeps crate names stable if order ever shifts). Always lowercase, hyphenated, `tiny-` prefix.

## 3. Status tracking (idea -> planned -> built)
- Single source of truth: promote `docs/ideas.md` into **`docs/CATALOG.md`**, one row per idea with columns: `Status` (💡 idea / 📋 planned / ✅ built), `NNN` (blank until planned), `Name`, `Category`, `Series`, `Tier` (★ count), `no_std`, `Size`.
- An idea's lifecycle is just editing its row: add a number + flip 💡->📋 when scheduled, flip ->✅ when the dir lands.
- Each built example carries machine-readable metadata in its `Cargo.toml`:
  ```toml
  [package.metadata.tiny]
  category = "Data Structures & Databases"
  series   = "build-your-own-database"
  tier     = 5
  difficulty = "medium"
  size_bytes = 14000
  ```

## 4. Catalog -> README generation
- Add `scripts/gen-readme` (on-brand: make it a future `tiny-md`/catalog example itself) that reads `docs/CATALOG.md` + each `Cargo.toml [package.metadata.tiny]` and regenerates the README tables. README is treated as generated output between `<!-- BEGIN CATALOG -->`/`<!-- END CATALOG -->` markers; humans edit `CATALOG.md`, never the table by hand.
- A CI check fails if the committed README diverges from `gen-readme` output, keeping table, sizes, and statuses honest.

## 5. README structure (newcomer navigation, top to bottom)
1. **Pitch** - the one-line hook (how small can Rust get / tiny binaries doing real work).
2. **Start Here** - a 5-step golden path for a first-timer: `001-release-opts` -> `003-no-std` -> `004-raw-syscall` -> `041-tiny-grep` -> pick a series.
3. **The Size Journey (001-008)** - the original incremental shrink story, kept intact.
4. **Series cards** - 6-8 themed tracks (Unix Toolbox, Parse Anything, Build Your Own Language, Emulate the Classics, Compression Step by Step, Pixels From Scratch, AI From Scratch, Build a Tiny OS), each a clickable ordered list. Backed by **`docs/SERIES.md`** which defines each track's ordered membership.
5. **Full catalog table** - generated, grouped by category, with status emoji so the backlog (💡) and roadmap (📋) are visible alongside shipped (✅) work.
6. **Docs** - links to size-optimization-guide, troubleshooting, CATALOG, SERIES.

## 6. Per-example README template (enforced)
Each dir's `README.md` follows a fixed shape: one-liner, why it matters, `build & run`, measured size, 'what you learn', and **Related: prev/next in series** links. Consistency lets a reader sit inside one example and still navigate the track.

## 7. Cargo workspace
- Keep one root workspace. As members pass ~100, list them explicitly (or glob `0*-tiny-*`/`1*-tiny-*`) and define **`default-members`** = the stable std/no_std+libc set so plain `cargo build` stays fast.
- Keep the existing carve-outs documented and EXCLUDED from the default build, exactly as 05 (wasm), 06/07 (nightly) already are: the wasm target, nightly `build-std` examples, and especially the **Tiny OS / bare-metal** track (custom target JSONs, `no_std` no-libc, QEMU run steps). Bare-metal examples get their own `docs/BARE-METAL.md` build/run instructions rather than fighting the host-target workspace.

## 8. docs/ layout going forward
- `docs/CATALOG.md` - master idea/status table (replaces ideas.md as source of truth).
- `docs/SERIES.md` - the curated tracks and their ordering.
- `docs/RENUMBER.md` - old->new number redirects from the migration.
- `docs/BARE-METAL.md` - how to build/run the OS track under QEMU.
- existing `size-optimization-guide.md`, `troubleshooting.md` stay.

