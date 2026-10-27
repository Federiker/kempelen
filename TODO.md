# kempelen TODO

The open work only (see [AGENTS.md](AGENTS.md#shared-roadmap)). Finished work is described in
[docs/](docs) and the git history; tick a slice only when it is done and verified, then remove
finished items when the area is next tidied. Checks that need ears, a GPU or native speakers go under
[Verification](#verification).

Most items came from manuzio's TODO (P1, narration) when kempelen was split out of it, 27 October 2026
([docs/provenance.md](docs/provenance.md)); manuzio's own file keeps them until it switches.

---

## Split from manuzio

- [ ] manuzio switches to kempelen (manuzio's `SPLITTER-HERE.md`, slice "kempelen"):
  - [ ] Agree with the owner on the commit manuzio pins, and check the parity test passes there
  - [ ] manuzio's adapters (voice ids, book languages, data folder, CUDA runtime) pass
        `kempelen-parity` against the same references ([docs/parity.md](docs/parity.md))
  - [ ] manuzio's speech staging reads this repository's manifest and notice
        (`crates/kempelen/assets`) from the pinned checkout
- [ ] Before the switch, port what changed in manuzio since the provenance commit, or ask the owner
      to (the `git log` command in [docs/provenance.md](docs/provenance.md))
- [ ] The sample references hold only on Windows x64 with 16 threads ([docs/parity.md](docs/parity.md)),
      so CI checks the phonemes but not the samples. Owner to decide: record kempelen's own sample
      references on CI's Linux runner (a regression guard there; not manuzio's output, which cannot run
      in CI)
- [ ] Owner checks: the repository on GitHub (`Federiker/kempelen`, pushed by hand), CI green on
      Linux and Windows with the model tier, and a chapter narrated by manuzio after the switch that
      sounds the same

## Pronunciation: jespersen ([docs/jespersen.md](docs/jespersen.md))

- [ ] Ask jespersen whether single letters should be letter names: they are read as sounds in Italian
      (`x y z` → `ks ˈi dz`), where eSpeak NG says the names
- [ ] Mandarin through jespersen: score its misaki notation against `mandarin-misaki.json`; switch only if
      it matches (owner, 24 October 2026: keep kempelen's own front end until then)
- [ ] Whole sentences to jespersen: kempelen hands it the words between Kokoro's punctuation, so context
      (homographs, "read") stops at a comma; try the whole sentence with its punctuation kept
- [ ] The shipped reading rules' abbreviations overlap jespersen's own: decide which wins, or drop the
      entries jespersen covers
- [ ] An English abbreviation in another language's text ("Mr. Smith" in Italian) still ends a sentence
      for Kokoro, as it did with eSpeak NG
- [ ] Golden corpus lines for the languages jespersen added (Greek … Akan): no eSpeak reference exists,
      so commit jespersen's output once native speakers have checked it, and guard it
- [ ] Abbreviations inside a word are not found (German "Goethestr."): consider suffix entries
- [ ] Later, with a native speaker: ordinal forms for numbers written with a full stop (German
      "am 3. Mai" → "dritten")

## Audio quality

- [ ] Pops inside chunks: `af_heart` shows about 7 abrupt onsets in 37 s (probably plosive bursts). If the
      owner still hears pops, try a gentle onset smoothing (≤ 0.3 ms) or a de-click limited to those
      events, and check by ear
- [ ] Artefacts at other speeds: click counts at speed 1.0 / 0.9 / 0.7 on `af_heart` and `af_nicole` with
      the `audio_check` example (audiobooks default to 0.9); is `af_nicole`'s slower reading (about 60 %)
      by design?
- [ ] CPU against CUDA output (needs the 1.6 GB runtime download; ask first)
- [ ] The click and clip detector as a permanent model-tier check on a short sample

## Mandarin ([docs/mandarin.md](docs/mandarin.md))

- [ ] Tone sandhi (3rd + 3rd tone): try it, listen, keep it only if it sounds better
- [ ] Heteronyms: pypinyin's phrase list misses words and applies no 一 / 不 tone changes; review with a
      native speaker's list
- [ ] Decide on Kokoro v1.1-zh (more Chinese voices, newer front end) against the v1.0 voices: size,
      licence, ONNX availability
- [ ] Re-measure the loudness of the 8 Chinese voices with Chinese text

## Verification

Checks for the owner (ears, a GPU, native speakers); these changes are made in kempelen, the samples
with the `audio_check` example.

- [ ] Audio pops: compare narration before and after the limiter and say whether the pops are gone
- [ ] Mandarin: `zf_xiaobei` and `zm_yunjian` samples, against narration made before the new reader
- [ ] The shipped reading rules' abbreviations, English spoken forms included ("St." is always
      "Saint", "Dr." always "Doctor"), and `ordinalDot` for Basque and Lithuanian (both off)
- [ ] Italian ʎ (docs/jespersen.md, "Fitting to Kokoro"): long `lʲː` chosen on "meglio", short ʎ on
      "sbagliavo"; listen to "gli" at a sentence's start, and whether Catalan and Portuguese ʎ (lh)
      sound wrong the same way
