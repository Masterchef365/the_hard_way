use crate::vertex::Vertex;
use anyhow::Result;
use erupt::{utils, vk1_0 as vk, DeviceLoader};
use std::ffi::CString;

/// Represents a backing pipeline that can render an object
/// with the material from which it was created.
pub struct Pipeline {
    pub pipeline: vk::Pipeline,
    pub pipeline_layout: vk::PipelineLayout,
    freed: bool,
}

/// Represents a set of drawing parameters to be turned into a pipeline
pub struct Material {
    draw_type: DrawType,
    vertex: vk::ShaderModule,
    fragment: vk::ShaderModule,
    freed: bool,
}

pub enum DrawType {
    Triangles,
    Lines,
    Points,
}

impl Material {
    pub fn new(
        device: &DeviceLoader,
        vertex_src: &[u8],
        fragment_src: &[u8],
        draw_type: DrawType,
    ) -> Result<Self> {
        let vert_decoded = utils::decode_spv(vertex_src)?;
        let create_info = vk::ShaderModuleCreateInfoBuilder::new().code(&vert_decoded);
        let vertex = unsafe { device.create_shader_module(&create_info, None, None) }.result()?;

        let frag_decoded = utils::decode_spv(fragment_src)?;
        let create_info = vk::ShaderModuleCreateInfoBuilder::new().code(&frag_decoded);
        let fragment = unsafe { device.create_shader_module(&create_info, None, None) }.result()?;

        Ok(Self {
            draw_type,
            vertex,
            fragment,
            freed: false,
        })
    }

    pub fn free(&mut self, device: &DeviceLoader) {
        unsafe {
            device.destroy_shader_module(Some(self.fragment), None);
            device.destroy_shader_module(Some(self.vertex), None);
        }
        self.freed = true;
    }
}

impl Pipeline {
    pub fn new(
        device: &DeviceLoader,
        material: &Material,
        render_pass: vk::RenderPass,
        descriptor_set_layout: vk::DescriptorSetLayout,
        extent: vk::Extent2D,
    ) -> Result<Self> {
        let attribute_descriptions = Vertex::get_attribute_descriptions();
        let binding_descriptions = [Vertex::binding_description()];

        let vertex_input = vk::PipelineVertexInputStateCreateInfoBuilder::new()
            .vertex_attribute_descriptions(&attribute_descriptions[..])
            .vertex_binding_descriptions(&binding_descriptions);

        let draw_type = match material.draw_type {
            DrawType::Triangles => vk::PrimitiveTopology::TRIANGLE_LIST,
            DrawType::Points => vk::PrimitiveTopology::POINT_LIST,
            DrawType::Lines => vk::PrimitiveTopology::LINE_LIST,
        };

        let input_assembly = vk::PipelineInputAssemblyStateCreateInfoBuilder::new()
            .topology(draw_type)
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
            .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
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

        let entry_point = CString::new("main")?;

        let shader_stages = [
            vk::PipelineShaderStageCreateInfoBuilder::new()
                .stage(vk::ShaderStageFlagBits::VERTEX)
                .module(material.vertex)
                .name(&entry_point),
            vk::PipelineShaderStageCreateInfoBuilder::new()
                .stage(vk::ShaderStageFlagBits::FRAGMENT)
                .module(material.fragment)
                .name(&entry_point),
        ];

        let descriptor_set_layouts = [descriptor_set_layout];

        let create_info = vk::PipelineLayoutCreateInfoBuilder::new()
            .set_layouts(&descriptor_set_layouts);

        let pipeline_layout =
            unsafe { device.create_pipeline_layout(&create_info, None, None) }.result()?;

        let create_info = vk::GraphicsPipelineCreateInfoBuilder::new()
            .stages(&shader_stages)
            .vertex_input_state(&vertex_input)
            .input_assembly_state(&input_assembly)
            .viewport_state(&viewport_state)
            .rasterization_state(&rasterizer)
            .multisample_state(&multisampling)
            .color_blend_state(&color_blending)
            .layout(pipeline_layout)
            .render_pass(render_pass)
            .subpass(0);

        let pipeline =
            unsafe { device.create_graphics_pipelines(None, &[create_info], None) }.result()?[0];

        Ok(Self {
            pipeline,
            pipeline_layout,
            freed: false,
        })
    }

    pub fn free(&mut self, device: &DeviceLoader) {
        unsafe {
            device.destroy_pipeline(Some(self.pipeline), None);
            device.destroy_pipeline_layout(Some(self.pipeline_layout), None);
        }
        self.freed = true;
    }
}

impl Drop for Pipeline {
    fn drop(&mut self) {
        if !self.freed {
            panic!("Pipeline was dropped before it was freed!");
        }
    }
}

impl Drop for Material {
    fn drop(&mut self) {
        if !self.freed {
            panic!("Material was dropped before it was freed!");
        }
    }
}
