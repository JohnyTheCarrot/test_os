use crate::color::Color;
use bootloader_api::info::{FrameBufferInfo, PixelFormat};

pub struct FrameBufferWrapper<'a> {
    pub(crate) buffer: &'a mut [u8],
    pub(crate) info: FrameBufferInfo,
}

impl FrameBufferWrapper<'_> {
    pub fn write_format_agnostic_pixel(&mut self, x: usize, y: usize, value: u8) {
        let location = (y * self.info.stride + x) * self.info.bytes_per_pixel;

        self.buffer[location] = value;
        self.buffer[location + 1] = value;
        self.buffer[location + 2] = value;
    }

    pub fn write_pixel(&mut self, x: usize, y: usize, color: Color) {
        let location = (y * self.info.stride + x) * self.info.bytes_per_pixel;
        if self.info.pixel_format != PixelFormat::Rgb {
            panic!(
                "pixel format {:?} not supported in framebuffer",
                self.info.pixel_format
            );
        }

        self.buffer[location] = color.r;
        self.buffer[location + 1] = color.g;
        self.buffer[location + 2] = color.b;
    }
}
