use super::{Engine, MaterialId, ObjectId, Object};
use crate::frame_sync::FrameSync;
use crate::hardware_query::HardwareSelection;
use crate::pipeline::{DrawType, Material};
use crate::swapchain::Swapchain;
use anyhow::Result;
use crate::allocated_buffer::AllocatedBuffer;
use crate::vertex::Vertex;
use erupt::{
    extensions::khr_surface,
    utils::{self, allocator::Allocator},
    vk1_0 as vk, DeviceLoader, InstanceLoader,
};
use nalgebra::Matrix4;
use std::collections::HashMap;

impl Engine {
    pub(crate) fn invalidate_swapchain(&mut self) -> Result<()> {
        if let Some(swapchain) = &mut self.swapchain {
            swapchain.free(&self.vk_device, &mut self.allocator)?;
        }
        self.swapchain = None;
        Ok(())
    }
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
        let material = Material::new(&self.vk_device, vertex, fragment, draw_type)?;
        if let Some(swapchain) = &mut self.swapchain {
            swapchain.add_pipeline(&self.vk_device, self.descriptor_set_layout, id, &material)?;
        }
        self.materials.insert(id, material);
        Ok(id)
    }

    pub fn unload_material(&mut self, material: MaterialId) {
        if let Some(mut mat) = self.materials.remove(&material) {
            mat.free(&self.vk_device);
        }
        if let Some(swapchain) = &mut self.swapchain {
            swapchain.remove_pipeline(&self.vk_device, material);
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
            &self.vk_device,
        )?;
        vertex_buffer.map(&self.vk_device, vertices)?;

        let create_info = vk::BufferCreateInfoBuilder::new()
            .usage(vk::BufferUsageFlags::INDEX_BUFFER)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);
        let index_buffer = AllocatedBuffer::new(
            indices.len(),
            create_info,
            &mut self.allocator,
            &self.vk_device,
        )?;
        index_buffer.map(&self.vk_device, indices)?;

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
            self.vk_device.device_wait_idle().result()?;
        }
        if let Some(mut object) = self.objects.remove(&id) {
            object.vertices.free(&self.vk_device, &mut self.allocator)?;
            object.indices.free(&self.vk_device, &mut self.allocator)?;
        }
        Ok(())
    }

    pub fn set_transform(&mut self, id: ObjectId, transform: Matrix4<f32>) {
        if let Some(object) = self.objects.get_mut(&id) {
            object.transform = transform;
        }
    }
}
