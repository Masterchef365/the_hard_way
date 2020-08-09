use crate::hardware_query::HardwareSelection;
use crate::pipeline::{MaterialId, Pipeline};
use anyhow::Result;
use erupt::{
    extensions::{ext_debug_utils, khr_surface, khr_swapchain},
    vk1_0 as vk, DeviceLoader, InstanceLoader,
};
use std::collections::HashMap;

struct Swapchain {
    swapchain: khr_swapchain::SwapchainKHR,
    render_pass: vk::RenderPass,
    extent: vk::Extent2D,
    pipelines: HashMap<MaterialId, Pipeline>,
    images: Vec<SwapChainImage>,
    freed: bool,
}

struct SwapChainImage {
    framebuffer: vk::Framebuffer,
    image_view: vk::ImageView,
    //command_buffers: vk::CommandBuffer,
    in_flight: vk::Fence,
    freed: bool,
}

impl Swapchain {
    pub fn new(
        instance: &InstanceLoader,
        device: &DeviceLoader,
        hardware: &HardwareSelection,
        surface: khr_surface::SurfaceKHR,
    ) -> Result<Self> {
        let surface_caps = unsafe {
            instance.get_physical_device_surface_capabilities_khr(
                hardware.physical_device,
                surface,
                None,
            )
        }
        .result()?;

        let mut image_count = surface_caps.min_image_count + 1;
        if surface_caps.max_image_count > 0 && image_count > surface_caps.max_image_count {
            image_count = surface_caps.max_image_count;
        }

        let create_info = khr_swapchain::SwapchainCreateInfoKHRBuilder::new()
            .surface(surface)
            .min_image_count(image_count)
            .image_format(hardware.format.format)
            .image_color_space(hardware.format.color_space)
            .image_extent(surface_caps.current_extent)
            .image_array_layers(1)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .pre_transform(surface_caps.current_transform)
            .composite_alpha(khr_surface::CompositeAlphaFlagBitsKHR::OPAQUE_KHR)
            .present_mode(hardware.present_mode)
            .clipped(true)
            .old_swapchain(khr_swapchain::SwapchainKHR::null());

        let swapchain =
            unsafe { device.create_swapchain_khr(&create_info, None, None) }.result()?;
        let swapchain_images =
            unsafe { device.get_swapchain_images_khr(swapchain, None) }.result()?;

        // Render pass
        let attachments = [vk::AttachmentDescriptionBuilder::new()
            .format(hardware.format.format)
            .samples(vk::SampleCountFlagBits::_1)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE)
            .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
            .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .final_layout(vk::ImageLayout::PRESENT_SRC_KHR)];

        let color_attachment_refs = [vk::AttachmentReferenceBuilder::new()
            .attachment(0)
            .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)];
        let subpasses = [vk::SubpassDescriptionBuilder::new()
            .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
            .color_attachments(&color_attachment_refs)];
        let dependencies = [vk::SubpassDependencyBuilder::new()
            .src_subpass(vk::SUBPASS_EXTERNAL)
            .dst_subpass(0)
            .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            .src_access_mask(vk::AccessFlags::empty())
            .dst_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)];

        let create_info = vk::RenderPassCreateInfoBuilder::new()
            .attachments(&attachments)
            .subpasses(&subpasses)
            .dependencies(&dependencies);

        let render_pass = unsafe { device.create_render_pass(&create_info, None, None) }.unwrap();

        let images = swapchain_images
            .iter()
            .map(|image| {
                SwapChainImage::new(
                    &device,
                    render_pass,
                    image,
                    surface_caps.current_extent,
                    hardware,
                )
            })
            .collect::<Result<Vec<_>>>();

        todo!();
    }

    pub fn free(&mut self, device: &DeviceLoader) {
        unsafe { 
            device.destroy_swapchain_khr(Some(self.swapchain), None);
            device.destroy_render_pass(Some(self.render_pass), None);
        }
    }
}

impl SwapChainImage {
    pub fn new(
        device: &DeviceLoader,
        render_pass: vk::RenderPass,
        swapchain_image: &vk::Image,
        extent: vk::Extent2D,
        hardware: &HardwareSelection,
    ) -> Result<Self> {
        let in_flight = vk::Fence::null();

        let create_info = vk::ImageViewCreateInfoBuilder::new()
            .image(*swapchain_image)
            .view_type(vk::ImageViewType::_2D)
            .format(hardware.format.format)
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
                    .layer_count(1)
                    .build(),
            );

        let image_view = unsafe { device.create_image_view(&create_info, None, None) }.result()?;
        let attachments = [image_view];
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
            device.destroy_fence(Some(self.in_flight), None);
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
