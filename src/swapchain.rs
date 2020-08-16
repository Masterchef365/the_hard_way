use crate::engine::MaterialId;
use crate::frame_sync::Frame;
use crate::hardware_query::HardwareSelection;
use crate::pipeline::{Material, Pipeline};
use anyhow::Result;
use erupt::{
    extensions::{khr_surface, khr_swapchain},
    utils::allocator::{Allocation, Allocator, MemoryTypeFinder},
    vk1_0 as vk, DeviceLoader, InstanceLoader,
};
use std::collections::HashMap;

/// Describes everything that changes when the swapchain changes. This isn't ideal, and will likely
/// be broken up later.
pub struct Swapchain {
    pub swapchain: khr_swapchain::SwapchainKHR,
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
    ) -> Result<Option<(u32, &mut SwapChainImage)>> {
        let image_index = unsafe {
            device.acquire_next_image_khr(
                self.swapchain,
                u64::MAX,
                Some(frame.image_available),
                None,
                None,
            )
        };

        let image_index = if image_index.raw == vk::Result::ERROR_OUT_OF_DATE_KHR {
            return Ok(None);
        } else {
            image_index.unwrap()
        };

        let image = &mut self.images[image_index as usize];

        // Wait until the frame associated with this swapchain image is finisehd rendering, if any
        // May be null if no frames have flowed just yet
        if !image.in_flight.is_null() {
            unsafe { device.wait_for_fences(&[image.in_flight], true, u64::MAX) }.result()?;
        }

        // Associate this swapchain image with the given frame. When the frame is finished, this
        // swapchain image will know (see above) when this image is rendered.
        image.in_flight = frame.in_flight_fence;

        Ok(Some((image_index, image)))
    }

    pub fn new(
        instance: &InstanceLoader,
        device: &DeviceLoader,
        hardware: &HardwareSelection,
        surface: khr_surface::SurfaceKHR,
        allocator: &mut Allocator,
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

        // Create depth image
        let depth_format = vk::Format::D32_SFLOAT;
        let create_info = vk::ImageCreateInfoBuilder::new()
            .image_type(vk::ImageType::_2D)
            .extent(
                vk::Extent3DBuilder::new()
                    .width(surface_caps.current_extent.width)
                    .height(surface_caps.current_extent.height)
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

        // Build the actual swapchain
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
        let color_attachment = vk::AttachmentDescriptionBuilder::new()
            .format(hardware.format.format)
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

        let create_info = vk::RenderPassCreateInfoBuilder::new()
            .attachments(&attachments)
            .subpasses(&subpasses)
            .dependencies(&dependencies);

        let render_pass =
            unsafe { device.create_render_pass(&create_info, None, None) }.result()?;

        // Build swapchain image views and buffers
        let images = swapchain_images
            .iter()
            .map(|&image| {
                SwapChainImage::new(
                    &device,
                    render_pass,
                    image,
                    surface_caps.current_extent,
                    hardware,
                    depth_image_view,
                )
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(Self {
            swapchain,
            render_pass,
            extent: surface_caps.current_extent,
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
            device.destroy_swapchain_khr(Some(self.swapchain), None);
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
        hardware: &HardwareSelection,
        depth_image_view: vk::ImageView,
    ) -> Result<Self> {
        let in_flight = vk::Fence::null();

        let create_info = vk::ImageViewCreateInfoBuilder::new()
            .image(swapchain_image)
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
