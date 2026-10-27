use super::vram_gib;
use ash::vk;
use nvml_wrapper::Nvml;

/// CUDA 12 runtime libraries need a driver reporting at least this version (12.0).
const MIN_CUDA_DRIVER: i32 = 12000;

/// The available compute backend.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Accelerator {
    Cuda,
    Vulkan,
    Metal,
    Cpu,
}

/// One graphics device and its device-local memory.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GpuDevice {
    pub name: String,
    pub vendor_id: u32,
    pub vram_bytes: u64,
    pub discrete: bool,
}

impl GpuDevice {
    /// Memory in whole GiB, rounded (see [`vram_gib`]).
    #[must_use]
    pub fn vram_gib(&self) -> i8 {
        vram_gib(self.vram_bytes)
    }

    #[must_use]
    pub fn is_nvidia(&self) -> bool {
        self.vendor_id == 0x10DE
    }
}

/// What accelerator the machine provides.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Capability {
    pub accelerator: Accelerator,
    pub device: Option<GpuDevice>,
    pub vram_gib: i8,
}

/// Vulkan devices with their device-local memory; empty if no Vulkan driver is present.
#[must_use]
pub fn vulkan_devices() -> Vec<GpuDevice> {
    let Ok(entry) = (unsafe { ash::Entry::load() }) else {
        return Vec::new();
    };
    let application_info = vk::ApplicationInfo::default();
    let create_info = vk::InstanceCreateInfo::default().application_info(&application_info);
    let Ok(instance) = (unsafe { entry.create_instance(&create_info, None) }) else {
        return Vec::new();
    };
    let devices = unsafe { instance.enumerate_physical_devices() }
        .unwrap_or_default()
        .into_iter()
        .map(|device| unsafe {
            let properties = instance.get_physical_device_properties(device);
            let memory = instance.get_physical_device_memory_properties(device);
            GpuDevice {
                name: properties
                    .device_name_as_c_str()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_default(),
                vendor_id: properties.vendor_id,
                vram_bytes: local_memory_bytes(&memory),
                discrete: properties.device_type == vk::PhysicalDeviceType::DISCRETE_GPU,
            }
        })
        .collect();
    unsafe { instance.destroy_instance(None) };
    devices
}

pub(crate) fn local_memory_bytes(memory: &vk::PhysicalDeviceMemoryProperties) -> u64 {
    let count = memory
        .memory_heap_count
        .min(u32::try_from(memory.memory_heaps.len()).unwrap_or(u32::MAX)) as usize;
    memory.memory_heaps[..count]
        .iter()
        .filter(|heap| heap.flags.contains(vk::MemoryHeapFlags::DEVICE_LOCAL))
        .fold(0u64, |total, heap| total.saturating_add(heap.size))
}

fn cuda_driver_supported() -> bool {
    Nvml::init()
        .and_then(|nvml| nvml.sys_cuda_driver_version())
        .is_ok_and(|version| version >= MIN_CUDA_DRIVER)
}

/// Detect the best available accelerator.
#[must_use]
pub fn detect() -> Capability {
    let devices = vulkan_devices();
    let cuda = std::env::consts::OS == "windows" && cuda_driver_supported();
    let apple_memory = apple_unified_memory();
    decide(
        &devices,
        cuda,
        std::env::consts::OS,
        std::env::consts::ARCH,
        apple_memory,
    )
}

fn apple_unified_memory() -> Option<u64> {
    if std::env::consts::OS != "macos" {
        return None;
    }
    let output = std::process::Command::new("sysctl")
        .args(["-n", "hw.memsize"])
        .output()
        .ok()?;
    String::from_utf8_lossy(&output.stdout).trim().parse().ok()
}

/// Pure decision logic, separated from hardware probing so it can be tested.
pub(crate) fn decide(
    devices: &[GpuDevice],
    cuda_usable: bool,
    os: &str,
    arch: &str,
    apple_memory: Option<u64>,
) -> Capability {
    if os == "macos" && arch == "aarch64" {
        // Unified memory is shared with the system, so only half counts as usable.
        let bytes = apple_memory.unwrap_or(0) / 2;
        return capability(Accelerator::Metal, None, bytes);
    }
    let best = devices
        .iter()
        .filter(|device| device.discrete || device.vram_bytes > 0)
        .max_by_key(|device| (device.discrete, device.vram_bytes));
    let Some(best) = best else {
        return capability(Accelerator::Cpu, None, 0);
    };
    let accelerator = if cuda_usable && best.is_nvidia() {
        Accelerator::Cuda
    } else {
        Accelerator::Vulkan
    };
    capability(accelerator, Some(best.clone()), best.vram_bytes)
}

fn capability(accelerator: Accelerator, device: Option<GpuDevice>, bytes: u64) -> Capability {
    Capability {
        accelerator,
        device,
        vram_gib: vram_gib(bytes),
    }
}

#[cfg(test)]
#[path = "../../tests/unit/gpu/accelerator.rs"]
mod tests;
