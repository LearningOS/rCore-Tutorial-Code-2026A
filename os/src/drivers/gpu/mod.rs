//! GPU driver for the chapter 9 API exercise.
//!
//! Initialize the device and its framebuffer, expose that same memory, and
//! submit display updates. Keep the types, device address, and cursor helper.
//! The driver owns the DMA resources; the framebuffer slice does not own them.

#![allow(dead_code, unused_imports, unused_variables)]

use crate::drivers::bus::virtio::VirtioHal;
use crate::sync::UPIntrFreeCell;
use alloc::{sync::Arc, vec::Vec};
use core::any::Any;
use embedded_graphics::pixelcolor::Rgb888;
use tinybmp::Bmp;
use virtio_drivers::{VirtIOGpu, VirtIOHeader};
const VIRTIO7: usize = 0x10007000;
pub trait GpuDevice: Send + Sync + Any {
    fn get_framebuffer(&self) -> &mut [u8];
    fn flush(&self);
}

lazy_static::lazy_static!(
    pub static ref GPU_DEVICE: Arc<dyn GpuDevice> = Arc::new(VirtIOGpuWrapper::new());
);

pub struct VirtIOGpuWrapper {
    gpu: UPIntrFreeCell<VirtIOGpu<'static, VirtioHal>>,
    fb: &'static [u8],
}
static BMP_DATA: &[u8] = include_bytes!("../../assert/mouse.bmp");

/// Provided cursor setup, preserving the reference implementation's appearance.
fn setup_cursor(gpu: &mut VirtIOGpu<'static, VirtioHal>) {
    let bmp = Bmp::<Rgb888>::from_slice(BMP_DATA).unwrap();
    let raw = bmp.as_raw();
    let mut b = Vec::new();
    for i in raw.image_data().chunks(3) {
        let mut v = i.to_vec();
        b.append(&mut v);
        if i == [255, 255, 255] {
            b.push(0x0)
        } else {
            b.push(0xff)
        }
    }
    gpu.setup_cursor(b.as_slice(), 50, 50, 50, 50).unwrap();
}

impl VirtIOGpuWrapper {
    /// Initialize VirtIOGpu at VIRTIO7 and set up its framebuffer.
    ///
    /// Save the device-owned framebuffer's address and length, call the provided
    /// setup_cursor(), and retain the device in gpu for the buffer's lifetime.
    /// Do not allocate a replacement pixel buffer or take ownership of DMA pages.
    pub fn new() -> Self {
        let mut gpu = unsafe {
            VirtIOGpu::<VirtioHal>::new(&mut *(VIRTIO7 as *mut VirtIOHeader)).unwrap()
        };
        let framebuffer = gpu.setup_framebuffer().unwrap();
        let ptr = framebuffer.as_mut_ptr();
        let len = framebuffer.len();
        // The DMA allocation stays valid while the stored GPU owns it.
        let fb = unsafe { core::slice::from_raw_parts(ptr, len) };
        setup_cursor(&mut gpu);
        Self {
            gpu: unsafe { UPIntrFreeCell::new(gpu) },
            fb,
        }
    }
}

impl GpuDevice for VirtIOGpuWrapper {
    /// Flush the existing GPU framebuffer, retaining the device's protected access.
    fn flush(&self) {
        self.gpu.exclusive_access().flush().unwrap();
    }
    /// Return a mutable view of the stored framebuffer with the same address/length.
    /// Follow the existing single-core use; do not allocate, clear, or free it here.
    fn get_framebuffer(&self) -> &mut [u8] {
        unsafe { core::slice::from_raw_parts_mut(self.fb.as_ptr() as *mut u8, self.fb.len()) }
    }
}
