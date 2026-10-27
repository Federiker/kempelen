//! Mandarin pronunciation for Kokoro's Chinese voices.
//!
//! Kokoro's Chinese voices were trained on the phonemes of misaki's Chinese front end, not on
//! eSpeak NG's notation, so Chinese is pronounced here rather than by jespersen. The steps are
//! those of misaki's `ZHG2P`: numbers become Chinese numerals, punctuation is made Kokoro-sized,
//! jieba splits runs of Han characters into words, pypinyin's dictionaries give each word's
//! readings (phrases first, so that 银行 is "háng" and 行走 is "xíng") and misaki's table turns
//! each syllable into Kokoro's phonemes with tone arrows. The data files in `mandarin/`
//! are generated from those projects by `export.py`; the tests compare the result with
//! misaki's own output. See the README next to this file for licences.

mod numbers;
mod phonemizer;
mod reader;

pub use phonemizer::MandarinPhonemizer;
