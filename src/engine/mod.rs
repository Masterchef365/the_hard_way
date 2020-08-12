mod frame;
mod internals;
mod setup;
mod unsetup;
use crate::frame_sync::FrameSync;
use crate::hardware_query::HardwareSelection;
pub use crate::pipeline::DrawType;
use crate::pipeline::Material;
use crate::swapchain::Swapchain;
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
        self.materials.insert(id, material);
        Ok(id)
    }

    pub fn unload_material(&mut self, material: MaterialId) -> Result<()> {
        if let Some(mut mat) = self.materials.remove(&material) {
            mat.free(&self.device);
            Ok(())
        } else {
            Err(anyhow::format_err!(
                "Tried to free non-existant material {:?}",
                material
            ))
        }
    }

    pub fn add_object(
        &mut self,
        vertices: &[Point3<f32>],
        //uv: &[Point2<f32>],
        indices: &[u16],
    ) -> Result<ObjectId> {
        let id = self.next_object_id;
        self.next_object_id += 1;
        Ok(ObjectId(id))
    }

    pub fn remove_object(&mut self, mesh: ObjectId) {
        todo!()
    }
}

/*
pub struct AllocatedBuffer {
    pub buffer: vk::Buffer,
    pub allocation: allocator::Allocation<vk::Buffer>,
}

pub struct Object {
    pub indices: AllocatedBuffer,
    pub vertices: AllocatedBuffer,
    pub n_indices: u32,
    pub material: MaterialId,
    pub mesh: MeshId,
    pub transform: Matrix4<f32>,
}
*/
