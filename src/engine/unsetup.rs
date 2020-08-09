use crate::Engine;

impl Drop for Engine {
    fn drop(&mut self) {
        unsafe {
            self.frame_sync.free(&self.device);
            self.device.destroy_command_pool(Some(self.command_pool), None);
            self.device.destroy_device(None);
            self.instance.destroy_surface_khr(Some(self.surface), None);
            self.instance.destroy_instance(None);
        }
    }
}

