use anyhow::Result;
use erupt::{vk1_0 as vk, DeviceLoader};

pub struct FrameSync {
    pub in_flight_fences: Vec<vk::Fence>,
    pub image_available_semaphores: Vec<vk::Semaphore>,
    pub render_finished_semaphores: Vec<vk::Semaphore>,
    pub frames_in_flight: usize,
    pub frame_idx: usize,
    pub freed: bool,
}

impl FrameSync {
    pub fn new(device: &DeviceLoader, frames_in_flight: usize) -> Result<Self> {
        unsafe {
            let create_info = vk::SemaphoreCreateInfoBuilder::new();
            let image_available_semaphores: Vec<_> = (0..frames_in_flight)
                .map(|_|  device.create_semaphore(&create_info, None, None).result())
                .collect::<Result<_, _>>()?;

            let render_finished_semaphores: Vec<_> = (0..frames_in_flight)
                .map(|_| device.create_semaphore(&create_info, None, None).result())
                .collect::<Result<_, _>>()?;

            let create_info = vk::FenceCreateInfoBuilder::new().flags(vk::FenceCreateFlags::SIGNALED);
            let in_flight_fences: Vec<_> = (0..frames_in_flight)
                .map(|_| device.create_fence(&create_info, None, None).result())
                .collect::<Result<_, _>>()?;

            Ok(Self {
                image_available_semaphores,
                render_finished_semaphores,
                in_flight_fences,
                frames_in_flight,
                freed: false,
                frame_idx: 0
            })
        }
    }

    pub fn fence(&self) -> &vk::Fence {
        &self.in_flight_fences[self.frame_idx]
    }

    pub fn image_available(&self) -> &vk::Semaphore {
        &self.image_available_semaphores[self.frame_idx]
    }

    pub fn render_finished(&self) -> &vk::Semaphore {
        &self.render_finished_semaphores[self.frame_idx]
    }

    pub fn next_frame(&mut self) {
        self.frame_idx = (self.frame_idx + 1) % self.frames_in_flight;
    }

    pub fn free(&mut self, device: &DeviceLoader) {
        unsafe {
            for &semaphore in self
                .image_available_semaphores
                .iter()
                .chain(self.render_finished_semaphores.iter())
            {
                device.destroy_semaphore(Some(semaphore), None);
            }

            for &fence in &self.in_flight_fences {
                device.destroy_fence(Some(fence), None);
            }
        }
    }
}

impl Drop for FrameSync {
    fn drop(&mut self) {
        if !self.freed {
            panic!("FrameSync dropped before its free() method was called!");
        }
    }
}
