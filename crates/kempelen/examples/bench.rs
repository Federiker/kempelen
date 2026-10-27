//! The speech benchmark manuzio measured narration with (its `mnz-bench speech`), on the same text:
//! the Introduction and Chapters I and II of its sample book, planned as its audiobook plans them
//! (`examples/bench/oz.json`; L. Frank Baum, public domain), read by `bf_emma` on the CPU, with
//! phonemes, model and levelling timed apart. docs/performance.md keeps the numbers.
//!
//! `cargo run --release -p kempelen --example bench` (`mise run bench`). The speech data is read
//! from `KEMPELEN_DATA_DIR`, else `tmp/speech`.

#![allow(
    clippy::cast_precision_loss,
    reason = "a development tool that measures audio"
)]

use anyhow::{Context, Result};
use kempelen::{
    Language, Voice,
    assets::SpeechData,
    phonemes::{JespersenPhonemizer, phonemize_with, reading_code},
    tts::{LoudnessTarget, SAMPLE_RATE, Synthesizer, normalize_loudness, wav_bytes},
};
use sha2::{Digest, Sha256};
use std::{
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

#[derive(serde::Deserialize)]
struct BenchTexts {
    voice: Voice,
    language: Language,
    speed: f32,
    /// The segments' texts, as the audiobook hands them to the synthesizer.
    segments: Vec<String>,
}

/// Named stage times, printed in the order they were recorded.
#[derive(Default)]
struct Stages(Vec<(&'static str, Duration)>);

impl Stages {
    fn time<T>(&mut self, name: &'static str, work: impl FnOnce() -> T) -> T {
        let start = Instant::now();
        let result = work();
        self.add(name, start.elapsed());
        result
    }

    fn add(&mut self, name: &'static str, time: Duration) {
        match self.0.iter_mut().find(|(known, _)| *known == name) {
            Some((_, total)) => *total += time,
            None => self.0.push((name, time)),
        }
    }
}

/// Highest memory the process has used so far (peak working set on Windows), in MB.
#[cfg(windows)]
#[allow(unsafe_code, non_snake_case)]
fn peak_memory_mb() -> Option<f64> {
    #[repr(C)]
    #[derive(Default)]
    struct Counters {
        cb: u32,
        PageFaultCount: u32,
        PeakWorkingSetSize: usize,
        WorkingSetSize: usize,
        QuotaPeakPagedPoolUsage: usize,
        QuotaPagedPoolUsage: usize,
        QuotaPeakNonPagedPoolUsage: usize,
        QuotaNonPagedPoolUsage: usize,
        PagefileUsage: usize,
        PeakPagefileUsage: usize,
    }
    unsafe extern "system" {
        fn GetCurrentProcess() -> isize;
        fn K32GetProcessMemoryInfo(process: isize, counters: *mut Counters, cb: u32) -> i32;
    }
    let mut counters = Counters {
        cb: u32::try_from(size_of::<Counters>()).ok()?,
        ..Default::default()
    };
    // SAFETY: the pseudo-handle of the current process needs no closing, and `counters` is a
    // properly sized, writable structure whose size is passed alongside it.
    let ok =
        unsafe { K32GetProcessMemoryInfo(GetCurrentProcess(), &raw mut counters, counters.cb) };
    (ok != 0).then(|| counters.PeakWorkingSetSize as f64 / 1_048_576.0)
}

#[cfg(not(windows))]
fn peak_memory_mb() -> Option<f64> {
    None
}

fn main() -> Result<()> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let texts: BenchTexts = serde_json::from_str(&std::fs::read_to_string(
        root.join("examples/bench/oz.json"),
    )?)?;
    let data = SpeechData::new(
        std::env::var_os("KEMPELEN_DATA_DIR")
            .map_or_else(|| root.join("../../tmp/speech"), PathBuf::from),
    );
    let passages = texts
        .segments
        .iter()
        .flat_map(|text| text.lines())
        .filter(|line| !line.trim().is_empty())
        .count();
    let threads = std::thread::available_parallelism().map_or(1, std::num::NonZero::get);
    println!(
        "speech: oz.epub, 3 chapters, {} segments, {passages} passages, {:?}, CPU, {threads} threads",
        texts.segments.len(),
        texts.voice
    );

    let mut stages = Stages::default();
    let start = Instant::now();
    let reading = reading_code(texts.voice, texts.language).context("the language is narrated")?;
    stages.time("phonemes alone", || -> Result<()> {
        let engine = JespersenPhonemizer::new(reading)?;
        for passage in texts.segments.iter().flat_map(|text| text.lines()) {
            if !passage.trim().is_empty() {
                phonemize_with(&engine, passage)?;
            }
        }
        Ok(())
    })?;
    let load = Instant::now();
    let mut synthesizer = Synthesizer::new(&data, texts.voice, texts.speed)?;
    synthesizer.read_as(texts.language)?;
    stages.add("model load", load.elapsed());

    let mut hash = Sha256::new();
    let mut seconds = 0.0;
    for text in &texts.segments {
        // What `Synthesizer::synthesize` does, a step at a time.
        let chunks = stages.time("phonemes and model", || synthesizer.synthesize_chunks(text))?;
        let mut samples = stages.time("join chunks", || chunks.concat());
        stages.time("levelling", || {
            normalize_loudness(&mut samples, LoudnessTarget::AUDIOBOOK);
        });
        let wav = stages.time("WAV encode", || wav_bytes(&samples))?;
        hash.update(format!("{:x}", Sha256::digest(&wav)).as_bytes());
        seconds += samples.len() as f64 / f64::from(SAMPLE_RATE);
    }
    let wall = start.elapsed().as_secs_f64();
    for (name, time) in &stages.0 {
        println!("  {name:<28} {:>9.2} s", time.as_secs_f64());
    }
    println!("  {:<28} {wall:>9.2} s", "total");
    match peak_memory_mb() {
        Some(mb) => println!("  {:<28} {mb:>9.0} MB", "peak memory"),
        None => println!("  peak memory: not measured on this platform"),
    }
    println!(
        "  audio                        {seconds:>9.1} s ({:.2}x real time)",
        seconds / wall
    );
    println!("  segments sha256              {:x}", hash.finalize());
    Ok(())
}
