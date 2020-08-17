use crate::engine::MaterialId;
use crate::frame_sync::Frame;
use crate::pipeline::{Material, Pipeline};
use anyhow::Result;
use erupt::{
    utils::allocator::{Allocation, Allocator, MemoryTypeFinder},
    vk1_0 as vk, DeviceLoader,
    vk1_1,
};
use std::collections::HashMap;
use openxr as xr;

pub const VIEW_COUNT: u32 = 2;
pub const COLOR_FORMAT: vk::Format = vk::Format::B8G8R8A8_SRGB;

/// Describes everything that changes when the swapchain changes. This isn't ideal, and will likely
/// be broken up later.
pub struct Swapchain {
    pub swapchain: xr::Swapchain<xr::Vulkan>,
    pub render_pass: vk::RenderPass,
    pub extent: vk::Extent2D,
    pub pipelines: HashMap<MaterialId, Pipeline>,
    pub depth_image: vk::Image,
    pub depth_image_mem: Option<Allocation<vk::Image>>,
    pub depth_image_view: vk::ImageView,
    images: Vec<SwapChainImage>,
    freed: bool,
}

pub struct SwapChainImage {
    pub framebuffer: vk::Framebuffer,
    pub image_view: vk::ImageView,
    /// Whether or not the frame which this swapchain image is dependent on is in flight or not
    pub in_flight: vk::Fence,
    freed: bool,
}

impl Swapchain {
    /// Returns None if the swapchain is out of date
    pub fn next_image(
        &mut self,
        device: &DeviceLoader,
        frame: &Frame,
    ) -> Result<(u32, &mut SwapChainImage)> {
        let image_index = self.swapchain.acquire_image()?;
        self.swapchain.wait_image(xr::Duration::INFINITE)?;

        let image = &mut self.images[image_index as usize];

        // Wait until the frame associated with this swapchain image is finisehd rendering, if any
        // May be null if no frames have flowed just yet
        if !image.in_flight.is_null() {
            unsafe { device.wait_for_fences(&[image.in_flight], true, u64::MAX) }.result()?;
        }

        // Associate this swapchain image with the given frame. When the frame is finished, this
        // swapchain image will know (see above) when this image is rendered.
        image.in_flight = frame.in_flight_fence;

        Ok((image_index, image))
    }

    pub fn new(
        xr_instance: &xr::Instance,
        session: &xr::Session<xr::Vulkan>,
        system: xr::SystemId,
        device: &DeviceLoader,
        allocator: &mut Allocator,
    ) -> Result<Self> {
        let views = xr_instance
            .enumerate_view_configuration_views(
                system,
                xr::ViewConfigurationType::PRIMARY_STEREO,
            )
            .unwrap();

        let extent = vk::Extent2D {
            width: views[0].recommended_image_rect_width,
            height: views[0].recommended_image_rect_height,
        };
        let swapchain = session
            .create_swapchain(&xr::SwapchainCreateInfo {
                create_flags: xr::SwapchainCreateFlags::EMPTY,
                usage_flags: xr::SwapchainUsageFlags::COLOR_ATTACHMENT
                    | xr::SwapchainUsageFlags::SAMPLED,
                    format: COLOR_FORMAT.0 as _,
                    sample_count: 1,
                    width: extent.width,
                    height: extent.height,
                    face_count: 1,
                    array_size: VIEW_COUNT,
                    mip_count: 1,
            })
        .unwrap();

        let swapchain_images = swapchain.enumerate_images().unwrap();

        // Create depth image
        let depth_format = vk::Format::D32_SFLOAT;
        let create_info = vk::ImageCreateInfoBuilder::new()
            .image_type(vk::ImageType::_2D)
            .extent(
                vk::Extent3DBuilder::new()
                    .width(extent.width)
                    .height(extent.height)
                    .depth(1)
                    .build(),
            )
            .mip_levels(1)
            .array_layers(1)
            .format(depth_format)
            .tiling(vk::ImageTiling::OPTIMAL)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .usage(vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT)
            .samples(vk::SampleCountFlagBits::_1)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);
        let depth_image = unsafe { device.create_image(&create_info, None, None) }.result()?;

        let depth_image_mem = allocator.allocate(device, depth_image, MemoryTypeFinder::gpu_only()).result()?;

        let create_info = vk::ImageViewCreateInfoBuilder::new()
            .image(depth_image)
            .view_type(vk::ImageViewType::_2D)
            .format(depth_format)
            .subresource_range(
                vk::ImageSubresourceRangeBuilder::new()
                    .aspect_mask(vk::ImageAspectFlags::DEPTH)
                    .base_mip_level(0)
                    .level_count(1)
                    .base_array_layer(0)
                    .layer_count(1)
                    .build(),
            );
        let depth_image_view =
            unsafe { device.create_image_view(&create_info, None, None) }.result()?;

        // Render pass
        let color_attachment = vk::AttachmentDescriptionBuilder::new()
            .format(COLOR_FORMAT)
            .samples(vk::SampleCountFlagBits::_1)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE)
            .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
            .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .final_layout(vk::ImageLayout::PRESENT_SRC_KHR);

        let depth_attachment = vk::AttachmentDescriptionBuilder::new()
            .format(depth_format)
            .samples(vk::SampleCountFlagBits::_1)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::DONT_CARE)
            .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
            .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .final_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL);

        let attachments = [color_attachment, depth_attachment];

        let color_attachment_refs = [vk::AttachmentReferenceBuilder::new()
            .attachment(0)
            .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)];

        let depth_attachment_ref = vk::AttachmentReferenceBuilder::new()
            .attachment(1)
            .layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
            .build();

        let subpasses = [vk::SubpassDescriptionBuilder::new()
            .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
            .color_attachments(&color_attachment_refs)
            .depth_stencil_attachment(&depth_attachment_ref)];

        let dependencies = [vk::SubpassDependencyBuilder::new()
            .src_subpass(vk::SUBPASS_EXTERNAL)
            .dst_subpass(0)
            .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            .src_access_mask(vk::AccessFlags::empty())
            .dst_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)];

        let mut create_info = vk::RenderPassCreateInfoBuilder::new()
            .attachments(&attachments)
            .subpasses(&subpasses)
            .dependencies(&dependencies);

        let view_mask = [!(!0 << 2)];
        let mut multiview = vk1_1::RenderPassMultiviewCreateInfoBuilder::new()
            .view_masks(&view_mask)
            .correlation_masks(&view_mask)
            .build();

        create_info.p_next = &mut multiview as *mut _ as _;

        let render_pass =
            unsafe { device.create_render_pass(&create_info, None, None) }.result()?;

        // Build swapchain image views and buffers
        let images = swapchain_images
            .iter()
            .map(|&image| {
                SwapChainImage::new(
                    &device,
                    render_pass,
                    vk::Image(image),
                    extent,
                    depth_image_view,
                )
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(Self {
            swapchain,
            render_pass,
            extent,
            pipelines: Default::default(),
            images,
            depth_image,
            depth_image_mem: Some(depth_image_mem),
            depth_image_view,
            freed: false,
        })
    }

    pub fn add_pipeline(
        &mut self,
        device: &DeviceLoader,
        descriptor_set_layout: vk::DescriptorSetLayout,
        id: MaterialId,
        material: &Material,
    ) -> Result<()> {
        self.pipelines.insert(
            id,
            Pipeline::new(
                &device,
                material,
                self.render_pass,
                descriptor_set_layout,
                self.extent,
            )?,
        );
        Ok(())
    }

    pub fn remove_pipeline(&mut self, device: &DeviceLoader, id: MaterialId) {
        if let Some(mut mat) = self.pipelines.remove(&id) {
            mat.free(device);
        }
    }

    pub fn free(&mut self, device: &DeviceLoader, allocator: &mut Allocator) -> Result<()> {
        unsafe {
            device.device_wait_idle().result()?;
            device.destroy_image_view(Some(self.depth_image_view), None);
        }

        allocator.free(device, self.depth_image_mem.take().unwrap());

        for pipeline in self.pipelines.values_mut() {
            pipeline.free(device);
        }

        for mut image in self.images.drain(..) {
            image.free(device);
        }

        unsafe {
            device.destroy_render_pass(Some(self.render_pass), None);
        }
        self.freed = true;
        Ok(())
    }
}

impl SwapChainImage {
    pub fn new(
        device: &DeviceLoader,
        render_pass: vk::RenderPass,
        swapchain_image: vk::Image,
        extent: vk::Extent2D,
        depth_image_view: vk::ImageView,
    ) -> Result<Self> {
        let in_flight = vk::Fence::null();

        let create_info = vk::ImageViewCreateInfoBuilder::new()
            .image(swapchain_image)
            .view_type(vk::ImageViewType::_2D)
            .format(COLOR_FORMAT)
            .components(vk::ComponentMapping {
                r: vk::ComponentSwizzle::IDENTITY,
                g: vk::ComponentSwizzle::IDENTITY,
                b: vk::ComponentSwizzle::IDENTITY,
                a: vk::ComponentSwizzle::IDENTITY,
            })
            .subresource_range(
                vk::ImageSubresourceRangeBuilder::new()
                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                    .base_mip_level(0)
                    .level_count(1)
                    .base_array_layer(0)
                    .layer_count(VIEW_COUNT)
                    .build(),
            );

        let image_view = unsafe { device.create_image_view(&create_info, None, None) }.result()?;

        let attachments = [
            image_view,
            depth_image_view,
        ];
        let create_info = vk::FramebufferCreateInfoBuilder::new()
            .render_pass(render_pass)
            .attachments(&attachments)
            .width(extent.width)
            .height(extent.height)
            .layers(1);

        let framebuffer =
            unsafe { device.create_framebuffer(&create_info, None, None) }.result()?;

        Ok(Self {
            framebuffer,
            image_view,
            in_flight,
            freed: false,
        })
    }

    pub fn free(&mut self, device: &DeviceLoader) {
        unsafe {
            device.destroy_framebuffer(Some(self.framebuffer), None);
            device.destroy_image_view(Some(self.image_view), None);
        }
        self.freed = true;
    }
}

impl Drop for SwapChainImage {
    fn drop(&mut self) {
        if !self.freed {
            panic!("Swapchain image dropped before it was freed");
        }
    }
}

impl Drop for Swapchain {
    fn drop(&mut self) {
        if !self.freed {
            panic!("Swapchain dropped before it was freed");
        }
    }
}
