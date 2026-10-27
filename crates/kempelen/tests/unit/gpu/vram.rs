use super::{BYTES_PER_GIB, device_local_vram_gib, vram_gib};

#[test]
fn vram_is_reported_as_whole_gib_and_saturates_at_i8_max() {
    assert_eq!(vram_gib(0), 0);
    assert_eq!(vram_gib(6 * BYTES_PER_GIB + BYTES_PER_GIB / 4), 6);
    assert_eq!(vram_gib(6 * BYTES_PER_GIB - BYTES_PER_GIB / 6), 6);
    assert_eq!(vram_gib(u64::MAX), i8::MAX);
}

#[test]
fn vulkan_vram_uses_only_device_local_memory_heaps() {
    let mut memory_heaps = [ash::vk::MemoryHeap::default(); 16];
    memory_heaps[0] = ash::vk::MemoryHeap::default()
        .size(6 * BYTES_PER_GIB)
        .flags(ash::vk::MemoryHeapFlags::DEVICE_LOCAL);
    memory_heaps[1] = ash::vk::MemoryHeap::default()
        .size(16 * BYTES_PER_GIB)
        .flags(ash::vk::MemoryHeapFlags::empty());
    let memory = ash::vk::PhysicalDeviceMemoryProperties {
        memory_heap_count: 2,
        memory_heaps,
        ..Default::default()
    };

    assert_eq!(device_local_vram_gib(&memory), 6);
}
