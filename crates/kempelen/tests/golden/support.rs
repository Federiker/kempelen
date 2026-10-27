//! Shared by `tests/it/golden.rs` and `examples/golden.rs`: corpus loading, capture and parity scoring.
#![allow(dead_code, clippy::cast_precision_loss, clippy::format_push_string)]

use anyhow::{Context, Result};
use kempelen::phonemes::{JespersenPhonemizer, Phonemizer, normalize_text, phonemize_with};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
    sync::Mutex,
};

pub const ESPEAK_UPSTREAM: &str = "espeak-upstream";
pub const JESPERSEN: &str = "jespersen";

/// Mandarin's corpus language, read by kempelen's own front end and measured against misaki
/// (`mandarin-misaki.json`), not against eSpeak NG.
pub const MANDARIN: &str = "cmn";

#[derive(Deserialize)]
pub struct Corpus {
    pub universal: Vec<String>,
    pub languages: BTreeMap<String, Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    pub text: String,
    /// The engine's output for the whole normalized line, before punctuation handling.
    pub raw: String,
    /// What narration feeds Kokoro (`phonemize_with`).
    pub kokoro: String,
}

/// Golden output of one engine: reading code -> entries in corpus order.
pub type Golden = BTreeMap<String, Vec<Entry>>;

pub fn golden_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden")
}

/// Where an engine's capture is kept: `tmp/golden`, gitignored. The eSpeak NG reference is never
/// committed (docs/jespersen.md): it is captured there with `golden capture-upstream`.
pub fn golden_path(engine: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tmp/golden")
        .join(format!("{engine}.json"))
}

/// The eSpeak NG reference, when it has been captured into `tmp/golden`.
pub fn upstream_reference() -> Option<Golden> {
    golden_path(ESPEAK_UPSTREAM)
        .exists()
        .then(|| load_golden(ESPEAK_UPSTREAM).ok())
        .flatten()
}

pub fn load_corpus() -> Result<Corpus> {
    let path = golden_dir().join("corpus.json");
    let text = std::fs::read_to_string(&path).with_context(|| path.display().to_string())?;
    Ok(serde_json::from_str(&text)?)
}

pub fn lines_for(corpus: &Corpus, code: &str) -> Vec<String> {
    let mut lines = corpus.languages.get(code).cloned().unwrap_or_default();
    lines.extend(corpus.universal.iter().cloned());
    lines
}

pub fn capture_line(engine: &dyn Phonemizer, text: &str) -> Entry {
    let describe = |error: kempelen::Error| format!("<error: {error}>");
    let raw = engine
        .text_to_phonemes(normalize_text(text).trim())
        .unwrap_or_else(describe);
    let kokoro = phonemize_with(engine, text).unwrap_or_else(describe);
    Entry {
        text: text.to_owned(),
        raw: raw.trim().to_owned(),
        kokoro,
    }
}

/// Capture through jespersen, what narration reads every language but Mandarin with.
pub fn capture_jespersen(corpus: &Corpus) -> Result<Golden> {
    let mut golden = Golden::new();
    for code in corpus.languages.keys().filter(|code| *code != MANDARIN) {
        let engine = JespersenPhonemizer::new(code)?;
        let entries = lines_for(corpus, code)
            .iter()
            .map(|line| capture_line(&engine, line))
            .collect();
        golden.insert(code.clone(), entries);
    }
    Ok(golden)
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Parity {
    pub lines: usize,
    pub exact_lines: usize,
    pub words: usize,
    pub matched_words: usize,
}

impl Parity {
    pub fn line_percent(&self) -> f64 {
        percent(self.exact_lines, self.lines)
    }
    pub fn word_percent(&self) -> f64 {
        percent(self.matched_words, self.words)
    }
}

fn percent(part: usize, whole: usize) -> f64 {
    if whole == 0 {
        100.0
    } else {
        part as f64 * 100.0 / whole as f64
    }
}

/// Score `actual` against `expected` on the Kokoro-fitted strings: exact lines, and
/// words in the longest common subsequence (robust to one word shifting the rest).
pub fn score(expected: &[Entry], actual: &[Entry]) -> Parity {
    let mut parity = Parity::default();
    for (want, got) in expected.iter().zip(actual) {
        let want_words: Vec<&str> = want.kokoro.split_whitespace().collect();
        let got_words: Vec<&str> = got.kokoro.split_whitespace().collect();
        parity.lines += 1;
        parity.exact_lines += usize::from(want.kokoro == got.kokoro);
        parity.words += want_words.len();
        parity.matched_words += lcs(&want_words, &got_words);
    }
    parity
}

fn lcs(a: &[&str], b: &[&str]) -> usize {
    let mut row = vec![0usize; b.len() + 1];
    for x in a {
        let mut diagonal = 0;
        for (j, y) in b.iter().enumerate() {
            let above = row[j + 1];
            row[j + 1] = if x == y {
                diagonal + 1
            } else {
                above.max(row[j])
            };
            diagonal = above;
        }
    }
    row[b.len()]
}

/// Upstream eSpeak NG through `upstream_espeak.py` (development reference only).
pub struct UpstreamEspeak {
    child: Child,
    pipes: Mutex<(ChildStdin, BufReader<ChildStdout>)>,
    pub version: String,
    language: String,
}

impl UpstreamEspeak {
    /// Start the helper for `language`. `python` must have `espeakng-loader` installed,
    /// or `library` and `data_parent` must name an eSpeak NG library and its data directory.
    pub fn start(python: &Path, language: &str, library: Option<(&Path, &Path)>) -> Result<Self> {
        let mut command = Command::new(python);
        command
            .arg("-I")
            .arg(golden_dir().join("upstream_espeak.py"))
            .arg(language);
        if let Some((library, data_parent)) = library {
            command.arg(library).arg(data_parent);
        }
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .context("cannot start the upstream eSpeak helper")?;
        let stdin = child.stdin.take().context("helper has no stdin")?;
        let mut stdout = BufReader::new(child.stdout.take().context("helper has no stdout")?);
        let mut banner = String::new();
        stdout.read_line(&mut banner)?;
        let version = serde_json::from_str(banner.trim())
            .with_context(|| format!("eSpeak voice {language:?} did not start"))?;
        Ok(Self {
            child,
            pipes: Mutex::new((stdin, stdout)),
            version,
            language: language.to_owned(),
        })
    }
}

impl Phonemizer for UpstreamEspeak {
    fn engine(&self) -> &'static str {
        ESPEAK_UPSTREAM
    }

    fn language(&self) -> &str {
        &self.language
    }

    fn text_to_phonemes(&self, words: &str) -> kempelen::Result<String> {
        self.ask(words)
            .map_err(|error| kempelen::Error::Phonemizer(error.into()))
    }
}

impl UpstreamEspeak {
    fn ask(&self, words: &str) -> Result<String> {
        let mut pipes = self
            .pipes
            .lock()
            .map_err(|_| anyhow::anyhow!("helper lock poisoned"))?;
        let (stdin, stdout) = &mut *pipes;
        writeln!(stdin, "{}", serde_json::to_string(words)?)?;
        stdin.flush()?;
        let mut line = String::new();
        anyhow::ensure!(
            stdout.read_line(&mut line)? > 0,
            "the eSpeak helper stopped"
        );
        Ok(serde_json::from_str(line.trim())?)
    }
}

impl Drop for UpstreamEspeak {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

pub fn capture_upstream(
    corpus: &Corpus,
    python: &Path,
    library: Option<(&Path, &Path)>,
) -> Result<(Golden, String)> {
    let mut golden = Golden::new();
    let mut version = String::new();
    for code in corpus.languages.keys() {
        let engine = UpstreamEspeak::start(python, code, library)?;
        version.clone_from(&engine.version);
        let entries = lines_for(corpus, code)
            .iter()
            .map(|line| capture_line(&engine, line))
            .collect();
        golden.insert(code.clone(), entries);
    }
    Ok((golden, version))
}

pub fn load_golden(engine: &str) -> Result<Golden> {
    let path = golden_path(engine);
    let text = std::fs::read_to_string(&path).with_context(|| path.display().to_string())?;
    Ok(serde_json::from_str(&text)?)
}

/// One row per language: how closely `actual` reproduces `expected`.
pub fn report(expected: &Golden, actual: &Golden) -> String {
    let mut out = format!(
        "{:<6} {:>6} {:>6}
",
        "code", "lines%", "words%"
    );
    let mut total = Parity::default();
    for (code, want) in expected {
        let Some(got) = actual.get(code) else {
            continue;
        };
        let parity = score(want, got);
        out += &format!(
            "{code:<6} {:>6.1} {:>6.1}
",
            parity.line_percent(),
            parity.word_percent()
        );
        total.lines += parity.lines;
        total.exact_lines += parity.exact_lines;
        total.words += parity.words;
        total.matched_words += parity.matched_words;
    }
    out += &format!(
        "{:<6} {:>6.1} {:>6.1}
",
        "all",
        total.line_percent(),
        total.word_percent()
    );
    out
}
