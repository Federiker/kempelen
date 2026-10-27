use crate::tts::{CEILING, LoudnessTarget, PeakLimiter, SpeechMeter, WavWriter, linear_gain};
use crate::{
    Error, Language, Result, Voice,
    assets::SpeechData,
    phonemes::{
        Phonemizer, ReadingRules, get_lang_code, phonemize_with_rules, phonemizer_for,
        reading_code, split_phonemes, unsupported_language,
    },
};
use crustytts_core::Tokenizer;
use crustytts_tokenizer::KokoroTokenizer;
use ndarray::{Array1, Array2, Axis};
use ort::{inputs, session::Session, value::Tensor};
use std::{
    fs::{File, read},
    io::{BufReader, BufWriter, Read, Write},
    path::{Path, PathBuf},
    sync::Arc,
};

/// Style vector width of Kokoro voice files.
const STYLE_SIZE: usize = 256;
/// Longest phoneme chunk synthesized at once; the model accepts up to 510.
const MAX_CHUNK_PHONEMES: usize = 400;
/// Samples read back at a time in the second pass of [`Synthesizer::synthesize_into`].
const LEVELLING_BLOCK: usize = 65_536;

/// A loaded Kokoro model and voice, reusable across many texts.
///
/// Loading the ONNX session is the slow part of synthesis, so a whole book is best read with
/// one `Synthesizer`.
pub struct Synthesizer {
    session: Session,
    styles: Array2<f32>,
    voice: Voice,
    /// Reading code the text is pronounced with (see `reading_code`).
    reading: &'static str,
    /// Phonemizer for `reading`, made on first use and kept for the whole book.
    phonemizer: Option<Box<dyn Phonemizer + Send>>,
    /// The caller's reading rules, used instead of those kempelen ships for `reading`.
    rules: Option<Arc<ReadingRules>>,
    speed: f32,
    model_path: PathBuf,
    device: Device,
    /// Level every synthesized passage is brought to; `None` only limits the peaks.
    loudness: Option<LoudnessTarget>,
}

/// Hardware running a [`Synthesizer`]'s model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Device {
    Cpu,
    /// An NVIDIA GPU through the installed runtime in [`crate::cuda`].
    Cuda,
}

impl Synthesizer {
    /// Load the verified model and voice from `data`, without network access.
    ///
    /// `speed` must be finite and greater than zero.
    pub fn new(data: &SpeechData, voice: Voice, speed: f32) -> Result<Self> {
        Self::load(data, voice, speed, None)
    }

    /// Like [`Synthesizer::new`], but run the model on an NVIDIA GPU using the CUDA
    /// runtime installed in `cuda_runtime` (see [`crate::cuda`]).
    ///
    /// Errors when the runtime or a CUDA device is unavailable; callers should then
    /// fall back to [`Synthesizer::new`]. Later GPU failures fall back to the CPU
    /// automatically.
    pub fn with_cuda(
        data: &SpeechData,
        voice: Voice,
        speed: f32,
        cuda_runtime: &Path,
    ) -> Result<Self> {
        Self::load(data, voice, speed, Some(cuda_runtime))
    }

    fn load(
        data: &SpeechData,
        voice: Voice,
        speed: f32,
        cuda_runtime: Option<&Path>,
    ) -> Result<Self> {
        if !speed.is_finite() || speed <= 0.0 {
            return Err(Error::InvalidSpeed(speed));
        }
        let model_path = data.model()?;
        let voice_path = data.voice(voice)?;
        let styles = parse_styles(&read(&voice_path).map_err(Error::file("read", &voice_path))?)?;
        let (session, device) = match cuda_runtime {
            Some(runtime) => (cuda_session(&model_path, runtime)?, Device::Cuda),
            None => (cpu_session(&model_path)?, Device::Cpu),
        };
        Ok(Self {
            session,
            styles,
            voice,
            reading: get_lang_code(voice),
            phonemizer: None,
            rules: None,
            speed,
            model_path,
            device,
            loudness: Some(LoudnessTarget::AUDIOBOOK),
        })
    }

    /// Pronounce text as `language` from now on; by default a voice reads its own language.
    ///
    /// A voice reading another language keeps its timbre and accent (see
    /// [`reading_code`]). Errors when narration cannot pronounce `language`.
    pub fn read_as(&mut self, language: Language) -> Result<()> {
        let reading =
            reading_code(self.voice, language).ok_or_else(|| unsupported_language(language))?;
        if reading != self.reading {
            self.reading = reading;
            self.phonemizer = None;
        }
        Ok(())
    }

    /// Read abbreviations and full stops with `rules` from now on, instead of the rules kempelen ships
    /// for the reading language (see [`ReadingRules`]); `None` goes back to the shipped ones.
    pub fn set_reading_rules(&mut self, rules: Option<Arc<ReadingRules>>) {
        self.rules = rules;
    }

    /// Choose the level narration is brought to, or `None` to keep each voice's own loudness
    /// (peaks are still limited). The default is [`LoudnessTarget::AUDIOBOOK`].
    pub fn set_loudness(&mut self, target: Option<LoudnessTarget>) {
        self.loudness = target;
    }

    /// Hardware currently running the model.
    pub fn device(&self) -> Device {
        self.device
    }

    /// Synthesize `text` as the separate model outputs, in reading order (diagnostics).
    ///
    /// Each item is one chunk of at most [`MAX_CHUNK_PHONEMES`] phonemes, exactly as the
    /// model produced it, before any joining.
    pub fn synthesize_chunks(&mut self, text: &str) -> Result<Vec<Vec<f32>>> {
        let chunks = self.phoneme_chunks(text)?;
        chunks
            .iter()
            .map(|chunk| self.synthesize_chunk(chunk))
            .collect()
    }

    /// Synthesize `text` to 24 kHz mono samples.
    ///
    /// Line breaks separate independent passages; long passages are split at
    /// sentence and clause boundaries to fit the model. Errors when the text
    /// contains nothing speakable.
    pub fn synthesize(&mut self, text: &str) -> Result<Vec<f32>> {
        let mut samples: Vec<f32> = self.synthesize_chunks(text)?.concat();
        match self.loudness {
            Some(target) => {
                crate::tts::normalize_loudness(&mut samples, target);
            }
            None => crate::tts::limit_peaks(&mut samples),
        }
        if samples.is_empty() {
            return Err(Error::NoSpeech);
        }
        Ok(samples)
    }

    /// Synthesize `text` as [`Synthesizer::synthesize`] does, handing the finished samples to
    /// `sink` a block at a time, and return how many there were.
    ///
    /// Memory stays flat however long the text: the model's output goes to `scratch`, a
    /// temporary file of 4 bytes a sample, while its speech level is measured; a second pass
    /// reads it back, levels it, limits its peaks and hands it on. `scratch` is removed
    /// afterwards. The samples are exactly those `synthesize` returns.
    ///
    /// `before_chunk(done, total)` is called before each model chunk with the phonemes
    /// synthesized so far and in all, so a caller can report progress or stop a long text
    /// between chunks by returning an error (see [`Error::interrupted`]), which ends the
    /// synthesis with it.
    pub fn synthesize_into(
        &mut self,
        text: &str,
        scratch: &Path,
        sink: &mut dyn FnMut(&[f32]) -> Result<()>,
        before_chunk: &mut dyn FnMut(usize, usize) -> Result<()>,
    ) -> Result<usize> {
        let result = self.synthesize_through(text, scratch, sink, before_chunk);
        let _ = std::fs::remove_file(scratch);
        result
    }

    #[expect(clippy::similar_names, reason = "the limiter and what it limited")]
    fn synthesize_through(
        &mut self,
        text: &str,
        scratch: &Path,
        sink: &mut dyn FnMut(&[f32]) -> Result<()>,
        before_chunk: &mut dyn FnMut(usize, usize) -> Result<()>,
    ) -> Result<usize> {
        // Phonemes first (a fraction of a second even for a long chapter), so progress has a total.
        let chunks = self.phoneme_chunks(text)?;
        let phonemes = chunks.iter().map(|chunk| chunk.chars().count()).sum();
        let mut raw =
            BufWriter::new(File::create(scratch).map_err(Error::file("create", scratch))?);
        let mut meter = SpeechMeter::default();
        let mut total = 0;
        let mut done = 0;
        for chunk in &chunks {
            before_chunk(done, phonemes)?;
            let samples = self.synthesize_chunk(chunk)?;
            meter.push(&samples);
            for sample in &samples {
                raw.write_all(&sample.to_le_bytes())
                    .map_err(Error::file("write", scratch))?;
            }
            total += samples.len();
            done += chunk.chars().count();
        }
        raw.flush().map_err(Error::file("write", scratch))?;
        drop(raw);
        if total == 0 {
            return Err(Error::NoSpeech);
        }
        let (gain, ceiling) = match self.loudness {
            Some(target) => (meter.gain_db(target).map(linear_gain), target.peak_ceiling),
            None => (None, CEILING),
        };
        let mut limiter = PeakLimiter::new(ceiling);
        let mut raw = BufReader::new(File::open(scratch).map_err(Error::file("read", scratch))?);
        let mut bytes = vec![0u8; LEVELLING_BLOCK * 4];
        let mut block = Vec::with_capacity(LEVELLING_BLOCK);
        let mut limited = Vec::with_capacity(LEVELLING_BLOCK * 2);
        let mut left = total;
        while left > 0 {
            let count = left.min(LEVELLING_BLOCK);
            raw.read_exact(&mut bytes[..count * 4])
                .map_err(Error::file("read", scratch))?;
            block.clear();
            block.extend(
                bytes[..count * 4]
                    .chunks_exact(4)
                    .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]])),
            );
            if let Some(gain) = gain {
                for sample in &mut block {
                    *sample *= gain;
                }
            }
            limiter.push(&block, &mut limited);
            sink(&limited)?;
            limited.clear();
            left -= count;
        }
        limiter.finish(&mut limited);
        sink(&limited)?;
        Ok(total)
    }

    /// Synthesize `text` and write it as a mono 24 kHz, 16-bit PCM WAV.
    pub fn synthesize_to_wav(&mut self, text: &str, out: &Path) -> Result<()> {
        let mut writer = WavWriter::create(out)?;
        let result = self
            .synthesize_into(
                text,
                &out.with_extension("raw.tmp"),
                &mut |samples| writer.write(samples),
                &mut |_, _| Ok(()),
            )
            .and_then(|_| writer.finish());
        if result.is_err() {
            // As before streaming: a failed synthesis leaves no file.
            let _ = std::fs::remove_file(out);
        }
        result
    }

    /// The model chunks of `text`: each non-empty line phonemized, then split to fit the model.
    fn phoneme_chunks(&mut self, text: &str) -> Result<Vec<String>> {
        let mut chunks = Vec::new();
        for passage in text.lines().filter(|line| !line.trim().is_empty()) {
            let rules = self
                .rules
                .clone()
                .unwrap_or_else(|| ReadingRules::for_code(self.reading));
            let phonemes = phonemize_with_rules(self.phonemizer()?, &rules, passage)?;
            chunks.extend(split_phonemes(&phonemes, MAX_CHUNK_PHONEMES));
        }
        Ok(chunks)
    }

    fn phonemizer(&mut self) -> Result<&(dyn Phonemizer + Send)> {
        if self.phonemizer.is_none() {
            self.phonemizer = Some(phonemizer_for(self.reading)?);
        }
        #[expect(clippy::expect_used, reason = "the phonemizer was just started")]
        Ok(self
            .phonemizer
            .as_deref()
            .expect("phonemizer was just started"))
    }

    fn synthesize_chunk(&mut self, phonemes: &str) -> Result<Vec<f32>> {
        let ids = KokoroTokenizer::new().encode(phonemes);
        // Two boundary tokens wrap the phonemes; the style is chosen by the
        // phoneme count, as in the reference implementation.
        let phoneme_count = ids.len().saturating_sub(2);
        if phoneme_count == 0 {
            return Ok(Vec::new());
        }
        let style_row = (phoneme_count - 1).min(self.styles.nrows() - 1);
        let style = self.styles.row(style_row).to_owned().insert_axis(Axis(0));
        let input_ids = Tensor::from_array(
            Array2::from_shape_vec((1, ids.len()), ids)
                .map_err(|_| Error::InvalidVoice("no phoneme row"))?,
        )?;
        let style = Tensor::from_array(style)?;
        let speed = Tensor::from_array(Array1::from_vec(vec![self.speed]))?;
        let run = |session: &mut Session| -> Result<Vec<f32>> {
            let outputs = session.run(inputs![
                "input_ids" => input_ids.clone(),
                "style" => style.clone(),
                "speed" => speed.clone(),
            ])?;
            let (_, waveform) = outputs["waveform"].try_extract_tensor::<f32>()?;
            Ok(waveform.to_vec())
        };
        match run(&mut self.session) {
            Err(error) if self.device == Device::Cuda => {
                log::warn!("GPU speech synthesis failed, continuing on the CPU: {error}");
                self.session = cpu_session(&self.model_path)?;
                self.device = Device::Cpu;
                run(&mut self.session)
            }
            result => result,
        }
    }
}

/// A session on the CPU using every logical core.
///
/// ONNX Runtime's default is the physical cores; all logical ones narrate about 10 % faster
/// (measured on an 8-core, 16-thread Ryzen, see docs/performance.md). The thread count
/// changes the order in which Kokoro draws its noise, so the waveform differs from one count to
/// another while sounding the same; at a given count it is reproducible.
fn cpu_session(model_path: &Path) -> Result<Session> {
    let threads = std::thread::available_parallelism().map_or(1, std::num::NonZero::get);
    Ok(Session::builder()?
        .with_intra_threads(threads)
        .map_err(ort::Error::from)?
        .commit_from_file(model_path)?)
}

#[cfg(all(windows, target_arch = "x86_64"))]
fn cuda_session(model_path: &Path, runtime: &Path) -> Result<Session> {
    crate::cuda::register(runtime)?;
    let environment = ort::environment::Environment::current()?;
    let devices: Vec<_> = environment
        .devices()
        .filter(|device| device.ep().is_ok_and(|ep| ep.contains("CUDA")))
        .collect();
    if devices.is_empty() {
        return Err(crate::cuda::CudaError::NoDevice.into());
    }
    Ok(Session::builder()?
        .with_devices(devices, None)
        .map_err(ort::Error::from)?
        .commit_from_file(model_path)?)
}

#[cfg(not(all(windows, target_arch = "x86_64")))]
fn cuda_session(_model_path: &Path, _runtime: &Path) -> Result<Session> {
    Err(crate::cuda::CudaError::Unsupported.into())
}

/// Parse a voice file: little-endian `f32` style vectors, one per sequence length.
pub(super) fn parse_styles(bytes: &[u8]) -> Result<Array2<f32>> {
    if !bytes.len().is_multiple_of(4) {
        return Err(Error::InvalidVoice(
            "its size is not a whole number of floats",
        ));
    }
    let floats: Vec<f32> = bytes
        .chunks_exact(4)
        .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .collect();
    if floats.is_empty() || !floats.len().is_multiple_of(STYLE_SIZE) {
        return Err(Error::InvalidVoice("its style vectors are not 256 wide"));
    }
    Array2::from_shape_vec((floats.len() / STYLE_SIZE, STYLE_SIZE), floats)
        .map_err(|_| Error::InvalidVoice("its style vectors are not 256 wide"))
}

#[cfg(test)]
#[path = "../../tests/unit/tts/synth.rs"]
mod tests;
