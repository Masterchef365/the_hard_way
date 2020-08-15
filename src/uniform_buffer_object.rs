use crate::camera::Camera;
use nalgebra::Matrix4;

#[repr(C)]
#[derive(Default, Copy, Clone)]
pub struct UniformBufferObject {
    pub model: [[f32; 4]; 4],
    pub view: [[f32; 4]; 4],
    pub proj: [[f32; 4]; 4],
}

unsafe impl bytemuck::Zeroable for UniformBufferObject {}
unsafe impl bytemuck::Pod for UniformBufferObject {}

impl UniformBufferObject {
    pub fn from_matrices(model: Matrix4<f32>, view: Matrix4<f32>, proj: Matrix4<f32>) -> Self {
        Self {
            model: *model.as_ref(),
            view: *view.as_ref(),
            proj: *proj.as_ref(),
        }
    }
}
