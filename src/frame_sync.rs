use anyhow::Result;
use erupt::{vk1_0 as vk, DeviceLoader};

/// Manages fences and semaphores for every given frame
pub struct FrameSync {
    frames: Vec<Frame>,
    frame_idx: usize,
    freed: bool,
}

struct Frame {
    in_flight_fence: vk::Fence,
    image_available: vk::Semaphore,
    render_finished: vk::Semaphore,
}

impl FrameSync {
    pub fn new(device: &DeviceLoader, frames_in_flight: usize) -> Result<Self> {
        let frames = (0..frames_in_flight)
            .map(|_| Frame::new(device))
            .collect::<Result<_>>()?;

        Ok(Self {
            frames,
            freed: false,
            frame_idx: 0,
        })
    }

    pub fn fence(&self) -> &vk::Fence {
        &self.frames[self.frame_idx].in_flight_fence
    }

    pub fn image_available(&self) -> &vk::Semaphore {
        &self.frames[self.frame_idx].image_available
    }

    pub fn render_finished(&self) -> &vk::Semaphore {
        &self.frames[self.frame_idx].render_finished
    }

    pub fn next_frame(&mut self) {
        self.frame_idx = (self.frame_idx + 1) % self.frames.len();
    }

    pub fn free(&mut self, device: &DeviceLoader) {
        for frame in &mut self.frames {
            frame.free(device);
        }
        self.freed = true;
    }
}

impl Drop for FrameSync {
    fn drop(&mut self) {
        if !self.freed {
            panic!("FrameSync dropped before its free() method was called!");
        }
    }
}

impl Frame {
    pub fn new(device: &DeviceLoader) -> Result<Self> {
        unsafe {
            let create_info = vk::SemaphoreCreateInfoBuilder::new();
            let image_available = device.create_semaphore(&create_info, None, None).result()?;
            let render_finished = device.create_semaphore(&create_info, None, None).result()?;

            let create_info =
                vk::FenceCreateInfoBuilder::new().flags(vk::FenceCreateFlags::SIGNALED);
            let in_flight_fence = device.create_fence(&create_info, None, None).result()?;
            Ok(Self {
                in_flight_fence,
                image_available,
                render_finished,
            })
        }
    }

    pub fn free(&mut self, device: &DeviceLoader) {
        unsafe {
            device.destroy_semaphore(Some(self.image_available), None);
            device.destroy_semaphore(Some(self.render_finished), None);
            device.destroy_fence(Some(self.in_flight_fence), None);
        }
    }
}
