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
    extensions::{ext_debug_utils, khr_surface, khr_swapchain},
    utils::{self, allocator, surface},
    vk1_0 as vk, DeviceLoader, EntryLoader, InstanceLoader,
};
use nalgebra::{Matrix4, Point2, Point3};
use std::collections::HashMap;
use std::path::Path;
use winit::window::Window;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MaterialId(u32);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ObjectId(u32);

pub struct Engine {
    materials: HashMap<MaterialId, Material>,
    objects: HashMap<ObjectId, Object>,
    swapchain: Option<Swapchain>,
    allocator: allocator::Allocator,
    frame_sync: FrameSync,
    command_pool: vk::CommandPool,
    queue: vk::Queue,
    device: DeviceLoader,
    hardware: HardwareSelection,
    surface: khr_surface::SurfaceKHR,
    instance: InstanceLoader,
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
            swapchain.add_pipeline(&self.device, id, &material)?;
        }
        self.materials.insert(id, material);
        Ok(id)
    }

    pub fn unload_material(&mut self, material: MaterialId) {
        let mut mat = self.materials.remove(&material).unwrap();
        if let Some(swapchain) = &mut self.swapchain {
            swapchain.remove_pipeline(&self.device, material);
        }
        mat.free(&self.device);
    }

    pub fn add_object(
        &mut self,
        vertices: &[Vertex],
        indices: &[u16],
        material: MaterialId,
    ) -> Result<ObjectId> {
        let id = ObjectId(self.next_object_id);
        self.next_object_id += 1;

        let create_info = vk::BufferCreateInfoBuilder::new()
            .usage(vk::BufferUsageFlags::VERTEX_BUFFER)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);
        let vertices = self.allocate_buffer(create_info, vertices)?;

        let create_info = vk::BufferCreateInfoBuilder::new()
            .usage(vk::BufferUsageFlags::INDEX_BUFFER)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        let n_indices = indices.len() as u32;
        let indices = self.allocate_buffer(create_info, indices)?;

        let object = Object {
            material,
            indices,
            vertices,
            n_indices,
            transform: Matrix4::identity(),
            freed: false,
        };

        self.objects.insert(id, object);

        Ok(id)
    }

    pub fn remove_object(&mut self, id: ObjectId) {
        unsafe {
            self.device.device_wait_idle().unwrap();
        } // Figure out how not to wait?
        let object = self.objects.remove(&id).unwrap();
        self.free_buffer(object.vertices);
        self.free_buffer(object.indices);
    }

    fn allocate_buffer<T: bytemuck::Pod>(
        &mut self,
        create_info: vk::BufferCreateInfoBuilder,
        data: &[T],
    ) -> Result<AllocatedBuffer> {
        let create_info = create_info.size((data.len() * std::mem::size_of::<T>()) as u64);
        let buffer = unsafe { self.device.create_buffer(&create_info, None, None) }.result()?;
        let allocation = self
            .allocator
            .allocate(&self.device, buffer, allocator::MemoryTypeFinder::dynamic())
            .result()?;
        let mut map = allocation.map(&self.device, ..).result()?;
        map.import(bytemuck::cast_slice(data));
        map.unmap(&self.device).result()?;

        Ok(AllocatedBuffer {
            buffer,
            allocation,
            freed: false,
        })
    }

    fn free_buffer(&mut self, buffer: AllocatedBuffer) {
        self.allocator.free(&self.device, buffer.allocation);
    }
}

pub struct AllocatedBuffer {
    pub buffer: vk::Buffer,
    pub allocation: allocator::Allocation<vk::Buffer>,
    freed: bool,
}

pub struct Object {
    pub indices: AllocatedBuffer,
    pub vertices: AllocatedBuffer,
    pub n_indices: u32,
    pub material: MaterialId,
    pub transform: Matrix4<f32>,
    freed: bool,
}
