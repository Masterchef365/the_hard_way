use super::{Engine, RealtimeUBO};
use crate::camera::Camera;
use crate::swapchain::Swapchain;
use anyhow::Result;
use erupt::{extensions::khr_swapchain, vk1_0 as vk};

impl Engine {
    pub fn next_frame(&mut self, camera: &Camera, time: f32) -> Result<()> {
        // Recreate the swapchain if necessary
        if self.swapchain.is_none() {
            let mut swapchain = Swapchain::new(
                &self.instance,
                &self.device,
                &self.hardware,
                self.surface,
                &mut self.allocator,
            )?;
            for (id, material) in self.materials.iter() {
                swapchain.add_pipeline(&self.device, self.descriptor_set_layout, *id, material)?;
            }
            self.swapchain = Some(swapchain);
        }
        let swapchain = self.swapchain.as_mut().unwrap();
        let render_pass = swapchain.render_pass; // These two needed for borrowing reasons
        let extent = swapchain.extent;
        let aspect = extent.width as f32 / extent.height as f32;

        // Wait for the next frame to become available
        let (frame_idx, frame) = self.frame_sync.next_frame(&self.device)?;

        // Wait for a swapchain image to become available and assign it the current frame
        let swapchain_image = swapchain.next_image(&self.device, frame)?;

        // Swapchain is out of date, reconstruct on the next pass
        let (swapchain_image_idx, swapchain_image) = match swapchain_image {
            Some(s) => s,
            None => {
                self.invalidate_swapchain()?;
                return Ok(());
            }
        };

        // Upload camera matrix and time
        let realtime_ubo = RealtimeUBO::new(&camera.matrix(aspect), time);

        self.realtime_ubo[frame_idx].map(&self.device, &[realtime_ubo])?;

        // Reset and write command buffers for this frame
        let command_buffer = self.command_buffers[frame_idx];
        let descriptor_set = self.descriptor_sets[frame_idx];
        unsafe {
            self.device
                .reset_command_buffer(command_buffer, None)
                .result()?;

            let begin_info = vk::CommandBufferBeginInfoBuilder::new();
            self.device
                .begin_command_buffer(command_buffer, &begin_info)
                .result()?;

            // Set render pass
            let clear_values = [
            vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: [0.0, 0.0, 0.0, 1.0],
                },
            },
            vk::ClearValue {
                depth_stencil: vk::ClearDepthStencilValue {
                    depth: 1.0,
                    stencil: 0,
                }
            }];

            let begin_info = vk::RenderPassBeginInfoBuilder::new()
                .framebuffer(swapchain_image.framebuffer)
                .render_pass(render_pass)
                .render_area(vk::Rect2D {
                    offset: vk::Offset2D { x: 0, y: 0 },
                    extent,
                })
                .clear_values(&clear_values);

            self.device.cmd_begin_render_pass(
                command_buffer,
                &begin_info,
                vk::SubpassContents::INLINE,
            );

            for (pipeline_id, pipeline) in &swapchain.pipelines {
                self.device.cmd_bind_pipeline(
                    command_buffer,
                    vk::PipelineBindPoint::GRAPHICS,
                    pipeline.pipeline,
                );

                self.device.cmd_bind_descriptor_sets(
                    command_buffer,
                    vk::PipelineBindPoint::GRAPHICS,
                    pipeline.pipeline_layout,
                    0,
                    &[descriptor_set],
                    &[],
                );

                for object in self
                    .objects
                    .values_mut()
                    .filter(|o| o.material == *pipeline_id)
                {
                    self.device.cmd_bind_vertex_buffers(
                        command_buffer,
                        0,
                        &[object.vertices.buffer],
                        &[0],
                    );

                    self.device.cmd_bind_index_buffer(
                        command_buffer,
                        object.indices.buffer,
                        0,
                        vk::IndexType::UINT16,
                    );

                    let descriptor_sets = [self.descriptor_sets[frame_idx]];
                    self.device.cmd_bind_descriptor_sets(
                        command_buffer,
                        vk::PipelineBindPoint::GRAPHICS,
                        pipeline.pipeline_layout,
                        0,
                        &descriptor_sets,
                        &[],
                    );

                    self.device.cmd_push_constants(
                        command_buffer,
                        pipeline.pipeline_layout,
                        vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                        0,
                        std::mem::size_of::<RealtimeUBO>() as u32,
                        object.transform.data.as_ptr() as _,
                    );

                    self.device
                        .cmd_draw_indexed(command_buffer, object.n_indices, 1, 0, 0, 0);
                }
            }

            self.device.cmd_end_render_pass(command_buffer);

            self.device.end_command_buffer(command_buffer).result()?;
        }

        // Submit to the queue
        let wait_semaphores = [frame.image_available];
        let command_buffers = [command_buffer];
        let signal_semaphores = [frame.render_finished];
        let submit_info = vk::SubmitInfoBuilder::new()
            .wait_semaphores(&wait_semaphores)
            .wait_dst_stage_mask(&[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT])
            .command_buffers(&command_buffers)
            .signal_semaphores(&signal_semaphores);
        unsafe {
            self.device
                .reset_fences(&[frame.in_flight_fence])
                .result()?; // TODO: Move this into the swapchain next_image
            self.device
                .queue_submit(self.queue, &[submit_info], Some(frame.in_flight_fence))
                .result()?;
        }

        // Present to swapchain
        let swapchains = [swapchain.swapchain];
        let image_indices = [swapchain_image_idx];
        let present_info = khr_swapchain::PresentInfoKHRBuilder::new()
            .wait_semaphores(&signal_semaphores)
            .swapchains(&swapchains)
            .image_indices(&image_indices);

        let queue_result = unsafe { self.device.queue_present_khr(self.queue, &present_info) };

        if queue_result.raw == vk::Result::ERROR_OUT_OF_DATE_KHR {
            self.invalidate_swapchain()?;
            return Ok(());
        } else {
            queue_result.result()?;
        };

        Ok(())
    }
}
