# jespersen: the pronunciation

Kokoro reads IPA-like symbols, not text, so narration needs phonemes. For every language but Mandarin
they come from [jespersen](https://github.com/Federiker/jespersen), the owner's own text-to-phonemes
library, dedicated to the public domain (CC0 1.0). It replaced eSpeak NG, a GPL program manuzio once
ran beside itself (24 October 2026, before kempelen was split out). Nothing GPL is built, linked or
committed here.

## How kempelen uses it

- **Dependency.** `jespersen` is a git dependency pinned to a release tag (`tag = "v0.1.0"` in the
  root `Cargo.toml`, commit `23a215b`; `Cargo.lock` records the commit), with every language feature
  but `zh`, as manuzio had it. manuzio pinned `a88d71d`, three commits earlier: v0.1.0 stresses the
  Italian verb "è", the one change in the references ([parity.md](parity.md)). Its data is compiled
  in; there is no helper process, no staging and nothing to download. `cargo deny` allows its git
  source.
- **The seam.** `JespersenPhonemizer` (`crates/kempelen/src/phonemes/jespersen.rs`) implements the
  `Phonemizer` trait. The pipeline around it is kempelen's: text normalisation, the reading rules
  (abbreviations, ordinal full stops, initials: `phonemes/reading.rs`, with the rules shipped in
  `crates/kempelen/data/reading-rules.json`), the punctuation Kokoro uses (kept by kempelen, so
  jespersen drops what reaches it: an initial's full stop must not become a sentence end), chunking and
  the Kokoro fitting.
- **Reading codes.** Every voice and book language maps to a reading code (`reading_code`): `en-us`,
  `en` (British: TRAP's vowel in bath, as eSpeak NG's British voice), `es` (Castilian), `pt`
  (European), `no` (Bokmål), `sr` and `sr-latn` (one per script, so each finds its own reading rules;
  jespersen reads both), and so on. The codes began as eSpeak NG's and never change: applications may
  store them (manuzio fingerprints resumed audiobooks with them).
- **Mandarin** keeps kempelen's own front end, checked against misaki ([mandarin.md](mandarin.md)); only
  the English words inside Chinese text are read by jespersen (American English). Owner decision,
  24 October 2026: measure jespersen's Mandarin against misaki before switching.
- **Languages.** Every book language but Japanese, Ukrainian and Bulgarian can be narrated
  (`can_narrate`). The ones jespersen added (Greek, Swahili, Kinyarwanda, Kirundi, Somali, Oromo,
  Luganda, Lingala, Shona, Macedonian, Serbian, Malay, Yoruba, Igbo, Hausa, Ewe, Akan) wait for
  listening checks ([TODO.md](../TODO.md)).

### What it cannot read

jespersen refuses what it cannot read (an emoji, a symbol, another script, a Latin word in Russian
text), where eSpeak NG always said something. Owner decision, 24 October 2026: a refused word in Latin
letters is read as American English (a name in Russian text), and anything still unreadable is left
silent and logged once per synthesizer (`Narration (ru) leaves out what it cannot pronounce: …`, through
the `log` crate). One stray character never stops a book.

### Fitting to Kokoro

Kokoro learnt a few languages in eSpeak NG's notation, which jespersen writes. `kokoro_symbol`
(`phonemes/get_phonemes.rs`) maps the symbols Kokoro has no token for to the closest it has: ɬ ɭ ɫ → l,
ʑ → ʒ, ʉ → u, ä → a, `?` → ʔ (Danish stød), ʏ → y, ɵ → ø, and ε → ɛ (Danish short e). Without the map
Kokoro silently drops the letter and its sound. `every_letter_jespersen_writes_reaches_kokoro` fails if
any letter in the corpus (and a line in each language the corpus lacks) would still be dropped.

A sound Kokoro has but says badly in one language is respelt for that language only (`kokoro_sounds`):
Italian ʎ (gli, meglio) becomes a palatalised l, `lʲ`, and `lʲː` where jespersen writes it long
(`ʎʎ`, between vowels), as the owner chose by ear on 27 October 2026 against ʎ, ʎʎ, ʎː and lj. Other
languages keep ʎ until someone listens to them.

eSpeak NG's own notation writes Danish short e as the Greek ε (`fem` → `fˈεm`), and jespersen copies it
on purpose; owner, 24 October 2026: jespersen stays faithful and the caller maps it. To ask jespersen:
single letters read as sounds rather than letter names in Italian (`x y z` → `ks ˈi dz`).

## Updating jespersen

1. Bump `tag` in the root `Cargo.toml` to jespersen's new release tag; run `cargo update -p jespersen`
   and `mise run deny`.
2. `mise run test`: the letter guard, the phoneme tests and the phoneme parity
   (`parity::the_phonemes_are_those_manuzio_read`), which fails on any changed line. A newer jespersen
   is expected to change some: check the differences, then ask the owner before recording new
   references ([parity.md](parity.md)).
3. With the eSpeak NG reference captured (below): `mise run golden` (`… --example golden --
   capture-jespersen`, then `… -- report espeak-upstream jespersen`); update the tracker below, and
   raise the floors in `tests/it/golden.rs` where jespersen came closer.
4. Tell the applications that store reading codes or pronunciation-dependent data (manuzio bumps its
   `PRONUNCIATION_REVISION`, so a book is not narrated half and half).
5. A new jespersen language needs a reading code (`reading_code`, `LANGUAGES` in `jespersen.rs`), its
   reading rules if it has abbreviations, and a line in the letter guard.

## The golden corpus

`crates/kempelen/tests/golden/`: per narrated language, shared lines (numbers, dates, abbreviations,
punctuation) and lines of its own (`corpus.json`), and the Mandarin corpus (`mandarin-corpus.json`).
The yardstick is upstream eSpeak NG 1.52.0, because Kokoro was trained on its phonemes. As in
jespersen ([its clean room](https://github.com/Federiker/jespersen/blob/main/docs/clean-room.md)), eSpeak
NG's output is **never committed**: `upstream_espeak.py` (with the `espeakng-loader` PyPI library,
development only) runs it as a black box, and `golden capture-upstream PYTHON` keeps its answers in the
gitignored `tmp/golden/espeak-upstream.json`. The two tests that need it
(`jespersen_keeps_its_parity_with_the_espeak_reference`, `the_reference_covers_every_corpus_language…`)
skip themselves without it, so CI never needs it. `golden report REFERENCE CANDIDATE` scores any engine
by exact lines and matched words. Mandarin (`cmn`) is measured against misaki instead
(`mandarin-misaki.json`, committed: misaki is Apache-2.0).

What is committed and always checked is jespersen's own output through kempelen, recorded from manuzio
before the split: `crates/parity/references/phonemes.json` ([parity.md](parity.md)).

### Parity tracker

jespersen against the upstream eSpeak NG reference: exact lines and matched words, in %. Measured on
commit `a88d71d` in manuzio, 24 October 2026, and again here on v0.1.0, 27 October 2026: only Italian
changed (lines 9.5 → 4.8, words 59.1 → 58.1; all languages 14.6 → 14.4 and 56.8), because eSpeak NG
leaves the verb "è" unstressed and jespersen now stresses it. The floors in `tests/it/golden.rs` sit
about two points under the words.

| Code | Lines | Words | Code | Lines | Words | Code | Lines | Words |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| en-us | 28.6 | 76.5 | fi | 19.0 | 51.4 | no | 4.8 | 50.7 |
| en | 19.0 | 75.8 | fr | 9.5 | 50.0 | pl | 19.0 | 66.1 |
| af | 19.0 | 66.3 | ga | 0.0 | 36.7 | pt | 9.5 | 51.1 |
| ca | 19.0 | 55.7 | hr | 33.3 | 66.2 | ro | 0.0 | 54.4 |
| cs | 23.8 | 62.5 | hu | 9.5 | 60.0 | ru | 4.8 | 35.5 |
| cy | 23.8 | 63.6 | id | 23.8 | 58.7 | sk | 14.3 | 59.3 |
| da | 9.5 | 34.9 | is | 14.3 | 57.1 | sl | 23.8 | 60.9 |
| de | 9.5 | 62.7 | it | 4.8 | 58.1 | sq | 9.5 | 59.8 |
| es | 33.3 | 67.6 | lt | 9.5 | 53.2 | sv | 9.5 | 57.4 |
| et | 19.0 | 49.5 | lv | 4.8 | 60.6 | tr | 4.8 | 34.3 |
| eu | 14.3 | 60.4 | nl | 14.3 | 57.6 | **all** | 14.4 | 56.8 |

How to read it: a word counts only when every symbol matches, so one stress mark or a slightly different
vowel fails it. Many differences are jespersen reading better (`$300,000` as "three hundred thousand
dollars", Roman numerals without eSpeak's "roman", dates as dates). The reference's Kokoro strings were
fitted with the older symbol map, which dropped ʏ, ɵ and ɫ (now kept as y, ø and l), so German, Dutch,
Swedish, Russian, Estonian, Albanian and Turkish lose a few points for sounds narration now keeps. Parity
is a regression guard, not the goal: how natural the narration sounds is checked by ear.

## Licences and notices

jespersen's code and own data are CC0. Without the `zh` feature it ships CMUdict (BSD-2-Clause: its
notice must accompany the program, so [NOTICE](../NOTICE) and `crates/kempelen/assets/speech-NOTICE.txt`
carry it in full), Moby Pronunciator II (public domain) and NST's Danish and Swedish lexicons (CC0,
credited anyway). jespersen's Mandarin data (CC BY 3.0, needing a credit) is not compiled in.
