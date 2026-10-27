use ash::vk;
use nvml_wrapper::Nvml;

pub(crate) const BYTES_PER_GIB: u64 = 1024 * 1024 * 1024;

/// Return whether NVML can detect at least one NVIDIA device.
#[must_use]
pub fn has_nvidia_card() -> bool {
    Nvml::init()
        .and_then(|nvml| nvml.device_count())
        .is_ok_and(|device_count| device_count > 0)
}

/// Return the largest detected NVIDIA or Vulkan device-local memory size in GiB.
///
/// The value is rounded to whole GiB and is zero if neither backend reports a
/// usable value.
#[must_use]
pub fn gpu_vram() -> i8 {
    max_nvidia_vram_gib()
        .into_iter()
        .chain(max_vulkan_vram_gib())
        .max()
        .unwrap_or(0)
}

fn max_nvidia_vram_gib() -> Option<i8> {
    let nvml = Nvml::init().ok()?;
    let device_count = nvml.device_count().ok()?;

    (0..device_count)
        .filter_map(|index| nvml.device_by_index(index).ok())
        .filter_map(|device| device.memory_info().ok())
        .map(|memory| vram_gib(memory.total))
        .max()
}

fn max_vulkan_vram_gib() -> Option<i8> {
    let entry = unsafe { ash::Entry::load() }.ok()?;
    let application_info = vk::ApplicationInfo::default();
    let create_info = vk::InstanceCreateInfo::default().application_info(&application_info);
    let instance = unsafe { entry.create_instance(&create_info, None) }.ok()?;
    let physical_devices = unsafe { instance.enumerate_physical_devices() };

    let max_vram = physical_devices.ok().and_then(|devices| {
        devices
            .iter()
            .map(|device| unsafe {
                device_local_vram_gib(&instance.get_physical_device_memory_properties(*device))
            })
            .max()
    });

    unsafe { instance.destroy_instance(None) };
    max_vram
}

fn device_local_vram_gib(memory: &vk::PhysicalDeviceMemoryProperties) -> i8 {
    let heap_count = memory
        .memory_heap_count
        .min(u32::try_from(memory.memory_heaps.len()).unwrap_or(u32::MAX))
        as usize;
    let local_memory = memory.memory_heaps[..heap_count]
        .iter()
        .filter(|heap| heap.flags.contains(vk::MemoryHeapFlags::DEVICE_LOCAL))
        .fold(0u64, |total, heap| total.saturating_add(heap.size));

    vram_gib(local_memory)
}

pub(crate) fn vram_gib(bytes: u64) -> i8 {
    // Cards sold as 6 GiB report slightly less (a 3060 laptop shows 5.85 GiB), so round.
    i8::try_from(bytes.saturating_add(BYTES_PER_GIB / 2) / BYTES_PER_GIB).unwrap_or(i8::MAX)
}

#[cfg(test)]
#[path = "../../tests/unit/gpu/vram.rs"]
mod tests;
