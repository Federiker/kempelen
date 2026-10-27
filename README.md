# kempelen

Local speech for Rust: [Kokoro](https://huggingface.co/onnx-community/Kokoro-82M-v1.0-ONNX) v1.0 ONNX
synthesis with [jespersen](https://github.com/Federiker/jespersen)'s pronunciation, offline, on the CPU
or an NVIDIA GPU. Named after Wolfgang von Kempelen (1734–1804), who built the first speaking machine.

It was split out of manuzio, the owner's book-production app, which pins it by commit, as does melies
(narration and trailers). It is not on crates.io.

## What it does

- **Synthesis.** A `Synthesizer` loads the model and one of 45 voices once and reads any number of texts
  into 24 kHz mono samples, or a 16-bit PCM WAV. Speed must be finite and positive. Line breaks separate
  passages; long passages are split at sentence and clause boundaries into model chunks of at most 400
  phonemes.
- **Streaming.** `synthesize_into` hands the samples to a sink a block at a time, with a callback before
  every model chunk (progress, cancellation); memory stays flat however long the text, and the samples
  equal `synthesize`'s exactly. `synthesize_to_wav` streams into a file the same way.
- **Level.** Every synthesis is levelled: speech is brought to -20 dBFS RMS (voices differ by up to
  12 dB) and peaks are limited, so nothing clips. `set_loudness(None)` keeps a voice's own level;
  `LoudnessTarget::RETAIL` lowers the peak ceiling to -4 dBFS for store-ready audiobooks.
- **Pronunciation.** Text is normalised, then read as its book `Language` (independently of the voice's
  accent): every language but Japanese, Ukrainian and Bulgarian. jespersen reads all of them but
  Mandarin; a refused Latin word is retried as English, anything still unreadable is left silent and
  logged. Mandarin has its own front end, a port of misaki ([docs/mandarin.md](docs/mandarin.md)).
  Sounds Kokoro lacks are mapped to the closest it has, so support does not guarantee native quality
  ([docs/jespersen.md](docs/jespersen.md)).
- **Reading rules.** Abbreviations are read in full (`Dr.` → "Doctor", `No. 5` → "number 5"), so their
  full stop is not a sentence end for Kokoro; nor is a full stop before a lower-case word, after an
  initial (`L. Frank Baum`), or, where the language writes ordinals so, after a number of up to three
  digits (`am 3. Mai`). kempelen ships the rules of every language (`data/reading-rules.json`); a caller
  may pass its own (`Synthesizer::set_reading_rules`, `phonemize_with_rules`).

```rust
use kempelen::{Language, Voice, assets::SpeechData, tts::Synthesizer};

let data = SpeechData::new("/path/to/speech"); // models/model.onnx, voices/*.bin
let mut synthesizer = Synthesizer::new(&data, Voice::BfEmma, 0.9)?;
synthesizer.read_as(Language::German)?; // a British voice reading a German book
synthesizer.synthesize_to_wav("Dr. Müller kam am 3. Mai.", "chapter.wav".as_ref())?;
```

Errors are one typed enum, `kempelen::Error` ([docs/errors.md](docs/errors.md)).

## The speech data

The model and the voices (about 349 MB) are **never downloaded at synthesis time and never shipped in
this repository**. The caller stages them and says where they are with `SpeechData`; each file is
checked by byte count and SHA-256 before use, and a missing or damaged file is an error.

- The pinned files are listed in [`crates/kempelen/assets/speech-assets.json`](crates/kempelen/assets/speech-assets.json)
  (Hugging Face revision, URL, size and SHA-256 of each), with the notice that goes beside them,
  [`speech-NOTICE.txt`](crates/kempelen/assets/speech-NOTICE.txt). The crate exposes both as
  `kempelen::assets::MANIFEST` and `NOTICE`, and the parsed manifest as `assets::manifest()`.
- `bun scripts/speech.ts [DIR]` (`mise run speech`) stages them into `DIR` (default `tmp/speech`, or
  `KEMPELEN_DATA_DIR`), downloading only what is missing or damaged, from allowed hosts only, checking
  sizes and checksums, and writes `NOTICE.txt` and `manifest.json` beside them.
- `kempelen::assets::stage(dir, progress)` (feature `speech-download`) does the same from Rust, for an
  application that fetches the files on first use instead of shipping them: the same checks, each file
  downloaded into a temporary `.part` file, refused if larger than pinned, and moved into place only once
  its size and SHA-256 match.
- **An application staging the same files for its installer** reads the manifest and the notice from
  the kempelen checkout Cargo pinned, so there is one source and no copy. Find it with
  `cargo metadata --format-version 1` (the `manifest_path` of the package named `kempelen`, whose folder
  holds `assets/`), then either read the two files, or import `stageSpeechAssets(destination, { manifest,
  notice })` from that checkout's `scripts/speech.ts`. This is how manuzio stages them.

## Optional parts

| Feature | What |
| --- | --- |
| (always) | `cuda`: on 64-bit Windows, `Synthesizer::with_cuda` runs the model on an NVIDIA GPU through an installed CUDA runtime, falling back to the CPU if the GPU fails later; `cuda::available`, `runtime_dir`, `is_installed` |
| `cuda-install` | `cuda::install`: downloads the pinned provider and NVIDIA libraries (about 1.6 GB, 2.3 GB installed) into a runtime folder, with checksums; only when the caller asks |
| `speech-download` | `assets::stage`: downloads the pinned model, voices and notices (about 349 MB) into a speech data folder, only what is missing or damaged; only when the caller asks |
| `gpu` | `gpu::detect`: which accelerator a machine has (NVML, Vulkan, Apple silicon) and its memory; it downloads nothing |

On 64-bit Windows kempelen links pyke's CUDA-enabled static ONNX Runtime core, as manuzio did, because
the CPU output depends on that build. An application using CUDA ships pyke's
`onnxruntime_providers_shared.dll` beside its executable, and builds with `ORT_CUDA_VERSION=12` (in its
own `.cargo/config.toml`, as here) so the core matches the runtime `cuda::install` fetches. The CUDA
libraries keep NVIDIA's and pyke's terms and are never shipped by kempelen.

## Same output as manuzio

The phonemes of the golden corpora and 42 CPU syntheses were recorded from manuzio's own crate before
the split; kempelen matches them exactly, sample for sample ([docs/parity.md](docs/parity.md)). The
`kempelen-parity` crate lets manuzio run the same check after switching. Speed: [docs/performance.md](docs/performance.md).

## Development

`mise install`, then `mise run test` (the light tier: no model needed) and, with the speech data staged
(`mise run speech`, about 349 MB, or `KEMPELEN_DATA_DIR` pointing at a staged copy), `mise run
test:model`, `test:offline` and `bench`. More in [AGENTS.md](AGENTS.md); the open work in
[TODO.md](TODO.md).

## Releasing

A release is the tag `v<version>` on `main`, with the version set once in the
root `Cargo.toml` (`[workspace.package]`). There is no GitHub release or
crates.io upload (the crates are `publish = false`), so plain Git is enough:
no token, only the credentials you already push with. Programs still pin the
commit a tag names (`git rev-parse v<version>^{commit}`), which no later
change to the tag can move.

Run `mise run tag` on `main` with everything committed and pushed or
pulled. It asks the new version (Enter for the next patch, or the current
one when it was never tagged), writes it to `Cargo.toml` and `Cargo.lock`
and commits those alone (`[release] kempelen <version>` by default, with
the hooks), tags `v<version>` (annotated), then asks before
`git push --atomic origin main v<version>`. Nothing changes before the
commit; if you stop after it, it says what it did, how to push later and
how to undo it here. The tool is the `kempelen-tag` crate, copied from
jespersen's.

## Licence

[Apache-2.0](LICENSE), with the [NOTICE](NOTICE). The Mandarin front end is a port of misaki
(Apache-2.0) with data from pypinyin and OpenCC (MIT, Apache-2.0), which is why kempelen is not CC0 like
jespersen. jespersen (CC0) brings CMUdict, whose BSD notice is in the NOTICE. The Kokoro model and voices
(Apache-2.0) are downloaded, not shipped; so is the optional CUDA runtime, under NVIDIA's and pyke's
terms. The golden corpus's eSpeak NG reference is never committed ([docs/jespersen.md](docs/jespersen.md)).
