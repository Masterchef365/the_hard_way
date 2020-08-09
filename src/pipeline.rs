use erupt::{vk1_0 as vk, DeviceLoader};
use crate::vertex::Vertex;
use anyhow::Result;

pub struct MaterialId(u32);

/// Represents a backing pipeline that can render an object
/// with the material from which it was created.
pub struct Pipeline {
    pipeline: vk::Pipeline,
    freed: bool,
}

/// Represents a set of drawing parameters to be turned into a pipeline
pub struct Material {
    draw_type: DrawType,
    vertex: vk::ShaderModule,
    fragment: vk::ShaderModule,
    descriptor_set_layout: vk::DescriptorSetLayout,
    freed: bool,
}

pub enum DrawType {
    Triangles,
    Lines,
    Points,
}

impl Material {
    pub fn new(device: &DeviceLoader) -> Result<Self> {
        let ubo_layout_bindings = [vk::DescriptorSetLayoutBindingBuilder::new()
            .binding(0)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .stage_flags(vk::ShaderStageFlags::VERTEX)
            .descriptor_count(1)];

        let descriptor_set_layout_ci =
            vk::DescriptorSetLayoutCreateInfoBuilder::new().bindings(&ubo_layout_bindings);

        let descriptor_set_layout =
            unsafe { device.create_descriptor_set_layout(&descriptor_set_layout_ci, None, None) }
                .result()?;
        todo!("Shaders")
    }
}

impl Pipeline {
    pub fn new(device: &DeviceLoader, material: &Material, extent: vk::Extent2D) -> Result<vk::PipelineLayout> {
        // Pipeline layouts 
        let attribute_descriptions = Vertex::get_attribute_descriptions();
        let binding_descriptions = [Vertex::binding_description()];

        let vertex_input = vk::PipelineVertexInputStateCreateInfoBuilder::new()
            .vertex_attribute_descriptions(&attribute_descriptions[..])
            .vertex_binding_descriptions(&binding_descriptions);

        let input_assembly = vk::PipelineInputAssemblyStateCreateInfoBuilder::new()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
            .primitive_restart_enable(false);

        let viewports = [vk::ViewportBuilder::new()
            .x(0.0)
            .y(0.0)
            .width(extent.width as f32)
            .height(extent.height as f32)
            .min_depth(0.0)
            .max_depth(1.0)];
        let scissors = [vk::Rect2DBuilder::new()
            .offset(vk::Offset2D { x: 0, y: 0 })
            .extent(extent)];
        let viewport_state = vk::PipelineViewportStateCreateInfoBuilder::new()
            .viewports(&viewports)
            .scissors(&scissors);

        let rasterizer = vk::PipelineRasterizationStateCreateInfoBuilder::new()
            .depth_clamp_enable(false)
            .rasterizer_discard_enable(false)
            .polygon_mode(vk::PolygonMode::FILL)
            .line_width(1.0)
            .cull_mode(vk::CullModeFlags::BACK)
            .front_face(vk::FrontFace::CLOCKWISE)
            .depth_clamp_enable(false);

        let multisampling = vk::PipelineMultisampleStateCreateInfoBuilder::new()
            .sample_shading_enable(false)
            .rasterization_samples(vk::SampleCountFlagBits::_1);

        let color_blend_attachments = [vk::PipelineColorBlendAttachmentStateBuilder::new()
            .color_write_mask(
                vk::ColorComponentFlags::R
                | vk::ColorComponentFlags::G
                | vk::ColorComponentFlags::B
                | vk::ColorComponentFlags::A,
            )
            .blend_enable(false)];
        let color_blending = vk::PipelineColorBlendStateCreateInfoBuilder::new()
            .logic_op_enable(false)
            .attachments(&color_blend_attachments);

        let descriptor_set_layouts = [material.descriptor_set_layout];

        let create_info =
            vk::PipelineLayoutCreateInfoBuilder::new().set_layouts(&descriptor_set_layouts);

        let pipeline_layout =
            unsafe { device.create_pipeline_layout(&create_info, None, None) }.result()?;

        Ok(pipeline_layout)
    }
}
