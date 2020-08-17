use erupt::vk1_0 as vk;

#[derive(Debug)]
pub struct HardwareSelection {
    pub physical_device: vk::PhysicalDevice,
    pub queue_family: u32,
}
