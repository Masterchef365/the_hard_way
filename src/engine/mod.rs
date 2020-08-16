mod frame;
mod internals;
mod setup;
mod unsetup;
use crate::frame_sync::FrameSync;
use crate::hardware_query::HardwareSelection;
use crate::pipeline::DrawType;
use crate::pipeline::Material;
use crate::swapchain::Swapchain;
use crate::vertex::Vertex;
use anyhow::Result;
use erupt::{
    extensions::khr_surface,
    utils::{
        self,
        allocator::Allocator,
    },
    vk1_0 as vk, DeviceLoader, InstanceLoader,
};
use nalgebra::Matrix4;
use std::collections::HashMap;
use crate::allocated_buffer::AllocatedBuffer;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MaterialId(u32);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ObjectId(u32);

pub struct Engine {
    materials: HashMap<MaterialId, Material>,
    objects: HashMap<ObjectId, Object>,
    swapchain: Option<Swapchain>,
    allocator: Allocator,
    frame_sync: FrameSync,
    command_pool: vk::CommandPool,
    command_buffers: Vec<vk::CommandBuffer>,
    queue: vk::Queue,
    device: DeviceLoader,
    hardware: HardwareSelection,
    surface: khr_surface::SurfaceKHR,
    instance: InstanceLoader,
    descriptor_pool: vk::DescriptorPool,
    descriptor_set_layout: vk::DescriptorSetLayout,
    descriptor_sets: Vec<vk::DescriptorSet>,
    camera_ubos: Vec<AllocatedBuffer<[[f32; 4]; 4]>>,
    next_material_id: u32,
    next_object_id: u32,
    _entry: utils::loading::DefaultEntryLoader,
}

impl Engine {
    pub fn load_material(
        &mut self,
        vertex: &[u8],
        fragment: &[u8],
        draw_type: DrawType,
    ) -> Result<MaterialId> {
        let id = MaterialId(self.next_material_id);
        self.next_material_id += 1;
        let material = Material::new(&self.device, vertex, fragment, draw_type)?;
        if let Some(swapchain) = &mut self.swapchain {
            swapchain.add_pipeline(&self.device, self.descriptor_set_layout, id, &material)?;
        }
        self.materials.insert(id, material);
        Ok(id)
    }

    pub fn unload_material(&mut self, material: MaterialId) {
        if let Some(mut mat) = self.materials.remove(&material) {
            mat.free(&self.device);
        }
        if let Some(swapchain) = &mut self.swapchain {
            swapchain.remove_pipeline(&self.device, material);
        }
    }

    pub fn add_object(
        &mut self,
        vertices: &[Vertex],
        indices: &[u16],
        material: MaterialId,
    ) -> Result<ObjectId> {
        let id = ObjectId(self.next_object_id);
        self.next_object_id += 1;

        let n_indices = indices.len() as u32;

        //TODO: Use staging buffers as well!
        let create_info = vk::BufferCreateInfoBuilder::new()
            .usage(vk::BufferUsageFlags::VERTEX_BUFFER)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);
        let vertex_buffer = AllocatedBuffer::new(
            vertices.len(),
            create_info,
            &mut self.allocator,
            &self.device,
        )?;
        vertex_buffer.map(&self.device, vertices)?;

        let create_info = vk::BufferCreateInfoBuilder::new()
            .usage(vk::BufferUsageFlags::INDEX_BUFFER)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);
        let index_buffer = AllocatedBuffer::new(
            indices.len(),
            create_info,
            &mut self.allocator,
            &self.device,
        )?;
        index_buffer.map(&self.device, indices)?;

        let object = Object {
            material,
            indices: index_buffer,
            vertices: vertex_buffer,
            n_indices,
            transform: Matrix4::identity(),
        };

        self.objects.insert(id, object);

        Ok(id)
    }

    pub fn remove_object(&mut self, id: ObjectId) -> Result<()> {
        // Figure out how not to wait?
        unsafe {
            self.device.device_wait_idle().result()?;
        }
        if let Some(mut object) = self.objects.remove(&id) {
            object.vertices.free(&self.device, &mut self.allocator)?;
            object.indices.free(&self.device, &mut self.allocator)?;
        }
        Ok(())
    }

    fn set_transform(&mut self, id: ObjectId, transform: Matrix4<f32>) {
        if let Some(object) = self.objects.get_mut(&id) {
            object.transform = transform;
        }
    }
}

pub struct Object {
    pub indices: AllocatedBuffer<u16>,
    pub vertices: AllocatedBuffer<Vertex>,
    pub n_indices: u32,
    pub material: MaterialId,
    pub transform: Matrix4<f32>,
}
