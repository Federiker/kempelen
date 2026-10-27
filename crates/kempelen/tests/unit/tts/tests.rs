use super::*;

#[test]
fn rejects_non_positive_speed_before_loading_assets() {
    let out = std::env::temp_dir().join(format!("kempelen-speed-{}.wav", std::process::id()));
    // No speech data at all: the speed is refused before anything is looked for.
    let data = SpeechData::new(std::env::temp_dir().join("kempelen-no-speech-data"));
    for speed in [0.0, -1.0, f32::NAN, f32::INFINITY] {
        let result = tts_with_options(&data, "Hello", Voice::AfAlloy, &out, TtsOptions { speed });

        // The speed is what is refused (missing or present assets would fail otherwise or not
        // at all), and nothing is written.
        let error = result.expect_err("an invalid speed is refused");
        assert!(
            matches!(error, Error::InvalidSpeed(_)),
            "speed {speed:?}: {error}"
        );
        assert!(
            error
                .to_string()
                .contains("speed must be a finite value greater than zero"),
            "speed {speed:?}: {error}"
        );
        assert!(!out.exists(), "speed {speed:?}: a WAV was written");
    }
}

#[test]
fn missing_speech_data_is_an_asset_error_naming_the_file() {
    let data = SpeechData::new(std::env::temp_dir().join("kempelen-no-speech-data"));
    let Err(error) = Synthesizer::new(&data, Voice::AfAlloy, 1.0) else {
        panic!("a synthesizer without speech data");
    };
    assert!(
        matches!(&error, Error::Asset { path, problem: crate::AssetProblem::Missing } if path == "models/model.onnx"),
        "{error}"
    );
}
