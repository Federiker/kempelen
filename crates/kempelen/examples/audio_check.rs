//! Diagnoses clicks ("pops" and "cracks") in narration.
//!
//! `cargo run --release -p kempelen --example audio_check -- OUT_DIR [VOICE_PREFIX]`
//!
//! The speech data is read from `KEMPELEN_DATA_DIR` (mise.toml sets it), else `tmp/speech`.
//!
//! Synthesizes sample passages with the real Kokoro model (prepared assets required), then
//! reports, per voice, the level at every chunk boundary and every sample-to-sample jump that
//! stands out from its surroundings, split into jumps inside a chunk and at a join. Writes the
//! joined WAVs to `OUT_DIR` so they can be listened to.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "a development tool: it may panic, and it measures audio"
)]

use anyhow::Result;
use kempelen::{
    Voice,
    assets::SpeechData,
    tts::{Synthesizer, limit_peaks, wav_bytes},
};
use std::path::PathBuf;

const SAMPLE_RATE: f32 = 24_000.0;

/// The staged model and voices: `KEMPELEN_DATA_DIR`, else `tmp/speech`.
fn speech_data() -> SpeechData {
    SpeechData::new(std::env::var_os("KEMPELEN_DATA_DIR").map_or_else(
        || std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tmp/speech"),
        PathBuf::from,
    ))
}

const ENGLISH: &str = "The old lighthouse keeper climbed the stairs, as he did every evening, counting each step under his breath. At the top, the wind pulled at his coat, and the sea was a field of grey glass. \"Where have you been?\" she asked. \"Out walking,\" he replied, and said nothing more.\nIn 1999 he bought two houses for three hundred thousand dollars, and none of his neighbours ever understood why. Peter picked a peck of pickled peppers; where is the peck of pickled peppers Peter Piper picked? Through the thorough thought, the rough cough, though. It was a bright cold day in April, and the clocks were striking thirteen.";

const ITALIAN: &str = "Il vecchio guardiano del faro saliva le scale, come ogni sera, contando ogni gradino sottovoce. In cima, il vento tirava il suo cappotto, e il mare era un campo di vetro grigio. «Dove sei stato?» chiese lei. «A passeggiare», rispose lui, e non disse altro.\nNel 1999 comprò due case per trecentomila euro, e nessuno dei suoi vicini capì mai il perché. Trentatré trentini entrarono a Trento tutti e trentatré trotterellando.";

const CHINESE_LONG: &str = "老灯塔守每天晚上都要爬上楼梯，一边走，一边在心里数着每一级台阶。到了顶上，风吹着他的外套，大海像一片灰色的玻璃。“你去哪儿了？”她问。“出去走了走，”他回答，然后再也没有说什么。
1999年，他花了三十万元买了两座房子，邻居们谁也不明白为什么。银行里的钱不够，他只好去行政部门问问：这个重要的会议将在重庆举行，共有120人参加，百分之五十的人同意这个计划。他每天用Python写代码，周末去Google开会。";

struct Stats {
    chunk_edges: Vec<(f32, f32, f32)>,
    in_chunk_jumps: Vec<(f32, f32)>,
    join_jumps: Vec<(f32, f32)>,
    clipped: usize,
    peak: f32,
}

/// Short broadband bursts: the second difference spikes far above its level 10 ms around,
/// which voiced speech and sustained fricatives (high everywhere nearby) do not do.
fn clicks(samples: &[f32]) -> Vec<(f32, f32)> {
    let second: Vec<f32> = (2..samples.len())
        .map(|i| (samples[i] - 2.0 * samples[i - 1] + samples[i - 2]).abs())
        .collect();
    let mut found: Vec<(f32, f32)> = Vec::new();
    let mut i = 240;
    while i + 240 < second.len() {
        let mut window: Vec<f32> = second[i - 240..i + 240].to_vec();
        window.sort_by(f32::total_cmp);
        let baseline = window[window.len() * 3 / 4].max(0.001);
        let value = second[i];
        if value > 0.06 && value > 30.0 * baseline {
            let time = (i + 1) as f32 / SAMPLE_RATE;
            match found.last_mut() {
                Some(last) if time - last.0 < 0.005 => last.1 = last.1.max(value),
                _ => found.push((time, value)),
            }
        }
        i += 1;
    }
    found
}

fn analyse(chunks: &[Vec<f32>]) -> Stats {
    let joined: Vec<f32> = chunks.concat();
    let mut boundaries = Vec::new();
    let mut at = 0;
    for chunk in chunks {
        at += chunk.len();
        boundaries.push(at as f32 / SAMPLE_RATE);
    }
    let mut stats = Stats {
        chunk_edges: Vec::new(),
        in_chunk_jumps: Vec::new(),
        join_jumps: Vec::new(),
        clipped: joined.iter().filter(|s| s.abs() >= 0.99).count(),
        peak: joined.iter().fold(0.0, |m, s| m.max(s.abs())),
    };
    for chunk in chunks {
        if let (Some(first), Some(last)) = (chunk.first(), chunk.last()) {
            let peak = chunk.iter().fold(0.0f32, |m, s| m.max(s.abs()));
            stats.chunk_edges.push((first.abs(), last.abs(), peak));
        }
    }
    for (time, value) in clicks(&joined) {
        if boundaries.iter().any(|b| (b - time).abs() < 0.01) {
            stats.join_jumps.push((time, value));
        } else {
            stats.in_chunk_jumps.push((time, value));
        }
    }
    stats
}

const SPANISH: &str = "El viejo farero subía las escaleras, como cada noche, contando cada escalón en voz baja. Arriba, el viento tiraba de su abrigo, y el mar era un campo de cristal gris. «¿Dónde has estado?» preguntó ella. «Paseando», respondió él, y no dijo nada más.";
const FRENCH: &str = "Le vieux gardien du phare montait l'escalier, comme chaque soir, en comptant chaque marche à voix basse. En haut, le vent tirait sur son manteau, et la mer était un champ de verre gris. « Où étais-tu ? » demanda-t-elle. « Je me promenais », répondit-il, et il n'ajouta rien.";
const PORTUGUESE: &str = "O velho faroleiro subia as escadas, como todas as noites, contando cada degrau em voz baixa. No alto, o vento puxava o seu casaco, e o mar era um campo de vidro cinzento. — Onde estiveste? — perguntou ela. — A passear — respondeu ele, e não disse mais nada.";
const CHINESE: &str = "老灯塔守每天晚上都要爬上楼梯，一边低声数着每一级台阶。顶上的风拉扯着他的外套，大海像一片灰色的玻璃。“你去哪里了？”她问道。“出去散步。”他回答，然后不再说话。";

/// `audio_check all [SPEED]`: every shipped voice reading a passage in its own language, with
/// the level and click statistics of the audio the audiobook would receive.
fn all_voices(speed: f32) -> Result<()> {
    #[derive(serde::Deserialize)]
    struct Manifest {
        files: Vec<Asset>,
    }
    #[derive(serde::Deserialize)]
    struct Asset {
        path: String,
    }
    let manifest: Manifest = serde_json::from_str(kempelen::assets::MANIFEST)?;
    let data = speech_data();
    println!("speed {speed}");
    println!(
        "voice        raw_peak  clipped  peak_out  speech_rms  overall_rms  clicks  seconds  acx"
    );
    for name in manifest.files.iter().filter_map(|f| {
        f.path
            .strip_prefix("voices/")
            .and_then(|v| v.strip_suffix(".bin"))
    }) {
        let voice: Voice = serde_json::from_value(serde_json::Value::String(name.into()))?;
        let text = match &name[..1] {
            "z" => CHINESE,
            "e" => SPANISH,
            "f" => FRENCH,
            "p" => PORTUGUESE,
            "i" => ITALIAN,
            _ => ENGLISH,
        };
        let mut synth = Synthesizer::new(&data, voice, speed)?;
        let raw: Vec<f32> = synth.synthesize_chunks(text)?.concat();
        let raw_peak = raw.iter().fold(0.0f32, |m, s| m.max(s.abs()));
        let clipped = raw.iter().filter(|s| s.abs() >= 0.99).count();
        let limited = synth.synthesize(text)?;
        let peak_out = limited.iter().fold(0.0f32, |m, s| m.max(s.abs()));
        let overall = 10.0
            * (limited.iter().map(|s| s * s).sum::<f32>() / limited.len() as f32)
                .max(1e-12)
                .log10();
        let speech = kempelen::tts::speech_rms_dbfs(&limited).unwrap_or(f32::NAN);
        // ACX/Audible: overall RMS -23 to -18 dBFS, peaks no higher than -3 dBFS.
        let acx = (-23.0..=-18.0).contains(&overall) && peak_out <= 0.7080;
        println!(
            "{name:<12} {raw_peak:>8.3} {clipped:>8} {peak_out:>9.3} {speech:>11.1} {overall:>12.1} {:>7} {:>8.1}  {}",
            clicks(&limited).len(),
            limited.len() as f32 / SAMPLE_RATE,
            if acx { "ok" } else { "NO" }
        );
    }
    Ok(())
}

/// `audio_check analyze FILE.f32le`: statistics of any decoded audio (mono, 24 kHz, 32-bit float).
fn analyze_file(path: &str) -> Result<()> {
    let bytes = std::fs::read(path)?;
    let samples: Vec<f32> = bytes
        .chunks_exact(4)
        .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .collect();
    let found = clicks(&samples);
    println!(
        "{path}: {:.1}s peak {:.3} samples>=0.99 {} clicks {}",
        samples.len() as f32 / SAMPLE_RATE,
        samples.iter().fold(0.0f32, |m, s| m.max(s.abs())),
        samples.iter().filter(|s| s.abs() >= 0.99).count(),
        found.len()
    );
    Ok(())
}

fn main() -> Result<()> {
    if std::env::args().nth(1).as_deref() == Some("all") {
        let speed = std::env::args()
            .nth(2)
            .and_then(|v| v.parse().ok())
            .unwrap_or(0.9);
        return all_voices(speed);
    }
    if std::env::args().nth(1).as_deref() == Some("analyze") {
        return analyze_file(
            &std::env::args()
                .nth(2)
                .expect("usage: audio_check analyze FILE"),
        );
    }
    let out = PathBuf::from(std::env::args().nth(1).expect("usage: audio_check OUT_DIR"));
    std::fs::create_dir_all(&out)?;
    let data = speech_data();
    for (voice, text) in [
        (Voice::AfHeart, ENGLISH),
        (Voice::BfEmma, ENGLISH),
        (Voice::IfSara, ITALIAN),
        (Voice::ZfXiaobei, CHINESE_LONG),
        (Voice::ZmYunjian, CHINESE_LONG),
    ] {
        let only = std::env::args().nth(2);
        if only.is_some_and(|prefix| !format!("{voice:?}").starts_with(&prefix)) {
            continue;
        }
        let mut synth = Synthesizer::new(&data, voice, 1.0)?;
        let chunks = synth.synthesize_chunks(text)?;
        let stats = analyse(&chunks);
        println!(
            "== {voice:?}: {} chunks, {:.1}s, peak {:.3}, samples >= 0.99: {}",
            chunks.len(),
            chunks.iter().map(Vec::len).sum::<usize>() as f32 / SAMPLE_RATE,
            stats.peak,
            stats.clipped
        );
        for (index, (first, last, peak)) in stats.chunk_edges.iter().enumerate() {
            println!(
                "  chunk {index:>2}: first {first:.4}  last {last:.4}  peak {peak:.3}  len {:.2}s",
                chunks[index].len() as f32 / SAMPLE_RATE
            );
        }
        println!("  clicks inside chunks: {}", stats.in_chunk_jumps.len());
        let joined = chunks.concat();
        for (time, jump) in stats.in_chunk_jumps.iter().take(8) {
            let at = (time * SAMPLE_RATE) as usize;
            let around: Vec<String> = joined[at.saturating_sub(8)..(at + 8).min(joined.len())]
                .iter()
                .map(|s| format!("{s:+.3}"))
                .collect();
            println!("    {time:>7.3}s  d2 {jump:.3}  {}", around.join(" "));
        }
        println!("  clicks at joins: {}", stats.join_jumps.len());
        for (time, jump) in &stats.join_jumps {
            println!("    {time:>7.3}s  step {jump:.3}");
        }
        let mut limited = chunks.concat();
        limit_peaks(&mut limited);
        println!(
            "  after the limiter: peak {:.3}, samples >= 0.99: {}, clicks {}",
            limited.iter().fold(0.0f32, |m, s| m.max(s.abs())),
            limited.iter().filter(|s| s.abs() >= 0.99).count(),
            clicks(&limited).len()
        );
        std::fs::write(out.join(format!("{voice:?}.wav")), wav_bytes(&limited)?)?;
    }
    Ok(())
}
