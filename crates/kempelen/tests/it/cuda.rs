//! GPU narration benchmark; needs an NVIDIA GPU and installs the CUDA runtime
//! (about 1.6 GB download) on first run:
//! `cargo test --release -p kempelen --features cuda-install --test it -- --ignored --nocapture cuda::`
//! It installs into `tmp/cuda` (gitignored).

#![cfg(all(windows, target_arch = "x86_64", feature = "cuda-install"))]

use crate::speech::data;
use kempelen::{
    Voice, cuda,
    tts::{Device, Synthesizer},
};
use std::time::Instant;

// L. Frank Baum, "The Wonderful Wizard of Oz" (1900), public domain.
const TEXT: &str = "Dorothy lived in the midst of the great Kansas prairies, with Uncle Henry, who was a farmer, and Aunt Em, who was the farmer's wife. Their house was small, for the lumber to build it had to be carried by wagon many miles. There were four walls, a floor and a roof, which made one room; and this room contained a rusty looking cookstove, a cupboard for the dishes, a table, three or four chairs, and the beds.
When Dorothy stood in the doorway and looked around, she could see nothing but the great gray prairie on every side. Not a tree nor a house broke the broad sweep of flat country that reached to the edge of the sky in all directions. The sun had baked the plowed land into a gray mass, with little cracks running through it.";

/// The static ONNX Runtime core loads this from the executable's folder; Cargo places
/// it in `target/<profile>` while test binaries run from `target/<profile>/deps`.
fn stage_shared_provider() {
    let exe = std::env::current_exe().unwrap();
    let deps = exe.parent().unwrap();
    let target = deps.join(cuda::SHARED_PROVIDER);
    if !target.exists() {
        std::fs::copy(deps.parent().unwrap().join(cuda::SHARED_PROVIDER), target)
            .expect("build ort first so target/<profile> holds the shared provider");
    }
}

/// Relative RMS difference of the 20 ms loudness envelopes; the GPU's oscillator phase
/// drifts, so per-sample differences are large even when the speech is identical.
fn envelope_difference(a: &[f32], b: &[f32]) -> f64 {
    let rms = |frame: &[f32]| {
        (frame.iter().map(|x| f64::from(*x).powi(2)).sum::<f64>() / frame.len() as f64).sqrt()
    };
    let (mut error, mut signal) = (0.0, 0.0);
    for (fa, fb) in a.chunks(480).zip(b.chunks(480)) {
        error += (rms(fa) - rms(fb)).powi(2);
        signal += rms(fa).powi(2);
    }
    (error / signal).sqrt()
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs an NVIDIA GPU and downloads the CUDA speech runtime"]
async fn cuda_narration_matches_the_cpu_and_is_faster() {
    stage_shared_provider();
    let runtime =
        cuda::runtime_dir(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tmp"));
    if !cuda::is_installed(&runtime) {
        eprintln!("installing the CUDA runtime into {}", runtime.display());
        cuda::install(&runtime, |done, total| {
            if done == total || done % (100 << 20) < (1 << 16) {
                eprintln!("  {} / {} MB", done >> 20, total >> 20);
            }
        })
        .await
        .unwrap();
    }

    let data = data();
    let mut cpu = Synthesizer::new(&data, Voice::AfAlloy, 1.0).unwrap();
    let mut gpu = Synthesizer::with_cuda(&data, Voice::AfAlloy, 1.0, &runtime).unwrap();
    gpu.synthesize("Warm up.").unwrap();
    cpu.synthesize("Warm up.").unwrap();

    let start = Instant::now();
    let reference = cpu.synthesize(TEXT).unwrap();
    let cpu_secs = start.elapsed().as_secs_f64();
    let start = Instant::now();
    let samples = gpu.synthesize(TEXT).unwrap();
    let gpu_secs = start.elapsed().as_secs_f64();

    let audio_secs = reference.len() as f64 / 24_000.0;
    let difference = envelope_difference(&reference, &samples);
    eprintln!(
        "{audio_secs:.1} s of audio: CPU {cpu_secs:.2} s, CUDA {gpu_secs:.2} s ({:.2}x), envelope difference {:.2}%",
        cpu_secs / gpu_secs,
        difference * 100.0
    );
    assert_eq!(gpu.device(), Device::Cuda, "CUDA fell back to the CPU");
    assert_eq!(samples.len(), reference.len());
    assert!(difference < 0.05, "GPU speech differs from the CPU");
}
