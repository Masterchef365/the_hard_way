use super::Engine;
use crate::swapchain::Swapchain;
use anyhow::Result;

impl Engine {
    fn recreate_swapchain(&mut self) -> Result<()> {
        if let Some(swapchain) = &mut self.swapchain {
            swapchain.free(&self.device, self.command_pool);
        }
        self.swapchain = Some(Swapchain::new(
                &self.instance,
                &self.device,
                &self.hardware,
                &self.materials,
                self.surface,
                self.command_pool,
        )?);
        Ok(())
    }
}
