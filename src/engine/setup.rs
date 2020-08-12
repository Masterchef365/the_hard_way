use crate::frame_sync::FrameSync;
use crate::hardware_query::HardwareSelection;
use crate::Engine;
use anyhow::Result;
use erupt::{
    cstr,
    extensions::{ext_debug_utils, khr_surface, khr_swapchain},
    utils::{self, allocator, surface},
    vk1_0 as vk, DeviceLoader, EntryLoader, InstanceLoader,
};
use nalgebra::{Matrix4, Point2, Point3};
use std::path::Path;
use std::{
    ffi::{CStr, CString},
    os::raw::c_char,
};
use winit::window::Window;
use std::collections::HashMap;

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
        let queue_create_info = [vk::DeviceQueueCreateInfoBuilder::new()
            .queue_family_index(hardware.queue_family)
            .queue_priorities(&[1.0])];

        let physical_device_features = vk::PhysicalDeviceFeaturesBuilder::new();
        let create_info = vk::DeviceCreateInfoBuilder::new()
            .queue_create_infos(&queue_create_info)
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

        // Frame synchronization
        let frame_sync = FrameSync::new(&device, 2)?;

        // Device memory allocator
        let allocator = allocator::Allocator::new(
            &instance,
            hardware.physical_device,
            allocator::AllocatorCreateInfo::default(),
        )
        .result()?;

        Ok(Self {
            _entry: entry,
            instance,
            surface,
            hardware,
            device,
            queue,
            command_pool,
            frame_sync,
            allocator,
            swapchain: None,
            materials: Default::default(),
            objects: Default::default(),
            next_material_id: 0,
            next_object_id: 0,
        })
    }
}
