use nalgebra::{Point3, Matrix4, Vector3};

pub struct Camera {
    pub eye: Point3<f32>,
    pub at: Point3<f32>,
    pub fovy: f32,
    pub clip_near: f32,
    pub clip_far: f32,
}

impl Camera {
    pub fn view(&self) -> Matrix4<f32> {
        Matrix4::look_at_rh(&self.eye, &self.at, &Vector3::new(0.0, -1.0, 0.0))
    }

    pub fn projection(&self, aspect: f32) -> Matrix4<f32> {
        Matrix4::new_perspective(aspect, self.fovy, self.clip_near, self.clip_far)
    }

    pub fn matrix(&self, aspect: f32) -> Matrix4<f32> {
        self.projection(aspect) * self.view()
    }
}
