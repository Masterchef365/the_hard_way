use crate::Engine;

impl Drop for Engine {
    fn drop(&mut self) {
        unsafe {
            self.instance.destroy_surface_khr(Some(self.surface), None);
            self.instance.destroy_instance(None)
        }
    }
}

