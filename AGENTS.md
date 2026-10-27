# kempelen — project guidance

Shared by every coding assistant (Claude reads it through [CLAUDE.md](CLAUDE.md)) and by people.

kempelen is a Rust library for local speech: [Kokoro](https://huggingface.co/onnx-community/Kokoro-82M-v1.0-ONNX)
ONNX synthesis (24 kHz mono 16-bit WAV, levelled and peak-limited) with
[jespersen](https://github.com/Federiker/jespersen)'s pronunciation, and its own Mandarin front end.
It is named after Wolfgang von Kempelen, who built the first speaking machine. It was split out of
manuzio, the owner's closed-source app, which pins it by commit; the owner's narration and trailer
library melies uses it too. Licence: [Apache-2.0](LICENSE), with the [NOTICE](NOTICE). Never
published to crates.io. Owner: Federiker.

## Hard rules

- **Never commit the model, the voices or any audio** they make: they are staged (`mise run speech`)
  into the gitignored `tmp/speech`, or wherever `KEMPELEN_DATA_DIR` points. The references are
  hashes and small fixtures only.
- **Never commit eSpeak NG's output.** The golden corpus's eSpeak NG reference is captured locally
  into `tmp/` (`crates/kempelen/tests/golden/upstream_espeak.py`) and the tests that need it skip
  without it, as in jespersen ([docs/jespersen.md](docs/jespersen.md)).
- **No download at synthesis time.** The library verifies byte counts and SHA-256 and reads only
  what the caller staged; only `assets::stage` (feature `speech-download`) and `cuda::install`
  (feature `cuda-install`) download, when the caller asks.
- **The library takes inputs.** No environment variables, no app folders, no app names: the caller
  says where the model and voices are, which reading rules to use, whether CUDA may be used. Only
  tests, examples and scripts read `KEMPELEN_DATA_DIR`.
- **The output stays the same.** The phonemes and the CPU samples must equal the references recorded
  from manuzio ([docs/parity.md](docs/parity.md)): run the parity test after any change that could
  touch them, and ask the owner before changing a reference.
- **Ask before downloading anything large** (the model and voices are about 349 MB; the CUDA runtime
  about 1.6 GB). `mise run test:model`, `test:offline` and `bench` need the staged files.
- Do not commit, push or contact live services unless the owner asks. Commit with
  `git commit --only -- <paths>` when another session may share the index.

## Shared roadmap

- Read the root [TODO.md](TODO.md) at the start of each task to understand the remaining work and
  avoid duplicating another session's progress.
- Update relevant checklist items before finishing: add newly discovered work, split large tasks,
  note blockers and tick completed slices only after implementation and verification. A stub does
  not complete a feature. Once an item is done and described in the docs, remove it.
- Keep roadmap changes focused on the current task; preserve other contributors' entries and
  unresolved decisions.

## Layout

| Path | What |
| --- | --- |
| `crates/kempelen` | The library: synthesis (`tts`), phonemes (`phonemes`, the Mandarin front end in `phonemes/chinese`), the speech assets (`assets`; downloading them behind `speech-download`), CUDA (`cuda`; installing behind `cuda-install`), GPU detection (`gpu`, behind `gpu`) |
| `crates/kempelen/assets` | The pinned speech manifest (`speech-assets.json`) and the notice staged beside the files (`speech-NOTICE.txt`) |
| `crates/kempelen/mandarin` | The Mandarin readings (pypinyin, misaki, OpenCC data) and their generator |
| `crates/kempelen/tests` | Unit tests (`tests/unit`), integration tests (`tests/it`, one program), the golden corpora (`tests/golden`) |
| `crates/parity` | `kempelen-parity` (unpublished): the references recorded from manuzio and the checks against them, which manuzio can run too |
| `scripts` | `speech.ts` stages the speech files (development, CI, installers); install scripts; the commit-message hook |
| `docs` | The documentation (index below) |
| `tmp` | Gitignored: staged speech files, eSpeak NG captures, the reference harness (`tmp/harness`, path dependencies on `../manuzio`), experiments |

## Tests

- **Light tier** (`mise run test`): no model; runs everywhere, in CI on Linux and Windows.
- **Model tier** (`mise run test:model`): the `#[ignore]`d tests that run Kokoro on the CPU, reading
  the staged files in `KEMPELEN_DATA_DIR`; `mise run test:offline` checks every voice with a fresh
  empty folder beside them. CI runs both on pushes to main and on demand, with the files cached by
  the manifest's hash. The CUDA tests (`cuda::`) run only by hand, on a machine with the runtime.
- The samples are bit for bit only on the references' machine (ONNX Runtime's thread count changes
  Kokoro's noise): elsewhere the parity test checks the phonemes and says the samples are not
  comparable ([docs/parity.md](docs/parity.md)).

## Conventions

- UK English in documentation and messages. The API keeps the names manuzio calls
  (`Synthesizer`, `synthesize`), as typos.toml notes.
- Rust: dependencies as `{ workspace = true }`; Clippy `all` and `pedantic` warn; **no inline test
  modules**: unit tests live in `crates/<crate>/tests/unit/…` and are included with
  `#[cfg(test)] #[path = "…"] mod tests;`; integration tests in `crates/<crate>/tests/`. `lib.rs` and
  `mod.rs` stay declarative (modules and exports only).
- Typed errors in the library (`kempelen::Error`, [docs/errors.md](docs/errors.md)); `anyhow` only in
  examples and tests. Report errors explicitly; never substitute empty or success-shaped values.
- Optional parts are Cargo features (`gpu`, `cuda-install`, `speech-download`), off by default.
- Run `mise run audit` (cargo-deny, every feature) after changing dependencies.
- Ask before decisions that change the public API, the output, the licences or the pinned assets.

## Commit messages

The first line is `[prefix] What changed`, in the imperative and UK English (checked by the
commit-msg hook, `scripts/commit-message.sh`), as in jespersen and manuzio. One commit, one prefix.

| Prefix | For |
| --- | --- |
| `[add]` | Something new: a feature, a voice, a test tier |
| `[fix]` | A bug fixed, including a wrong pronunciation |
| `[remove]` | A feature, file or dead code taken out |
| `[refactor]` | Code reorganised with the same behaviour (the references must not change) |
| `[rename]` | Files or identifiers renamed or moved, nothing else |
| `[perf]` | Faster or smaller, same output; its measurements belong in the message and [docs/performance.md](docs/performance.md) |
| `[data]` | Language data or references changed: reading rules, Mandarin tables, golden corpora, parity references |
| `[docs]` | Documentation only |
| `[test]` | Tests only |
| `[fmt]` | Formatting only |
| `[build]` | Tooling, scripts and configuration |
| `[ci]` | The CI workflows |
| `[deps]` | Dependency updates (Renovate uses it), including the jespersen pin |
| `[security]` | A security fix or hardening |
| `[release]` | A version and its notes |
| `[revert]` | Undoing an earlier commit by hand |
| `[init]` | The repository's first commit |

## How manuzio uses kempelen

manuzio depends on it as a git dependency pinned to a commit:

```toml
kempelen = { git = "https://github.com/Federiker/kempelen", rev = "…" }
```

It maps its own types (its saved voice ids, book languages, data folder, `MNZ_` variables) onto
kempelen's, stages the same speech files for its installer from this repository's manifest, and runs
`kempelen-parity` against its adapters after switching. jespersen is pinned here, by commit, with
every language but `zh`: updating jespersen means bumping it here, checking the references, then
bumping kempelen in manuzio. What was copied from manuzio, and from which commit:
[docs/provenance.md](docs/provenance.md).

## Assistant conduct

- **Local notes:** if `tmp/docs/README.md` exists, read it at the start of each task and follow it;
  it holds local-only instructions that are deliberately not committed.
- Use the gitignored `tmp/` folder for downloads, captures, the reference harness and temporary
  scripts. Never stage it or make the build depend on it. Keep durable findings in tracked docs.
- Read sibling repositories (`../manuzio`, `../jespersen`) only; never change them from here.
- Stop helper processes you started when finished.
- When a decision is the owner's, ask with a small set of concrete choices and a recommended option
  (the question tool when there is one, numbered choices otherwise).

## Docs index

- [docs/provenance.md](docs/provenance.md) — the manuzio commit kempelen was copied from, and what came from where.
- [docs/parity.md](docs/parity.md) — the references recorded from manuzio, the harness and the parity test.
- [docs/jespersen.md](docs/jespersen.md) — pronunciation through jespersen: reading codes, unreadable words, the Kokoro symbol map, the golden corpus.
- [docs/mandarin.md](docs/mandarin.md) — the Mandarin front end and its data.
- [docs/performance.md](docs/performance.md) — speed and memory, measured against manuzio's numbers.
- [docs/errors.md](docs/errors.md) — the error types.
