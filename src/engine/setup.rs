use crate::allocated_buffer::AllocatedBuffer;
use crate::frame_sync::FrameSync;
use crate::hardware_query::HardwareSelection;
use crate::Engine;
use anyhow::{bail, Result};
use erupt::{utils::allocator, vk1_0 as vk, vk1_1, DeviceLoader, EntryLoader, InstanceLoader};
use openxr as xr;
use std::ffi::CString;

const FRAMES_IN_FLIGHT: usize = 2;

impl Engine {
    pub fn new(
        xr_instance: &xr::Instance,
        system: xr::SystemId,
        app_name: &str,
    ) -> Result<(xr::Session<xr::Vulkan>, Self)> {
        // Vulkan entry
        let vk_entry = EntryLoader::new()?;

        // Vulkan Instance
        let application_name = CString::new(app_name)?;
        let engine_name = CString::new("Prototype engine")?;
        let app_info = vk::ApplicationInfoBuilder::new()
            .application_name(&application_name)
            .application_version(vk::make_version(1, 0, 0))
            .engine_name(&engine_name)
            .engine_version(vk::make_version(1, 0, 0))
            .api_version(vk::make_version(1, 1, 0));

        // Check to see if OpenXR and Vulkan are compatible
        let vk_version = unsafe { vk_entry.enumerate_instance_version(None).result()? };

        let vk_version = xr::Version::new(
            vk::version_major(vk_version) as u16,
            vk::version_major(vk_version) as u16,
            0,
        );

        let reqs = xr_instance
            .graphics_requirements::<xr::Vulkan>(system)
            .unwrap();
        if reqs.min_api_version_supported > vk_version {
            bail!(
                "OpenXR runtime requires Vulkan version > {}",
                reqs.min_api_version_supported
            );
        }

        // Vulkan vk_instance extensions required by OpenXR
        let vk_instance_exts = xr_instance
            .vulkan_instance_extensions(system)
            .unwrap()
            .split(' ')
            .map(|x| std::ffi::CString::new(x).unwrap())
            .collect::<Vec<_>>();

        let vk_instance_ext_ptrs = vk_instance_exts
            .iter()
            .map(|x| x.as_ptr())
            .collect::<Vec<_>>();

        // Vulkan vk_device extensions required by OpenXR
        let vk_device_exts = xr_instance
            .vulkan_device_extensions(system)
            .unwrap()
            .split(' ')
            .map(|x| CString::new(x).unwrap())
            .collect::<Vec<_>>();

        let vk_device_ext_ptrs = vk_device_exts
            .iter()
            .map(|x| x.as_ptr())
            .collect::<Vec<_>>();

        // Create Vulkan vk_instance
        let create_info = vk::InstanceCreateInfoBuilder::new()
            .application_info(&app_info)
            .enabled_extension_names(&vk_instance_ext_ptrs);

        let vk_instance = InstanceLoader::new(&vk_entry, &create_info, None)?;

        // Obtain physical vk_device, queue_family_index, and vk_device from OpenXR
        let vk_physical_device = vk::PhysicalDevice(
            xr_instance
                .vulkan_graphics_device(system, vk_instance.handle.0 as _)
                .unwrap() as _,
        );

        let queue_family_index = unsafe {
            vk_instance
                .get_physical_device_queue_family_properties(vk_physical_device, None)
                .into_iter()
                .enumerate()
                .filter_map(|(queue_family_index, info)| {
                    if info.queue_flags.contains(vk::QueueFlags::GRAPHICS) {
                        Some(queue_family_index as u32)
                    } else {
                        None
                    }
                })
                .next()
                .expect("Vulkan vk_device has no graphics queue")
        };

        let mut create_info = vk::DeviceCreateInfoBuilder::new()
            .queue_create_infos(&[vk::DeviceQueueCreateInfoBuilder::new()
                .queue_family_index(queue_family_index)
                .queue_priorities(&[1.0])])
            .enabled_extension_names(&vk_device_ext_ptrs)
            .build();

        let mut phys_device_features = vk1_1::PhysicalDeviceMultiviewFeatures {
            multiview: vk::TRUE,
            ..Default::default()
        };

        create_info.p_next = &mut phys_device_features as *mut _ as _;

        let vk_device = DeviceLoader::new(&vk_instance, vk_physical_device, &create_info, None)?;
        let queue = unsafe { vk_device.get_device_queue(queue_family_index, 0, None) };

        // Command pool
        let create_info = vk::CommandPoolCreateInfoBuilder::new()
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
            .queue_family_index(queue_family_index);
        let command_pool =
            unsafe { vk_device.create_command_pool(&create_info, None, None) }.result()?;

        // Allocate command buffers
        let allocate_info = vk::CommandBufferAllocateInfoBuilder::new()
            .command_pool(command_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(FRAMES_IN_FLIGHT as u32);

        let command_buffers =
            unsafe { vk_device.allocate_command_buffers(&allocate_info) }.result()?;

        // Device memory allocator
        let mut allocator = allocator::Allocator::new(
            &vk_instance,
            vk_physical_device,
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

        let descriptor_set_layout = unsafe {
            vk_device.create_descriptor_set_layout(&descriptor_set_layout_ci, None, None)
        }
        .result()?;

        // Create descriptor pool
        let pool_sizes = [vk::DescriptorPoolSizeBuilder::new()
            ._type(vk::DescriptorType::UNIFORM_BUFFER)
            .descriptor_count(FRAMES_IN_FLIGHT as u32)];
        let create_info = vk::DescriptorPoolCreateInfoBuilder::new()
            .pool_sizes(&pool_sizes)
            .max_sets(FRAMES_IN_FLIGHT as u32);
        let descriptor_pool =
            unsafe { vk_device.create_descriptor_pool(&create_info, None, None) }.result()?;

        // Create descriptor sets
        let layouts = vec![descriptor_set_layout; FRAMES_IN_FLIGHT];
        let create_info = vk::DescriptorSetAllocateInfoBuilder::new()
            .descriptor_pool(descriptor_pool)
            .set_layouts(&layouts);

        let descriptor_sets =
            unsafe { vk_device.allocate_descriptor_sets(&create_info) }.result()?;

        // Camera's UBOs
        let create_info = vk::BufferCreateInfoBuilder::new()
            .usage(vk::BufferUsageFlags::UNIFORM_BUFFER)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);
        let camera_ubos = (0..FRAMES_IN_FLIGHT)
            .map(|_| AllocatedBuffer::new(1, create_info.clone(), &mut allocator, &vk_device))
            .collect::<Result<Vec<_>>>()?;

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
                vk_device.update_descriptor_sets(&writes, &[]);
            }
        }

        // Frame synchronization
        let frame_sync = FrameSync::new(&vk_device, FRAMES_IN_FLIGHT)?;

        let hardware = HardwareSelection {
            physical_device: vk_physical_device,
            queue_family: queue_family_index,
        };

        let (session, frame_wait, frame_stream) = unsafe {
            xr_instance.create_session::<xr::Vulkan>(
                system,
                &xr::vulkan::SessionCreateInfo {
                    instance: vk_instance.handle.0 as _,
                    physical_device: vk_physical_device.0 as _,
                    device: vk_device.handle.0 as _,
                    queue_family_index,
                    queue_index: 0,
                },
            )
        }?;

        let stage = session
            .create_reference_space(xr::ReferenceSpaceType::STAGE, xr::Posef::IDENTITY)
            .unwrap();

        Ok((
            session,
            Self {
                _entry: vk_entry,
                camera_ubos,
                descriptor_set_layout,
                descriptor_pool,
                descriptor_sets,
                vk_instance,
                hardware,
                vk_device,
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
                stage: Some(stage),
                frame_wait: Some(frame_wait),
                frame_stream: Some(frame_stream),
            },
        ))
    }
}
