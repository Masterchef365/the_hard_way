use crate::Engine;

impl Drop for Engine {
    fn drop(&mut self) {
        unsafe {
            let ids = self.objects.keys().copied().collect::<Vec<_>>();
            for id in ids {
                self.remove_object(id).unwrap();
            }
            for material in self.materials.values_mut() {
                material.free(&self.device);
            }
            if let Some(swapchain) = &mut self.swapchain {
                swapchain.free(&self.device, &mut self.allocator).unwrap();
            }
            for ubo in &mut self.camera_ubos {
                ubo.free(&self.device, &mut self.allocator).unwrap();
            }
            self.frame_sync.free(&self.device);
            self.device.destroy_descriptor_set_layout(Some(self.descriptor_set_layout), None);
            self.device.destroy_descriptor_pool(Some(self.descriptor_pool), None);
            self.device.free_command_buffers(self.command_pool, &self.command_buffers);
            self.device.destroy_command_pool(Some(self.command_pool), None);
            self.device.destroy_device(None);
            self.instance.destroy_surface_khr(Some(self.surface), None);
            self.instance.destroy_instance(None);
        }
    }
}

