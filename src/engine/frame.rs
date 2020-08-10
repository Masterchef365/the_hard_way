use super::{Engine, Object};
use crate::swapchain::Swapchain;
use anyhow::Result;
use erupt::{
    extensions::{ext_debug_utils, khr_surface, khr_swapchain},
    vk1_0 as vk, DeviceLoader, InstanceLoader,
};
use nalgebra::{Matrix4, Point2, Point3};

impl Engine {
    pub fn next_frame(
        &mut self,
        objects: &[Object],
        camera: &Matrix4<f32>,
        time: f32,
    ) -> Result<()> {
        // Recreate the swapchain if necessary
        if self.swapchain.is_none() {
            self.swapchain = Some(Swapchain::new(
                &self.instance,
                &self.device,
                &self.hardware,
                &self.materials,
                self.surface,
                self.command_pool,
            )?);
        }
        let swapchain = self.swapchain.as_mut().unwrap();

        // Wait for the next frame to become available
        let frame = self.frame_sync.next_frame(&self.device);

        // Wait for a swapchain image to become available and assign it the current frame
        let swapchain_image = swapchain.next_image(&self.device, frame);

        // Swapchain is out of date, reconstruct on the next pass
        let (swapchain_image_idx, swapchain_image) = match swapchain_image {
            Some(s) => s,
            None => {
                swapchain.free(&self.device, self.command_pool);
                self.swapchain = None;
                return Ok(());
            }
        };

        //TODO: COMMAND BUFFER REWRITE GOES HERE
        let command_buffer = swapchain_image.command_buffer;
        unsafe {
            let begin_info = vk::CommandBufferBeginInfoBuilder::new();
            self.device.reset_command_buffer(command_buffer, None);
            self.device
                .begin_command_buffer(command_buffer, &begin_info)
                .result()?;
            self.device.end_command_buffer(command_buffer).result()?;
        }

        // Submit to the queue
        let wait_semaphores = [frame.image_available];
        let command_buffers = [swapchain_image.command_buffer];
        let signal_semaphores = [frame.render_finished];
        let submit_info = vk::SubmitInfoBuilder::new()
            .wait_semaphores(&wait_semaphores)
            .wait_dst_stage_mask(&[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT])
            .command_buffers(&command_buffers)
            .signal_semaphores(&signal_semaphores);
        unsafe {
            self.device.reset_fences(&[frame.in_flight_fence]).unwrap(); // TODO: Move this into the swapchain next_image
            self.device
                .queue_submit(self.queue, &[submit_info], Some(frame.in_flight_fence))
                .unwrap()
        }

        let swapchains = [swapchain.swapchain];
        let image_indices = [swapchain_image_idx];
        let present_info = khr_swapchain::PresentInfoKHRBuilder::new()
            .wait_semaphores(&signal_semaphores)
            .swapchains(&swapchains)
            .image_indices(&image_indices);

        let queue_result = unsafe { self.device.queue_present_khr(self.queue, &present_info) };

        if queue_result.raw == vk::Result::ERROR_OUT_OF_DATE_KHR {
            swapchain.free(&self.device, self.command_pool);
            self.swapchain = None;
            return Ok(());
        } else {
            queue_result.unwrap();
        };

        Ok(())
    }
}
