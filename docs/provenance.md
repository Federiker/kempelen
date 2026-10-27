# Provenance

kempelen was split out of manuzio (owner, 27 October 2026). Its code, tests, data and documentation were
copied from manuzio at this commit, and adapted:

| | |
| --- | --- |
| Repository | manuzio (private) |
| Commit | `0ec8e8ede4be114c3f7d715f196a622e94c1b989` (25 October 2026) |
| Subject | `[deps] Slice 12: kuchiki and html5ever 0.25 replaced by dom_query 0.29 (editing) and scraper 0.29 (reading)` |
| Working tree | clean in every copied path (only manuzio's `TODO.md` and its split notes were changed) |

What came from where:

| kempelen | manuzio |
| --- | --- |
| the library (`crates/kempelen`) | `crates/tts` (`mnz-tts`) |
| `gpu` (feature `gpu`) | `crates/gpu` (`mnz-gpu`) |
| `cuda::install` and the shared download (features `cuda-install`, `speech-download`) | `crates/tts/src/cuda.rs`, `crates/downloader` (`mnz-downloader`) |
| `Voice` | `VoiceId` (`crates/models/src/voices.rs`) |
| `Language`, the shipped reading rules | `Language` (`crates/lang`), the packs' `narration` sections (`assets/json/lang/*.json`) |
| `assets/speech-assets.json`, `assets/speech-NOTICE.txt`, `scripts/speech.ts` | `assets/config/…`, `scripts/speech.ts` |
| `docs/jespersen.md`, `docs/mandarin.md` | `docs/jespersen.md`, `docs/mandarin.md` |

Before manuzio switches to kempelen, port whatever changed there since this commit, or ask the owner to:

```sh
git log 0ec8e8ede4be114c3f7d715f196a622e94c1b989..HEAD -- crates/tts crates/gpu crates/downloader \
  crates/lang crates/models assets/config/speech-assets.json assets/config/speech-NOTICE.txt \
  assets/json/lang scripts/speech.ts docs/jespersen.md docs/mandarin.md
```

The output is checked against references recorded from manuzio's crate at the same commit: see
[parity.md](parity.md).

## What changed for a caller of `mnz-tts`

The pipeline is manuzio's, line for line where it could be; what had to become generic:

| `mnz-tts` | kempelen |
| --- | --- |
| `mnz_models::VoiceId` | `kempelen::Voice`, the same serialised names and aliases (`hf_*` read as `bf_alice`), plus `Voice::ALL` and `name()` |
| `mnz_lang::Language` | `kempelen::Language`, the same variants and codes (`code()`, serde) |
| the packs' `narration` (`mnz_lang::Narration`, `Abbreviation`) | `phonemes::{Narration, Abbreviation}`, the same JSON form; every language's copy shipped in `data/reading-rules.json`; a caller's own through `Synthesizer::set_reading_rules` or `phonemize_with_rules` |
| `Synthesizer::new(voice, speed).await`, `with_cuda(…).await` | `Synthesizer::new(&data, voice, speed)`, `with_cuda(&data, …)`: synchronous (nothing in them waited) |
| `tts(text, voice, out: &str).await`, `tts_with_options`, `load_voice(…).await` | the same with `&SpeechData` first, `out: &Path`, synchronous |
| `utils::{ensure_model, ensure_voice, ensure_all_models}` | `SpeechData::{model, voice, asset}` |
| `bundled_speech_dirs()` (`tool_dirs()/tts`, the repository's `assets/tools/tts` in debug builds), then `MNZ_TTS_DATA_DIR` or the app data's `models/tts` | the caller's `SpeechData::with_dirs([...])`, searched in order with the same rule (the first folder that has a file must hold it intact) |
| `include_str!` of `assets/config/speech-assets.json` | `assets::MANIFEST`, `assets::manifest()`, `assets::NOTICE` |
| `cuda::supported()` (Windows x64, and `MNZ_TTS_CUDA` not `0`/`off`/`false`/`no`) | `cuda::available()` (Windows x64); the variable is the app's to read |
| `cuda::runtime_dir()` (inside the data folder) | `cuda::runtime_dir(data_dir)`, the same `cuda/<RUNTIME_ID>` layout |
| `cuda::install` | the same, behind the `cuda-install` feature, with `mnz-downloader`'s download copied in |
| `mnz_gpu::{detect, …}` | `kempelen::gpu::{detect, …}`, behind the `gpu` feature |
| `anyhow::Error` | `kempelen::Error` ([errors.md](errors.md)); a callback stops a synthesis with `Error::interrupted(e)` |

Stays in manuzio: reading `MNZ_TTS_DATA_DIR`, `MNZ_TTS_CUDA` and `MNZ_TOOLS_DIR`, the Tauri resource
folder (and its test, `tauri_resource_registration_includes_bin_speech_ahead_of_repository_fallback`),
the app data folder, and the message the app shows when a speech file is missing (kempelen's says no
download is attempted; manuzio's added "reinstall the application or run `mise run engines`").

Not copied: the eSpeak NG reference (`tests/golden/espeak-upstream.json`), which stays out of the
repository ([jespersen.md](jespersen.md)).
