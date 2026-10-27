use super::Voice;

#[test]
fn display_preserves_voice_provider_identifiers() {
    for (voice, code) in [
        (Voice::AfHeart, "af_heart"),
        (Voice::ZfXiaoxiao, "zf_xiaoxiao"),
        (Voice::ImNicola, "im_nicola"),
        (Voice::PmSanta, "pm_santa"),
    ] {
        assert_eq!(voice.to_string(), code);
        assert_eq!(format!("{voice}"), code);
        assert_eq!(serde_json::to_value(voice).unwrap(), code);
    }
}

/// Applications store these names (manuzio's saved projects do): they must never change.
#[test]
fn the_serialised_names_never_change() {
    let names: Vec<&str> = Voice::ALL.iter().map(|voice| voice.name()).collect();
    assert_eq!(
        names,
        [
            "af_heart",
            "af_alloy",
            "af_aoede",
            "af_bella",
            "af_jessica",
            "af_kore",
            "af_nicole",
            "af_nova",
            "af_river",
            "af_sarah",
            "af_sky",
            "am_adam",
            "am_echo",
            "am_eric",
            "am_fenrir",
            "am_liam",
            "am_michael",
            "am_onyx",
            "am_puck",
            "am_santa",
            "bf_alice",
            "bf_emma",
            "bf_isabella",
            "bf_lily",
            "bm_daniel",
            "bm_fable",
            "bm_george",
            "bm_lewis",
            "zf_xiaobei",
            "zf_xiaoni",
            "zf_xiaoxiao",
            "zf_xiaoyi",
            "zm_yunjian",
            "zm_yunxi",
            "zm_yunxia",
            "zm_yunyang",
            "ef_dora",
            "em_alex",
            "em_santa",
            "ff_siwis",
            "if_sara",
            "im_nicola",
            "pf_dora",
            "pm_alex",
            "pm_santa",
        ]
    );
    for voice in Voice::ALL {
        let value = serde_json::to_value(voice).unwrap();
        assert_eq!(value, voice.name());
        assert_eq!(serde_json::from_value::<Voice>(value).unwrap(), voice);
    }
}

#[test]
fn the_removed_hindi_voices_read_as_the_default_voice() {
    assert_eq!(Voice::default(), Voice::BfAlice);
    for name in ["hf_alpha", "hf_beta", "hm_omega", "hm_psi"] {
        let voice: Voice = serde_json::from_value(name.into()).unwrap();
        assert_eq!(voice, Voice::BfAlice, "{name}");
    }
}
