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
| `cuda::install` and its download (feature `cuda-install`) | `crates/tts/src/cuda.rs`, `crates/downloader` (`mnz-downloader`) |
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
