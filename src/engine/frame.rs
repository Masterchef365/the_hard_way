use super::Engine;
use crate::swapchain::Swapchain;
use anyhow::Result;
use erupt::vk1_0 as vk;
use openxr as xr;

impl Engine {
    pub fn next_frame(
        &mut self,
        xr_instance: &xr::Instance,
        xr_session: &xr::Session<xr::Vulkan>,
        xr_system: xr::SystemId,
        _time: f32,
    ) -> Result<()> {
        // Wait for OpenXR to signal it has a frame ready
        let xr_frame_state = self.frame_wait.as_mut().unwrap().wait()?;
        self.frame_stream.as_mut().unwrap().begin()?;

        if !xr_frame_state.should_render {
            self.frame_stream.as_mut().unwrap().end(
                xr_frame_state.predicted_display_time,
                xr::EnvironmentBlendMode::OPAQUE,
                &[],
            )?;
            return Ok(());
        }

        // Recreate the swapchain if necessary
        if self.swapchain.is_none() {
            let mut swapchain = Swapchain::new(
                xr_instance,
                xr_session,
                xr_system,
                &self.vk_device,
                &mut self.allocator,
            )?;
            for (id, material) in self.materials.iter() {
                swapchain.add_pipeline(
                    &self.vk_device,
                    self.descriptor_set_layout,
                    *id,
                    material,
                )?;
            }
            self.swapchain = Some(swapchain);
        }
        let swapchain = self.swapchain.as_mut().unwrap();
        let render_pass = swapchain.render_pass; // These two needed for borrowing reasons
        let extent = swapchain.extent;

        // Wait for the next frame to become available
        let (frame_idx, frame) = self.frame_sync.next_frame(&self.vk_device)?;

        // Wait for a swapchain image to become available and assign it the current frame
        let (swapchain_image_idx, swapchain_image) =
            swapchain.next_image(&self.vk_device, frame)?;

        // Reset and write command buffers for this frame
        let command_buffer = self.command_buffers[frame_idx];
        let descriptor_set = self.descriptor_sets[frame_idx];
        unsafe {
            self.vk_device
                .reset_command_buffer(command_buffer, None)
                .result()?;

            let begin_info = vk::CommandBufferBeginInfoBuilder::new();
            self.vk_device
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
                    },
                },
            ];

            let begin_info = vk::RenderPassBeginInfoBuilder::new()
                .framebuffer(swapchain_image.framebuffer)
                .render_pass(render_pass)
                .render_area(vk::Rect2D {
                    offset: vk::Offset2D { x: 0, y: 0 },
                    extent,
                })
                .clear_values(&clear_values);

            self.vk_device.cmd_begin_render_pass(
                command_buffer,
                &begin_info,
                vk::SubpassContents::INLINE,
            );

            for (pipeline_id, pipeline) in &swapchain.pipelines {
                self.vk_device.cmd_bind_pipeline(
                    command_buffer,
                    vk::PipelineBindPoint::GRAPHICS,
                    pipeline.pipeline,
                );

                self.vk_device.cmd_bind_descriptor_sets(
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
                    self.vk_device.cmd_bind_vertex_buffers(
                        command_buffer,
                        0,
                        &[object.vertices.buffer],
                        &[0],
                    );

                    self.vk_device.cmd_bind_index_buffer(
                        command_buffer,
                        object.indices.buffer,
                        0,
                        vk::IndexType::UINT16,
                    );

                    let descriptor_sets = [self.descriptor_sets[frame_idx]];
                    self.vk_device.cmd_bind_descriptor_sets(
                        command_buffer,
                        vk::PipelineBindPoint::GRAPHICS,
                        pipeline.pipeline_layout,
                        0,
                        &descriptor_sets,
                        &[],
                    );

                    self.vk_device.cmd_push_constants(
                        command_buffer,
                        pipeline.pipeline_layout,
                        vk::ShaderStageFlags::VERTEX,
                        0,
                        std::mem::size_of::<[[f32; 4]; 4]>() as u32,
                        object.transform.data.as_ptr() as _,
                    );

                    self.vk_device
                        .cmd_draw_indexed(command_buffer, object.n_indices, 1, 0, 0, 0);
                }
            }

            self.vk_device.cmd_end_render_pass(command_buffer);

            self.vk_device.end_command_buffer(command_buffer).result()?;
        }

        // Get views
        let (_, views) = xr_session.locate_views(
            xr::ViewConfigurationType::PRIMARY_STEREO,
            xr_frame_state.predicted_display_time,
            self.stage.as_ref().unwrap(),
        )?;

        //let unit = nalgebra::Quaternion::new()

        // Submit to the queue
        let command_buffers = [command_buffer];
        let submit_info = vk::SubmitInfoBuilder::new().command_buffers(&command_buffers);
        unsafe {
            self.vk_device
                .reset_fences(&[frame.in_flight_fence])
                .result()?; // TODO: Move this into the swapchain next_image
            self.vk_device
                .queue_submit(self.queue, &[submit_info], Some(frame.in_flight_fence))
                .result()?;
        }

        // Present to swapchain
        swapchain.swapchain.release_image()?;

        // Upload camera matrix TODO: Only map once, never unmap!
        self.camera_ubos[frame_idx].map(
            &self.vk_device,
            &[*matrix_from_view(&views[0], extent).as_ref()],
        )?;

        // Tell OpenXR what to present for this frame
        let rect = xr::Rect2Di {
            offset: xr::Offset2Di { x: 0, y: 0 },
            extent: xr::Extent2Di {
                width: swapchain.extent.width as _,
                height: swapchain.extent.height as _,
            },
        };
        self.frame_stream.as_mut().unwrap().end(
            xr_frame_state.predicted_display_time,
            xr::EnvironmentBlendMode::OPAQUE,
            &[&xr::CompositionLayerProjection::new()
                .space(self.stage.as_ref().unwrap())
                .views(&[
                    xr::CompositionLayerProjectionView::new()
                        .pose(views[0].pose)
                        .fov(views[0].fov)
                        .sub_image(
                            xr::SwapchainSubImage::new()
                                .swapchain(&swapchain.swapchain)
                                .image_array_index(0)
                                .image_rect(rect),
                        ),
                    xr::CompositionLayerProjectionView::new()
                        .pose(views[1].pose)
                        .fov(views[1].fov)
                        .sub_image(
                            xr::SwapchainSubImage::new()
                                .swapchain(&swapchain.swapchain)
                                .image_array_index(1)
                                .image_rect(rect),
                        ),
                ])],
        )?;

        Ok(())
    }
}

use nalgebra::{Matrix4, Quaternion, Vector3, Unit};
fn matrix_from_view(view: &xr::View, extent: vk::Extent2D) -> Matrix4<f32> {
    let proj = projection_from_fov(&view.fov, 0.001, 100.0);
    let view = view_from_pose(&view.pose);
    println!("{}", proj);
    proj * view
}

// Ported from:
// https://gitlab.freedesktop.org/monado/demos/xrgears/-/blob/master/src/main.cpp
fn view_from_pose(pose: &xr::Posef) -> Matrix4<f32> {
    let quat = pose.orientation;
    let quat = nalgebra::Quaternion::new(-quat.w, quat.x, -quat.y, quat.z);
    let quat = Unit::try_new(quat, 0.0).expect("Not a unit quaternion");
    let mut rotation = quat.to_rotation_matrix().to_homogeneous();

    let position = pose.position;
    let position = Vector3::new(position.x, -position.y, position.z);
    let translation = Matrix4::new_translation(&position);

    let view = translation * rotation;
    let inv = view.try_inverse().expect("Matrix didn't invert");
    inv
}

fn projection_from_fov(fov: &xr::Fovf, near: f32, far: f32) -> Matrix4<f32> {
    let tan_left = fov.angle_left.tan();
    let tan_right = fov.angle_right.tan();

    let tan_up = fov.angle_up.tan();
    let tan_down = fov.angle_down.tan();

    let tan_width = tan_right - tan_left;
    let tan_height = tan_up - tan_down;

    let a11 = 2.0 / tan_width;
    let a22 = 2.0 / tan_height;

    let a31 = (tan_right + tan_left) / tan_width;
    let a32 = (tan_up + tan_down) / tan_height;
    let a33 = -far / (far - near);

    let a43 = -(far * near) / (far - near);
    let cols = vec![
        a11, 0.0, 0.0, 0.0, 
        0.0, a22, 0.0, 0.0,
        a31, a32, a33, -1.0,
        0.0, 0.0, a43, 0.0,
    ];
    Matrix4::from_vec(cols)
}
