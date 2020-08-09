mod setup;
mod unsetup;
mod internals;
use anyhow::Result;
use crate::frame_sync::FrameSync;
use crate::hardware_query::HardwareSelection;
use nalgebra::{Matrix4, Point2, Point3};
use std::path::Path;
use winit::window::Window;
use erupt::{
    extensions::{ext_debug_utils, khr_surface, khr_swapchain},
    utils::{self, allocator, surface},
    vk1_0 as vk, DeviceLoader, EntryLoader, InstanceLoader,
};
pub use crate::pipeline::{DrawType, MaterialId};
use crate::swapchain::Swapchain;
use crate::pipeline::Material;
use std::collections::HashMap;

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
    _entry: utils::loading::DefaultEntryLoader,
}

impl Engine {
    pub fn next_frame(
        &mut self,
        objects: &[Object],
        camera: &Matrix4<f32>,
        time: f32,
    ) -> Result<()> {
        // For each object, set its transform uniform
        // Set uniform for time
        // Set uniform for view and projection from Camera
        // pseudocode from earlier, grouping objects by ShaderId
        Ok(())
    }

    pub fn load_material(
        &mut self,
        vertex: impl AsRef<Path>,
        fragment: impl AsRef<Path>,
        draw_type: DrawType,
    ) -> Result<MaterialId> {
        todo!()
    }

    pub fn unload_material(&mut self, material: MaterialId) {
        todo!()
    }

    pub fn load_mesh(
        &mut self,
        vertices: &[Point3<f32>],
        uv: &[Point2<f32>],
        indices: &[u16],
    ) -> Result<MeshId> {
        todo!()
    }

    pub fn unload_mesh(&mut self, mesh: MeshId) {
        todo!()
    }
}

pub struct MeshId(u32); // Refers to a set of vertex and index buffers

pub struct Object {
    pub material: MaterialId,
    pub mesh: MeshId,
    pub transform: Matrix4<f32>,
}
