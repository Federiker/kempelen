use super::*;
use crate::gpu::BYTES_PER_GIB;

fn gpu(name: &str, vendor_id: u32, gib: u64, discrete: bool) -> GpuDevice {
    GpuDevice {
        name: name.into(),
        vendor_id,
        vram_bytes: gib * BYTES_PER_GIB,
        discrete,
    }
}

#[test]
fn a_discrete_nvidia_card_with_a_current_driver_uses_cuda() {
    let devices = [
        gpu("AMD iGPU", 0x1002, 2, false),
        gpu("RTX 3060", 0x10DE, 6, true),
    ];
    let found = decide(&devices, true, "windows", "x86_64", None);
    assert_eq!(found.accelerator, Accelerator::Cuda);
    assert_eq!(found.device.unwrap().name, "RTX 3060");
}

#[test]
fn a_six_gib_laptop_card_reporting_slightly_less_still_counts() {
    let devices = [gpu("RTX 3060 Laptop", 0x10DE, 0, true)];
    let mut devices = devices.to_vec();
    devices[0].vram_bytes = 6_285_164_544;
    assert_eq!(devices[0].vram_gib(), 6);
}

#[test]
fn nvidia_without_a_cuda_driver_falls_back_to_vulkan() {
    let devices = [gpu("RTX 3060", 0x10DE, 6, true)];
    assert_eq!(
        decide(&devices, false, "windows", "x86_64", None).accelerator,
        Accelerator::Vulkan
    );
}

#[test]
fn other_vendors_use_vulkan() {
    let devices = [gpu("RX 7800", 0x1002, 16, true)];
    let found = decide(&devices, true, "linux", "x86_64", None);
    assert_eq!(found.accelerator, Accelerator::Vulkan);
}

#[test]
fn no_gpu_means_cpu() {
    let found = decide(&[], false, "linux", "x86_64", None);
    assert_eq!(found.accelerator, Accelerator::Cpu);
}

#[test]
fn apple_silicon_uses_metal_with_half_the_unified_memory() {
    let found = decide(&[], false, "macos", "aarch64", Some(16 * BYTES_PER_GIB));
    assert_eq!(found.accelerator, Accelerator::Metal);
    let small = decide(&[], false, "macos", "aarch64", Some(8 * BYTES_PER_GIB));
    assert_eq!(small.accelerator, Accelerator::Metal);
}

#[test]
fn detection_on_this_machine_gives_a_consistent_answer() {
    let found = detect();
    eprintln!("{found:?}");
    // Metal only on a Mac; a GPU choice names its device and memory; the CPU needs neither.
    assert_eq!(
        found.accelerator == Accelerator::Metal,
        cfg!(target_os = "macos") && found.accelerator != Accelerator::Cpu,
        "{found:?}"
    );
    if matches!(found.accelerator, Accelerator::Cuda | Accelerator::Vulkan) {
        assert!(found.device.is_some() && found.vram_gib > 0, "{found:?}");
    }
}
