use anyhow::Result;
use erupt::{
    utils::
        allocator::{self, Allocator},
    vk1_0 as vk, DeviceLoader,
};
use std::marker::PhantomData;

pub struct AllocatedBuffer<T> {
    pub buffer: vk::Buffer,
    pub allocation: Option<allocator::Allocation<vk::Buffer>>,
    create_info: vk::BufferCreateInfoBuilder<'static>,
    dynamic: bool,
    _phantom: PhantomData<T>,
    freed: bool,
}

impl<T: Sized + bytemuck::Pod> AllocatedBuffer<T> {
    pub fn new(
        count: usize,
        create_info: vk::BufferCreateInfoBuilder<'static>,
        allocator: &mut Allocator,
        device: &DeviceLoader,
    ) -> Result<Self> {
        anyhow::ensure!(count > 0, "Must allocate at least one object");
        let size = std::mem::size_of::<T>() * count;
        let mut create_info = create_info.size(size as u64);
        create_info.usage |= vk::BufferUsageFlags::TRANSFER_SRC;
        let buffer = unsafe { device.create_buffer(&create_info, None, None) }.result()?;
        let allocation = allocator
            .allocate(device, buffer, allocator::MemoryTypeFinder::dynamic())
            .result()?;
        Ok(Self {
            buffer,
            allocation: Some(allocation),
            dynamic: true,
            freed: false,
            create_info,
            _phantom: PhantomData::default(),
        })
    }

    pub fn map(&self, device: &DeviceLoader, data: &[T]) -> Result<()> {
        if !self.dynamic {
            anyhow::bail!("Cannot write to gpu-only memory");
        }
        let mut map = self
            .allocation
            .as_ref()
            .expect("Use-after-free")
            .map(device, ..)
            .result()?;
        map.import(bytemuck::cast_slice(data));
        map.unmap(&device).result()?;
        Ok(())
    }

    pub fn gpu_only(mut self, device: &DeviceLoader, allocator: &mut Allocator, command_pool: vk::CommandPool, queue: vk::Queue) -> Result<Self> {
        self.create_info.usage |= vk::BufferUsageFlags::TRANSFER_DST;
        let buffer = unsafe { device.create_buffer(&self.create_info, None, None) }.result()?;
        let allocation = allocator
            .allocate(device, buffer, allocator::MemoryTypeFinder::gpu_only())
            .result()?;

        let create_info = vk::CommandBufferAllocateInfoBuilder::new()
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_pool(command_pool)
            .command_buffer_count(1);

        let command_buffer = unsafe { 
            device.allocate_command_buffers(&create_info)
        }.result()?[0];

        let begin_info = vk::CommandBufferBeginInfoBuilder::new()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

        unsafe {
            device.begin_command_buffer(command_buffer, &begin_info);
            let copy_region = vk::BufferCopyBuilder::new()
                .src_offset(0)
                .dst_offset(0)
                .size(self.create_info.size);
            device.cmd_copy_buffer(command_buffer, self.buffer, buffer, &[copy_region]);
            device.end_command_buffer(command_buffer);
            let command_buffers = [command_buffer];
            let submit_info = vk::SubmitInfoBuilder::new()
                .command_buffers(&command_buffers);
            device.queue_submit(queue, &[submit_info], None).result()?;
            device.queue_wait_idle(queue).result()?;
            device.free_command_buffers(command_pool, &[command_buffer]);
        }

        self.free(device, allocator)?;

        Ok(Self {
            buffer,
            allocation: Some(allocation),
            create_info: self.create_info,
            _phantom: self._phantom,
            dynamic: false,
            freed: false,
        })
    }

    pub fn free(&mut self, device: &DeviceLoader, allocator: &mut Allocator) -> Result<()> {
        unsafe {
            device.device_wait_idle().result()?;
        }
        allocator.free(
            &device,
            self.allocation.take().expect("Already deallocated"),
        );
        self.freed = true;
        Ok(())
    }
}

impl<T> Drop for AllocatedBuffer<T> {
    fn drop(&mut self) {
        if !self.freed {
            panic!(
                "AllocatedBuffer<{}> was dropped before it was freed!",
                std::any::type_name::<T>()
            );
        }
    }
}
