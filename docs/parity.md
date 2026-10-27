# Parity with manuzio

kempelen must say exactly what manuzio's speech crate said: the same phonemes for every line, and on
the CPU the same samples, bit for bit. Before any code was copied, the references were recorded from
manuzio's own crate at the [provenance](provenance.md) commit, and committed with the checks that hold
kempelen to them (27 October 2026).

## The references

`crates/parity` (`kempelen-parity`, unpublished) holds them and the comparison:

- **`references/phonemes.json`**: 2,072 lines, each with its reading code, the engine's raw output and
  what is fed to Kokoro (`phonemize_with`), engine errors included as `<error: …>` (there are none).
  They are, for every reading code narration has (all 51, `en-us` and `en` included): the language's own
  lines of the golden corpus and its shared lines (`crates/kempelen/tests/golden/corpus.json`), the whole
  Mandarin corpus for `cmn` (`mandarin-corpus.json`), and, for each language's reading rules, a line
  putting every abbreviation where its rules matter (before a number, before a lower-case and a
  capitalised word, capitalised at a sentence start, at the end), plus two lines of ordinals and initials.
- **`references/synthesis.json`**: 42 syntheses on the CPU, each by sample count and SHA-256 of its
  little-endian `f32` samples (of the file's bytes for a WAV), with `synthesize_into`'s returned count and
  its `before_chunk` reports, and each model chunk's digest for `synthesize_chunks`. The cases cover:
  - voices of every accent (`af_heart`, `af_alloy`, `af_nova`, `am_adam`, `am_michael`, `bf_emma`,
    `bf_alice`, `bm_george`, `ef_dora`, `em_alex`, `ff_siwis`, `if_sara`, `pm_alex`, `zf_xiaoxiao`,
    `zm_yunxi`), reading their own language and book languages they were not trained on (German, Italian,
    Russian, Serbian in Latin letters, Greek, Chinese, English by a Chinese voice);
  - speeds 0.7, 0.9, 1.0 and 1.3;
  - the three loudness settings: the default (-20 dBFS speech), none (the voice's own level, peaks still
    limited) and retail (peaks at -4 dBFS);
  - the whole buffer, streamed, written as a WAV, and per model chunk;
  - a text of several model chunks (20 sentences, 1,670,400 samples), passages split by a line break, a
    loud voice whose peaks need limiting, punctuation pauses (`One. Two. …` against `One two …`),
    abbreviations and initials, an emoji left silent, a Latin name inside Russian text;
  - four failures (no speech, whole and streamed; Japanese; speed zero), compared by failing only.

### The machine

Kokoro's decoder draws noise in an order that depends on ONNX Runtime's thread count, which is every
logical core, and ONNX Runtime's kernels differ by platform and CPU. So the samples are reproducible only
on one kind of machine. The references were recorded on **Windows 11 x64, 16 threads** (AMD Ryzen 7
5800H, 8 cores), with ONNX Runtime 1.24.2 from `ort` 2.0.0-rc.12 (pyke's static CUDA-enabled core, run on
the CPU), twice, with identical results. Elsewhere the synthesis check says it is skipped and why; the
phoneme check runs everywhere, in CI too.

## The checks

- `parity::the_phonemes_are_those_manuzio_read` (light tier, `mise run test`): every reference line
  through `JespersenPhonemizer` or `MandarinPhonemizer`, raw and fitted to Kokoro.
- `parity::the_samples_are_those_manuzio_synthesized` (model tier, `mise run test:model`): every case
  through `Synthesizer`, as the harness ran it through manuzio. It needs the staged model and voices.

Both passed at the first run after the port: 2,072 of 2,072 lines, 42 of 42 syntheses.

A change that alters a reference is a change of output: find out why before anything else, and ask the
owner before recording new references. CUDA output is never compared (it differs from run to run); the
test `without_the_gpu_runtime_narration_falls_back_to_the_same_cpu_samples` checks that a missing GPU
runtime leaves the CPU's own samples.

## manuzio after switching

manuzio keeps its own types (`VoiceId`, `Language`, its data folder and variables) and maps them onto
kempelen's. To check its adapters give the same output, it adds `kempelen-parity` from the same pinned
commit as a dev-dependency and runs the same cases through its own code:

```toml
[dev-dependencies]
kempelen-parity = { git = "https://github.com/Federiker/kempelen", rev = "…" }
```

```rust
let references = kempelen_parity::SynthesisReferences::committed();
if references.machine == kempelen_parity::Machine::current() {
    let differences = kempelen_parity::check_synthesis(&references, |case| {
        // Build manuzio's own VoiceId and Language from case.voice and case.language, synthesize
        // through manuzio's adapter, and return the samples as kempelen_parity::Produced.
        todo!()
    });
    assert!(differences.is_empty(), "{}", differences.join("\n"));
}
```

`check_phonemes` works the same way for the phonemes. The case names, voices (`af_heart`) and languages
(`de`, `sr-Latn`) are plain strings, the serialised names both sides share, so nothing in the
references depends on kempelen's types.

## The harness

The program that recorded the references lives in the gitignored `tmp/harness`, because it names paths
into the private `../manuzio` (owner, 27 October 2026). To rebuild it: a binary crate with its own
`[workspace]`, manuzio's `Cargo.lock` copied beside it (so every dependency has manuzio's version), and
path dependencies on `../../../manuzio/crates/{tts,lang,models,audiobook,epub}` and on
`../../crates/parity`. Its commands:

- `phonemes OUT`: the phoneme references, as described above;
- `synthesis OUT [NAME]`: the synthesis cases, defined in the harness, run through `mnz-tts`;
- `bench-texts OUT`: the benchmark's text (`crates/kempelen/examples/bench/oz.json`), planned by
  manuzio's audiobook;
- `bench TEXTS`: manuzio's speech benchmark through `mnz-tts`, for [performance.md](performance.md).

It runs with `MNZ_TOOLS_DIR=../manuzio/assets/tools` (so manuzio finds its staged speech files),
`MNZ_TTS_DATA_DIR` pointing at an empty folder (no per-user copy) and `MNZ_TTS_CUDA=off`.
