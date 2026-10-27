use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
/// A Kokoro v1.0 voice. Its serialised name (`af_heart`) is the voice file's name and never changes:
/// applications store it.
pub enum Voice {
    #[serde(rename = "af_heart")]
    AfHeart,
    #[serde(rename = "af_alloy")]
    AfAlloy,
    #[serde(rename = "af_aoede")]
    AfAoede,
    #[serde(rename = "af_bella")]
    AfBella,
    #[serde(rename = "af_jessica")]
    AfJessica,
    #[serde(rename = "af_kore")]
    AfKore,
    #[serde(rename = "af_nicole")]
    AfNicole,
    #[serde(rename = "af_nova")]
    AfNova,
    #[serde(rename = "af_river")]
    AfRiver,
    #[serde(rename = "af_sarah")]
    AfSarah,
    #[serde(rename = "af_sky")]
    AfSky,
    #[serde(rename = "am_adam")]
    AmAdam,
    #[serde(rename = "am_echo")]
    AmEcho,
    #[serde(rename = "am_eric")]
    AmEric,
    #[serde(rename = "am_fenrir")]
    AmFenrir,
    #[serde(rename = "am_liam")]
    AmLiam,
    #[serde(rename = "am_michael")]
    AmMichael,
    #[serde(rename = "am_onyx")]
    AmOnyx,
    #[serde(rename = "am_puck")]
    AmPuck,
    #[serde(rename = "am_santa")]
    AmSanta,
    // The Hindi voices were removed; projects that still name one fall back to the default voice.
    #[serde(
        rename = "bf_alice",
        alias = "hf_alpha",
        alias = "hf_beta",
        alias = "hm_omega",
        alias = "hm_psi"
    )]
    #[default]
    BfAlice,
    #[serde(rename = "bf_emma")]
    BfEmma,
    #[serde(rename = "bf_isabella")]
    BfIsabella,
    #[serde(rename = "bf_lily")]
    BfLily,
    #[serde(rename = "bm_daniel")]
    BmDaniel,
    #[serde(rename = "bm_fable")]
    BmFable,
    #[serde(rename = "bm_george")]
    BmGeorge,
    #[serde(rename = "bm_lewis")]
    BmLewis,
    #[serde(rename = "zf_xiaobei")]
    ZfXiaobei,
    #[serde(rename = "zf_xiaoni")]
    ZfXiaoni,
    #[serde(rename = "zf_xiaoxiao")]
    ZfXiaoxiao,
    #[serde(rename = "zf_xiaoyi")]
    ZfXiaoyi,
    #[serde(rename = "zm_yunjian")]
    ZmYunjian,
    #[serde(rename = "zm_yunxi")]
    ZmYunxi,
    #[serde(rename = "zm_yunxia")]
    ZmYunxia,
    #[serde(rename = "zm_yunyang")]
    ZmYunyang,
    #[serde(rename = "ef_dora")]
    EfDora,
    #[serde(rename = "em_alex")]
    EmAlex,
    #[serde(rename = "em_santa")]
    EmSanta,
    #[serde(rename = "ff_siwis")]
    FfSiwis,
    #[serde(rename = "if_sara")]
    IfSara,
    #[serde(rename = "im_nicola")]
    ImNicola,
    #[serde(rename = "pf_dora")]
    PfDora,
    #[serde(rename = "pm_alex")]
    PmAlex,
    #[serde(rename = "pm_santa")]
    PmSanta,
}

impl Voice {
    /// Every voice, in the order of the voice list: American, British, Mandarin, Spanish, French, Italian
    /// and Portuguese.
    pub const ALL: [Voice; 45] = [
        Voice::AfHeart,
        Voice::AfAlloy,
        Voice::AfAoede,
        Voice::AfBella,
        Voice::AfJessica,
        Voice::AfKore,
        Voice::AfNicole,
        Voice::AfNova,
        Voice::AfRiver,
        Voice::AfSarah,
        Voice::AfSky,
        Voice::AmAdam,
        Voice::AmEcho,
        Voice::AmEric,
        Voice::AmFenrir,
        Voice::AmLiam,
        Voice::AmMichael,
        Voice::AmOnyx,
        Voice::AmPuck,
        Voice::AmSanta,
        Voice::BfAlice,
        Voice::BfEmma,
        Voice::BfIsabella,
        Voice::BfLily,
        Voice::BmDaniel,
        Voice::BmFable,
        Voice::BmGeorge,
        Voice::BmLewis,
        Voice::ZfXiaobei,
        Voice::ZfXiaoni,
        Voice::ZfXiaoxiao,
        Voice::ZfXiaoyi,
        Voice::ZmYunjian,
        Voice::ZmYunxi,
        Voice::ZmYunxia,
        Voice::ZmYunyang,
        Voice::EfDora,
        Voice::EmAlex,
        Voice::EmSanta,
        Voice::FfSiwis,
        Voice::IfSara,
        Voice::ImNicola,
        Voice::PfDora,
        Voice::PmAlex,
        Voice::PmSanta,
    ];

    /// The serialised name, also the voice file's (`voices/af_heart.bin`).
    pub fn name(self) -> &'static str {
        match self {
            Voice::AfHeart => "af_heart",
            Voice::AfAlloy => "af_alloy",
            Voice::AfAoede => "af_aoede",
            Voice::AfBella => "af_bella",
            Voice::AfJessica => "af_jessica",
            Voice::AfKore => "af_kore",
            Voice::AfNicole => "af_nicole",
            Voice::AfNova => "af_nova",
            Voice::AfRiver => "af_river",
            Voice::AfSarah => "af_sarah",
            Voice::AfSky => "af_sky",
            Voice::AmAdam => "am_adam",
            Voice::AmEcho => "am_echo",
            Voice::AmEric => "am_eric",
            Voice::AmFenrir => "am_fenrir",
            Voice::AmLiam => "am_liam",
            Voice::AmMichael => "am_michael",
            Voice::AmOnyx => "am_onyx",
            Voice::AmPuck => "am_puck",
            Voice::AmSanta => "am_santa",
            Voice::BfAlice => "bf_alice",
            Voice::BfEmma => "bf_emma",
            Voice::BfIsabella => "bf_isabella",
            Voice::BfLily => "bf_lily",
            Voice::BmDaniel => "bm_daniel",
            Voice::BmFable => "bm_fable",
            Voice::BmGeorge => "bm_george",
            Voice::BmLewis => "bm_lewis",
            Voice::ZfXiaobei => "zf_xiaobei",
            Voice::ZfXiaoni => "zf_xiaoni",
            Voice::ZfXiaoxiao => "zf_xiaoxiao",
            Voice::ZfXiaoyi => "zf_xiaoyi",
            Voice::ZmYunjian => "zm_yunjian",
            Voice::ZmYunxi => "zm_yunxi",
            Voice::ZmYunxia => "zm_yunxia",
            Voice::ZmYunyang => "zm_yunyang",
            Voice::EfDora => "ef_dora",
            Voice::EmAlex => "em_alex",
            Voice::EmSanta => "em_santa",
            Voice::FfSiwis => "ff_siwis",
            Voice::IfSara => "if_sara",
            Voice::ImNicola => "im_nicola",
            Voice::PfDora => "pf_dora",
            Voice::PmAlex => "pm_alex",
            Voice::PmSanta => "pm_santa",
        }
    }
}

impl std::fmt::Display for Voice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

#[cfg(test)]
#[path = "../tests/unit/voice.rs"]
mod tests;
