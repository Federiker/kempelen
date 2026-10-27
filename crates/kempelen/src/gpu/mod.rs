//! GPU presence and device memory checks, for choosing whether narration runs on CUDA (feature
//! `gpu`).
//!
//! [`detect`] reports an accelerator and device through NVIDIA NVML, Vulkan and Apple-silicon
//! memory information; [`has_nvidia_card`] checks NVML, while [`gpu_vram`] can also report Vulkan
//! memory. Missing backend information yields zero rather than proving that acceleration is
//! available.
//!
//! CUDA is chosen on Windows only, with an NVIDIA driver supporting CUDA 12 or newer, the runtime
//! [`crate::cuda`] uses; other Vulkan devices and Apple silicon are reported as such, otherwise the
//! CPU. Capacity checks do not establish that the model and runtime will work: a
//! [`crate::tts::Synthesizer::with_cuda`] that fails is the signal to synthesize on the CPU.

mod accelerator;
mod vram;

pub use accelerator::{Accelerator, Capability, GpuDevice, detect, vulkan_devices};
#[cfg(test)]
pub(crate) use vram::BYTES_PER_GIB;
pub(crate) use vram::vram_gib;
pub use vram::{gpu_vram, has_nvidia_card};
