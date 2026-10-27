//! Model-backed synthesis checks. They require prepared assets and never touch
//! the network.

use crate::speech::data;
use kempelen::{Voice, tts::Synthesizer};

#[test]
#[ignore = "tier:model runs real Kokoro speech synthesis"]
fn punctuation_slows_the_reading_down() {
    let data = data();
    let mut synth = Synthesizer::new(&data, Voice::AfAlloy, 1.0).unwrap();
    // Kokoro marks a sentence end mostly by slowing down and lengthening the last word, not with a
    // clear silence (measured 24 October 2026: "One. Two. Three. Four. Five." is about 175 ms
    // longer, with no new silent gap). So the check is the length, with four sentence ends.
    let plain = synth.synthesize("One two three four five").unwrap();
    let punctuated = synth.synthesize("One. Two. Three. Four. Five.").unwrap();
    assert!(
        punctuated.len() >= plain.len() + 2_400,
        "{} samples with full stops, {} without: less than 0.1 s longer",
        punctuated.len(),
        plain.len()
    );
}

#[test]
#[ignore = "tier:model synthesizes long text with the real Kokoro model"]
fn text_longer_than_the_model_limit_is_synthesized_in_chunks() {
    let data = data();
    let sentence = "The quick brown fox jumps over the lazy dog, and then it rests. ";
    let mut synth = Synthesizer::new(&data, Voice::AfAlloy, 1.0).unwrap();
    let one = synth.synthesize(sentence).unwrap();
    let many = synth.synthesize(&sentence.repeat(20)).unwrap();
    assert!(
        many.len() > one.len() * 15,
        "{} vs {}",
        many.len(),
        one.len()
    );
}

#[test]
#[ignore = "tier:model initializes Kokoro and validates no-speech output"]
fn text_without_speech_is_an_error() {
    let data = data();
    let mut synth = Synthesizer::new(&data, Voice::AfAlloy, 1.0).unwrap();
    assert!(synth.synthesize("  \n ").is_err());
}

#[test]
#[ignore = "tier:model runs real Kokoro speech synthesis"]
fn a_voice_reads_a_book_language_it_was_not_trained_on() {
    let data = data();
    let mut synth = Synthesizer::new(&data, Voice::IfSara, 1.0).unwrap();
    synth.read_as(kempelen::Language::German).unwrap();
    let german = synth
        .synthesize("Der alte Leuchtturmwärter stieg die Treppe hinauf.")
        .unwrap();
    assert!(german.len() > 24_000, "{} samples", german.len());
    // The book's language decides the pronunciation: the same voice reading the same words as
    // Italian says something else.
    synth.read_as(kempelen::Language::Italian).unwrap();
    let italian = synth
        .synthesize("Der alte Leuchtturmwärter stieg die Treppe hinauf.")
        .unwrap();
    let difference = german.len().abs_diff(italian.len()) as f64 / german.len() as f64;
    assert!(
        german != italian && difference > 0.03,
        "German and Italian readings are alike ({} and {} samples)",
        german.len(),
        italian.len()
    );
    assert!(synth.read_as(kempelen::Language::Japanese).is_err());
}

#[test]
#[ignore = "tier:model runs real Kokoro speech synthesis"]
fn loud_voices_are_limited_instead_of_clipped() {
    let data = data();
    let mut synth = Synthesizer::new(&data, Voice::IfSara, 1.0).unwrap();
    // The raw model output of this voice peaks well above full scale on this text (about 1.3).
    let text = "Il vecchio guardiano del faro saliva le scale, come ogni sera, contando ogni gradino sottovoce. In cima, il vento tirava il suo cappotto, e il mare era un campo di vetro grigio.";
    let raw: Vec<f32> = synth.synthesize_chunks(text).unwrap().concat();
    let raw_peak = raw.iter().fold(0.0f32, |peak, s| peak.max(s.abs()));
    let samples = synth.synthesize(text).unwrap();
    let peak = samples.iter().fold(0.0f32, |peak, s| peak.max(s.abs()));
    assert!(raw_peak > 1.0, "the premise changed: raw peak {raw_peak}");
    assert!(peak <= kempelen::tts::CEILING + 1e-6, "peak {peak}");
    assert!(peak > 0.5, "the speech must stay audible, peak {peak}");
}

#[test]
#[ignore = "tier:model runs real Kokoro speech synthesis"]
fn a_quiet_and_a_loud_voice_speak_at_the_same_level() {
    let data = data();
    // af_nova is among the quietest voices and if_sara the loudest: about 12 dB apart when raw.
    let text = "The old lighthouse keeper climbed the stairs, as he did every evening, counting each step under his breath. At the top, the wind pulled at his coat.";
    let mut levels = Vec::new();
    for voice in [Voice::AfNova, Voice::IfSara] {
        let mut synth = Synthesizer::new(&data, voice, 1.0).unwrap();
        let normalized = synth.synthesize(text).unwrap();
        synth.set_loudness(None);
        let raw_level = kempelen::tts::speech_rms_dbfs(&synth.synthesize(text).unwrap()).unwrap();
        let level = kempelen::tts::speech_rms_dbfs(&normalized).unwrap();
        let peak = normalized.iter().fold(0.0f32, |p, s| p.max(s.abs()));
        assert!(peak <= 0.708, "{voice:?} peak {peak}");
        levels.push((level, raw_level));
    }
    let [(nova, nova_raw), (sara, sara_raw)] = levels[..] else {
        unreachable!()
    };
    assert!(
        (sara_raw - nova_raw).abs() > 6.0,
        "the premise changed: raw levels {nova_raw} and {sara_raw}"
    );
    assert!((nova - sara).abs() < 1.5, "levels {nova} and {sara}");
    assert!((nova + 20.0).abs() < 1.5, "level {nova}");
}

#[test]
#[ignore = "tier:model runs real Kokoro speech synthesis"]
fn streamed_synthesis_gives_exactly_the_samples_of_the_whole_buffer() {
    let data = data();
    // Two passages and more than one levelling block, from a voice whose peaks need limiting.
    let text = "Il vecchio guardiano del faro saliva le scale, come ogni sera, contando ogni gradino sottovoce. In cima, il vento tirava il suo cappotto, e il mare era un campo di vetro grigio.\nPoi, quando la luce si accese, guardò lontano, verso le navi che tornavano al porto, e sorrise.";
    let dir = std::env::temp_dir().join(format!("kempelen-stream-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    for loudness in [Some(kempelen::tts::LoudnessTarget::AUDIOBOOK), None] {
        let mut synth = Synthesizer::new(&data, Voice::IfSara, 1.0).unwrap();
        synth.read_as(kempelen::Language::Italian).unwrap();
        synth.set_loudness(loudness);
        let whole = synth.synthesize(text).unwrap();
        assert!(whole.len() > 65_536, "too short to need two blocks");
        let mut streamed = Vec::new();
        let scratch = dir.join("scratch.raw");
        let mut reports = Vec::new();
        let count = synth
            .synthesize_into(
                text,
                &scratch,
                &mut |samples| {
                    streamed.extend_from_slice(samples);
                    Ok(())
                },
                &mut |done, total| {
                    reports.push((done, total));
                    Ok(())
                },
            )
            .unwrap();
        assert_eq!(count, whole.len());
        // One report before each chunk, at least one per passage.
        assert!(reports.len() >= 2, "{reports:?}");
        assert_eq!(reports[0].0, 0);
        assert!(
            reports
                .windows(2)
                .all(|pair| pair[0].0 < pair[1].0 && pair[1].0 < pair[1].1)
        );
        assert!(streamed == whole, "streamed samples differ ({loudness:?})");
        assert!(!scratch.exists(), "the scratch file is removed");
        let wav = dir.join("whole.wav");
        synth.synthesize_to_wav(text, &wav).unwrap();
        assert!(std::fs::read(&wav).unwrap() == kempelen::tts::wav_bytes(&whole).unwrap());
    }
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
#[ignore = "tier:model runs real Kokoro speech synthesis"]
fn an_error_before_a_chunk_stops_the_synthesis_with_it() {
    let data = data();
    let text = "The old lighthouse keeper climbed the stairs, as he did every evening, counting each step under his breath. At the top, the wind pulled at his coat, and the sea was a field of grey glass. Far away, the boats came home.
Then he lit the lamp.";
    let mut synth = Synthesizer::new(&data, Voice::AfAlloy, 1.0).unwrap();
    let scratch = std::env::temp_dir().join(format!("kempelen-stop-{}.raw", std::process::id()));
    let mut chunks = 0;
    let mut sunk = 0;
    let error = synth
        .synthesize_into(
            text,
            &scratch,
            &mut |samples| {
                sunk += samples.len();
                Ok(())
            },
            &mut |done, _| {
                chunks += 1;
                if done > 0 {
                    return Err(kempelen::Error::interrupted("stopped"));
                }
                Ok(())
            },
        )
        .unwrap_err();
    assert_eq!(error.to_string(), "stopped");
    assert_eq!(chunks, 2, "stops before the second chunk");
    assert_eq!(sunk, 0, "nothing is handed on");
    assert!(!scratch.exists());
}

/// CUDA output is not compared with the CPU's (it differs from run to run); what must hold is that
/// a machine without the runtime gets the CPU's own samples by falling back.
#[test]
#[ignore = "tier:model runs real Kokoro speech synthesis"]
fn without_the_gpu_runtime_narration_falls_back_to_the_same_cpu_samples() {
    let data = data();
    let missing = std::env::temp_dir().join(format!("kempelen-no-cuda-{}", std::process::id()));
    let error = Synthesizer::with_cuda(&data, Voice::AfAlloy, 1.0, &missing)
        .err()
        .expect("no runtime is installed there");
    assert!(matches!(error, kempelen::Error::Cuda(_)), "{error}");
    let text = "One two three four five";
    let mut fallback = Synthesizer::with_cuda(&data, Voice::AfAlloy, 1.0, &missing)
        .or_else(|_| Synthesizer::new(&data, Voice::AfAlloy, 1.0))
        .unwrap();
    assert_eq!(fallback.device(), kempelen::tts::Device::Cpu);
    let mut cpu = Synthesizer::new(&data, Voice::AfAlloy, 1.0).unwrap();
    assert!(fallback.synthesize(text).unwrap() == cpu.synthesize(text).unwrap());
}
