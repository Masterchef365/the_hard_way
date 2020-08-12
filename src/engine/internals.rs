use super::Engine;
use crate::swapchain::Swapchain;
use anyhow::Result;

impl Engine {
    pub(crate) fn invalidate_swapchain(&mut self) -> Result<()> {
        if let Some(swapchain) = &mut self.swapchain {
            swapchain.free(&self.device, self.command_pool);
        }
        self.swapchain = None;
        Ok(())
    }
}
