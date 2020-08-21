mod frame;
mod internals;
mod setup;
mod unsetup;
use crate::frame_sync::FrameSync;
use crate::hardware_query::HardwareSelection;
use crate::pipeline::Material;
use crate::swapchain::Swapchain;
use crate::vertex::Vertex;
use erupt::{
    utils::{
        self,
        allocator::Allocator,
    },
    vk1_0 as vk, DeviceLoader, InstanceLoader,
};
use openxr as xr;
use nalgebra::Matrix4;
use std::collections::HashMap;
use crate::allocated_buffer::AllocatedBuffer;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MaterialId(u32);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ObjectId(u32);

pub struct Engine {
    frame_wait: Option<xr::FrameWaiter>,
    frame_stream: Option<xr::FrameStream<xr::Vulkan>>,
    stage: Option<xr::Space>,
    materials: HashMap<MaterialId, Material>,
    objects: HashMap<ObjectId, Object>,
    swapchain: Option<Swapchain>,
    allocator: Allocator,
    frame_sync: FrameSync,
    command_pool: vk::CommandPool,
    command_buffers: Vec<vk::CommandBuffer>,
    queue: vk::Queue,
    vk_device: DeviceLoader,
    hardware: HardwareSelection,
    vk_instance: InstanceLoader,
    descriptor_pool: vk::DescriptorPool,
    descriptor_set_layout: vk::DescriptorSetLayout,
    descriptor_sets: Vec<vk::DescriptorSet>,
    camera_ubos: Vec<AllocatedBuffer<[f32; 32]>>,
    next_material_id: u32,
    next_object_id: u32,
    _entry: utils::loading::DefaultEntryLoader,
}

pub struct Object {
    pub indices: AllocatedBuffer<u16>,
    pub vertices: AllocatedBuffer<Vertex>,
    pub n_indices: u32,
    pub material: MaterialId,
    pub transform: Matrix4<f32>,
}
