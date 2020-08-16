use crate::frame_sync::FrameSync;
use crate::hardware_query::HardwareSelection;
use crate::Engine;
use anyhow::Result;
use erupt::{
    cstr,
    extensions::{ext_debug_utils, khr_swapchain},
    utils::{allocator, surface},
    vk1_0 as vk, DeviceLoader, EntryLoader, InstanceLoader,
};
use std::{
    ffi::CString,
    os::raw::c_char,
};
use winit::window::Window;
use crate::allocated_buffer::AllocatedBuffer;

const FRAMES_IN_FLIGHT: usize = 2;

impl Engine {
    pub fn new(window: &Window, app_name: &str) -> Result<Self> {
        // Entry
        let entry = EntryLoader::new()?;

        // Instance
        let application_name = CString::new(app_name)?;
        let engine_name = CString::new("Prototype engine")?;
        let app_info = vk::ApplicationInfoBuilder::new()
            .application_name(&application_name)
            .application_version(vk::make_version(1, 0, 0))
            .engine_name(&engine_name)
            .engine_version(vk::make_version(1, 0, 0))
            .api_version(vk::make_version(1, 0, 0));

        let mut instance_extensions = surface::enumerate_required_extensions(window).result()?;
        if cfg!(debug_assertions) {
            instance_extensions.push(ext_debug_utils::EXT_DEBUG_UTILS_EXTENSION_NAME);
        }

        const LAYER_KHRONOS_VALIDATION: *const c_char = cstr!("VK_LAYER_KHRONOS_validation");

        let mut instance_layers = Vec::new();
        if cfg!(debug_assertions) {
            instance_layers.push(LAYER_KHRONOS_VALIDATION);
        }

        let device_extensions = [khr_swapchain::KHR_SWAPCHAIN_EXTENSION_NAME];

        let mut device_layers = Vec::new();
        if cfg!(debug_assertions) {
            device_layers.push(LAYER_KHRONOS_VALIDATION);
        }

        let create_info = vk::InstanceCreateInfoBuilder::new()
            .application_info(&app_info)
            .enabled_extension_names(&instance_extensions)
            .enabled_layer_names(&instance_layers);

        let mut instance = InstanceLoader::new(&entry, &create_info, None)?;

        // Surface
        let surface = unsafe { surface::create_surface(&mut instance, window, None) }.result()?;

        // Hardware selection
        let hardware = HardwareSelection::query(&instance, surface, &device_extensions)?;

        // Create logical device and queues
        let create_info = [vk::DeviceQueueCreateInfoBuilder::new()
            .queue_family_index(hardware.queue_family)
            .queue_priorities(&[1.0])];

        let physical_device_features = vk::PhysicalDeviceFeaturesBuilder::new();
        let create_info = vk::DeviceCreateInfoBuilder::new()
            .queue_create_infos(&create_info)
            .enabled_features(&physical_device_features)
            .enabled_extension_names(&device_extensions)
            .enabled_layer_names(&device_layers);

        let device = DeviceLoader::new(&instance, hardware.physical_device, &create_info, None)?;
        let queue = unsafe { device.get_device_queue(hardware.queue_family, 0, None) };

        // Command pool
        let create_info =
            vk::CommandPoolCreateInfoBuilder::new()
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
            .queue_family_index(hardware.queue_family);
        let command_pool =
            unsafe { device.create_command_pool(&create_info, None, None) }.result()?;

        // Allocate command buffers
        let allocate_info = vk::CommandBufferAllocateInfoBuilder::new()
            .command_pool(command_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(FRAMES_IN_FLIGHT as u32);

        let command_buffers =
            unsafe { device.allocate_command_buffers(&allocate_info) }.result()?;

        // Device memory allocator
        let mut allocator = allocator::Allocator::new(
            &instance,
            hardware.physical_device,
            allocator::AllocatorCreateInfo::default(),
        )
        .result()?;

        // Create descriptor layout
        let bindings = [vk::DescriptorSetLayoutBindingBuilder::new()
            .binding(0)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::VERTEX)];

        let descriptor_set_layout_ci =
            vk::DescriptorSetLayoutCreateInfoBuilder::new().bindings(&bindings);

        let descriptor_set_layout =
            unsafe { device.create_descriptor_set_layout(&descriptor_set_layout_ci, None, None) }
                .result()?;

        // Create descriptor pool
        let pool_sizes = [
            vk::DescriptorPoolSizeBuilder::new()
                ._type(vk::DescriptorType::UNIFORM_BUFFER)
                .descriptor_count(FRAMES_IN_FLIGHT as u32)
        ];
        let create_info = vk::DescriptorPoolCreateInfoBuilder::new()
            .pool_sizes(&pool_sizes)
            .max_sets(FRAMES_IN_FLIGHT as u32);
        let descriptor_pool = unsafe {
            device.create_descriptor_pool(&create_info, None, None)
        }.result()?;
        
        // Create descriptor sets
        let layouts = vec![descriptor_set_layout; FRAMES_IN_FLIGHT];
        let create_info = vk::DescriptorSetAllocateInfoBuilder::new()
            .descriptor_pool(descriptor_pool)
            .set_layouts(&layouts);

        let descriptor_sets = unsafe {
            device.allocate_descriptor_sets(&create_info)
        }.result()?;

        // Camera's UBOs
        let create_info = vk::BufferCreateInfoBuilder::new()
            .usage(vk::BufferUsageFlags::UNIFORM_BUFFER)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);
        let camera_ubos = (0..FRAMES_IN_FLIGHT).map(|_| 
            AllocatedBuffer::new(1, create_info.clone(), &mut allocator, &device)).collect::<Result<Vec<_>>>()?;

        // Bind buffers to descriptors
        for (alloc, descriptor) in camera_ubos.iter().zip(descriptor_sets.iter()) {
            let buffer_infos = [vk::DescriptorBufferInfoBuilder::new()
                .buffer(alloc.buffer)
                .offset(0)
                .range(std::mem::size_of::<[[f32; 4]; 4]>() as u64)];

            let writes = [vk::WriteDescriptorSetBuilder::new()
                .buffer_info(&buffer_infos)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .dst_set(*descriptor)
                .dst_binding(0)
                .dst_array_element(0)];

            unsafe {
                device.update_descriptor_sets(&writes, &[]);
            }
        }

        // Frame synchronization
        let frame_sync = FrameSync::new(&device, FRAMES_IN_FLIGHT)?;

        Ok(Self {
            _entry: entry,
            camera_ubos,
            descriptor_set_layout,
            descriptor_pool,
            descriptor_sets,
            instance,
            surface,
            hardware,
            device,
            queue,
            command_pool,
            frame_sync,
            allocator,
            command_buffers,
            swapchain: None,
            materials: Default::default(),
            objects: Default::default(),
            next_material_id: 0,
            next_object_id: 0,
        })
    }
}
