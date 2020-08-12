use crate::Engine;

impl Drop for Engine {
    fn drop(&mut self) {
        unsafe {
            for material in self.materials.values_mut() {
                material.free(&self.device);
            }
            if let Some(swapchain) = &mut self.swapchain {
                swapchain.free(&self.device, self.command_pool);
            }
            self.frame_sync.free(&self.device);
            self.device.destroy_command_pool(Some(self.command_pool), None);
            self.device.destroy_device(None);
            self.instance.destroy_surface_khr(Some(self.surface), None);
            self.instance.destroy_instance(None);
        }
    }
}

