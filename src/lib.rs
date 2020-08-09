#![allow(unused)]
use anyhow::Result;
use hecs::World;
use nalgebra::{Matrix4, Point2, Point3};
use std::path::Path;
use winit::window::Window;

pub struct Engine {}

type Id = u32;
pub struct MaterialId(Id); // Refers to a VkPipeline
pub struct MeshId(Id); // Refers to a set of vertex and index buffers

pub enum DrawType {
    Triangles,
    Lines,
    Points,
}

pub struct Object {
    pub material: MaterialId,
    pub mesh: MeshId,
    pub transform: Matrix4<f32>,
}

impl Engine {
    pub fn new(window: &Window) -> Result<Self> {
        todo!()
    }

    pub fn next_frame(
        &mut self,
        objects: &[Object],
        camera: &Matrix4<f32>,
        time: f32,
    ) -> Result<Self> {
        // For each object, set its transform uniform
        // Set uniform for time
        // Set uniform for view and projection from Camera
        // pseudocode from earlier, grouping objects by ShaderId
        todo!()
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
