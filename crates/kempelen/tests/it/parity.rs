//! kempelen against the references recorded from manuzio's own speech crate (docs/parity.md).

use crate::speech::data;
use kempelen::{
    Error, Language, Voice,
    phonemes::{
        JespersenPhonemizer, MandarinPhonemizer, Phonemizer, normalize_text, phonemize_with,
    },
    tts::{LoudnessTarget, Synthesizer},
};
use kempelen_parity::{
    Loudness, Machine, Output, PhonemeReferences, Produced, SynthesisCase, SynthesisReferences,
    check_phonemes, check_synthesis,
};

/// An error with its causes, as anyhow's `{:#}` writes it (how the references wrote them).
fn chain(error: &Error) -> String {
    let mut text = error.to_string();
    let mut source = std::error::Error::source(error);
    while let Some(cause) = source {
        text.push_str(": ");
        text.push_str(&cause.to_string());
        source = cause.source();
    }
    text
}

fn engine(code: &str) -> Box<dyn Phonemizer> {
    if code == "cmn" {
        Box::new(MandarinPhonemizer::new())
    } else {
        Box::new(JespersenPhonemizer::new(code).unwrap())
    }
}

#[test]
fn the_phonemes_are_those_manuzio_read() {
    let references = PhonemeReferences::committed();
    let mut engines = std::collections::HashMap::new();
    let differences = check_phonemes(&references, |code, text| {
        let engine = engines
            .entry(code.to_owned())
            .or_insert_with(|| engine(code));
        let describe = |error: Error| format!("<error: {}>", chain(&error));
        let raw = engine
            .text_to_phonemes(normalize_text(text).trim())
            .unwrap_or_else(describe);
        let kokoro = phonemize_with(&**engine, text).unwrap_or_else(describe);
        (raw.trim().to_owned(), kokoro)
    });
    assert!(
        differences.is_empty(),
        "{} of {} lines differ:\n{}",
        differences.len(),
        references.entries.len(),
        differences.join("\n")
    );
}

/// One case through kempelen, the way the harness ran it through manuzio.
pub fn produce(data: &kempelen::assets::SpeechData, case: &SynthesisCase) -> Produced {
    let run = || -> kempelen::Result<Produced> {
        let voice: Voice =
            serde_json::from_value(case.voice.clone().into()).expect("a known voice");
        let mut synthesizer = Synthesizer::new(data, voice, case.speed)?;
        if let Some(code) = &case.language {
            let language: Language =
                serde_json::from_value(code.clone().into()).expect("a known language");
            synthesizer.read_as(language)?;
        }
        match case.loudness {
            Loudness::Default => {}
            Loudness::None => synthesizer.set_loudness(None),
            Loudness::Retail => synthesizer.set_loudness(Some(LoudnessTarget::RETAIL)),
        }
        let scratch = std::env::temp_dir().join(format!("kempelen-parity-{}", std::process::id()));
        std::fs::create_dir_all(&scratch).expect("a scratch folder");
        Ok(match case.output {
            Output::Whole => Produced::Whole(synthesizer.synthesize(&case.text)?),
            Output::Chunks => Produced::Chunks(synthesizer.synthesize_chunks(&case.text)?),
            Output::Streamed => {
                let mut samples = Vec::new();
                let mut reports = Vec::new();
                let count = synthesizer.synthesize_into(
                    &case.text,
                    &scratch.join("scratch.raw"),
                    &mut |block| {
                        samples.extend_from_slice(block);
                        Ok(())
                    },
                    &mut |done, total| {
                        reports.push((done, total));
                        Ok(())
                    },
                )?;
                Produced::Streamed {
                    samples,
                    count,
                    reports,
                }
            }
            Output::Wav => {
                let path = scratch.join("out.wav");
                synthesizer.synthesize_to_wav(&case.text, &path)?;
                let bytes = std::fs::read(&path).expect("the WAV just written");
                std::fs::remove_file(&path).expect("the WAV just written");
                Produced::Wav(bytes)
            }
        })
    };
    run().unwrap_or_else(|error| Produced::Failed(chain(&error)))
}

#[test]
#[ignore = "tier:model runs real Kokoro speech synthesis"]
fn the_samples_are_those_manuzio_synthesized() {
    let references = SynthesisReferences::committed();
    let machine = Machine::current();
    if machine != references.machine {
        // Kokoro's noise depends on the thread count, and ONNX Runtime's kernels on the platform.
        eprintln!(
            "skipped: the references belong to {:?}, this is {machine:?} (docs/parity.md)",
            references.machine
        );
        return;
    }
    let data = data();
    let differences = check_synthesis(&references, |case| produce(&data, case));
    assert!(
        differences.is_empty(),
        "{} of {} syntheses differ:\n{}",
        differences.len(),
        references.cases.len(),
        differences.join("\n")
    );
}
