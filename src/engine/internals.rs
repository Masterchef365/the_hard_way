use super::Engine;
use anyhow::Result;

impl Engine {
    pub(crate) fn invalidate_swapchain(&mut self) -> Result<()> {
        if let Some(swapchain) = &mut self.swapchain {
            swapchain.free(&self.vk_device, &mut self.allocator)?;
        }
        self.swapchain = None;
        Ok(())
    }
}
