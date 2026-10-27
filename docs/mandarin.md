# Mandarin

Kokoro's Chinese voices (`zf_*`, `zm_*`) were trained on the phonemes of misaki's Chinese front end:
IPA-like symbols plus four tone arrows (`↓ ↗ → ↘`). eSpeak NG writes tones differently, and Kokoro drops
what it does not know, so tones disappeared when manuzio sent Chinese to eSpeak NG (before 19 October
2026). Chinese is a tonal language: without tones the narration is close to unintelligible. So Mandarin
has its own front end here, not jespersen.

## What it does

`crates/kempelen/src/phonemes/chinese/` reads Chinese the way misaki's `ZHG2P` does, in Rust, with no
Python at run time ([its README](../crates/kempelen/src/phonemes/chinese/README.md) has the details):

1. numbers become Chinese numerals (cn2an's rules: dates, fractions, percentages, temperatures, ranges);
2. Chinese punctuation becomes the marks Kokoro knows;
3. jieba-rs cuts runs of Han characters into words;
4. pypinyin's phrase dictionary (47k phrases) and character table (21k characters) give each word's
   readings, so 银行 is "háng" and 行走 is "xíng";
5. misaki's table (1,420 numbered syllables) turns each syllable into Kokoro phonemes.

Steps 4 and 5 are data generated once by `crates/kempelen/mandarin/export.py` (about 0.8 MB of text
compiled in). English words are read by jespersen as American English. The seam is the `Phonemizer`
trait (`MandarinPhonemizer`, reading code `cmn`), so punctuation, chunking and loudness are the same as
for every other language; the reading rules are the English ones, for the English words.

Traditional text (`zh-TW`, `zh-HK`) is first written in Simplified characters with OpenCC's character
table, because jieba's dictionary and pypinyin know Simplified text best. Hong Kong text is read in
Mandarin: the voices cannot speak Cantonese.

## Result

- Exact match with misaki on 444 recorded texts (`mandarin_matches_misaki`, against
  `tests/golden/mandarin-misaki.json`), so the Kokoro Chinese voices get what their own front end would
  give them.
- The phonemes of the whole Mandarin corpus are also among the references recorded from manuzio
  ([parity.md](parity.md)), and the Chinese voices among the recorded syntheses.
- Both tested voices (`zf_xiaobei`, `zm_yunjian`) synthesise with no clicks or clipping; levelling applies
  as for any voice. Not yet judged by ear: see Verification in [TODO.md](../TODO.md).

## Known limits

- No tone sandhi (misaki's v1.0 path has none); the voices may therefore sound slightly off on 3rd + 3rd
  tones.
- Kokoro's Chinese voices are the weakest of the 45 (small training set). Kokoro v1.1-zh exists with more
  voices and a newer front end; adopting it means a different model file and a different front end.
- Rare characters without a reading in pypinyin are skipped.
- Segmentation of Traditional text is weaker than of Simplified text.

## Licences

The front end is a port of misaki (Apache-2.0), with its pinyin-to-ipa table (MIT); its data comes from
pypinyin, pinyin-data and phrase-pinyin-data (MIT), cn2an's rules (MIT) and OpenCC's
`TSCharacters.txt` (Apache-2.0); jieba-rs (MIT) is a dependency. That is why kempelen as a whole is
under Apache-2.0 rather than jespersen's CC0 (owner, 27 October 2026: one repository, one crate); the
credits and the MIT notice are in [NOTICE](../NOTICE).
