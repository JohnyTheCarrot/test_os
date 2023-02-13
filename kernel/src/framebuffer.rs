use crate::color::Color;
use alloc::vec::Vec;
use bootloader_api::info::{FrameBufferInfo, PixelFormat};

pub struct FrameBufferWrapper<'a> {
    pub(crate) buffer: &'a mut [u8],
    pub(crate) info: FrameBufferInfo,
}

impl FrameBufferWrapper<'_> {
    #[inline]
    pub fn write_pixel_as_bgr(&mut self, location: usize, color: Color) {
        self.buffer[location] = color.b;
        self.buffer[location + 1] = color.g;
        self.buffer[location + 2] = color.r;
    }

    #[inline]
    pub fn write_pixel_as_rgb(&mut self, location: usize, color: Color) {
        self.buffer[location] = color.r;
        self.buffer[location + 1] = color.g;
        self.buffer[location + 2] = color.b;
    }

    pub fn write_pixel(&mut self, x: usize, y: usize, color: Color) {
        let location = (y * self.info.stride + x) * self.info.bytes_per_pixel;

        if self.info.pixel_format == PixelFormat::Rgb {
            self.write_pixel_as_rgb(location, color);
        } else if self.info.pixel_format == PixelFormat::Bgr {
            self.write_pixel_as_bgr(location, color);
        }
    }

    pub fn fill_rect(&mut self, x: usize, y: usize, width: usize, height: usize, color: Color) {
        let mut location = (y * self.info.stride + x) * self.info.bytes_per_pixel;
        let bytes_until_next_line = (self.info.stride - width) * self.info.bytes_per_pixel;

        if self.info.pixel_format == PixelFormat::Bgr {
            for _ in 0..height {
                for _ in 0..width {
                    self.buffer[location] = color.b;
                    self.buffer[location + 1] = color.g;
                    self.buffer[location + 2] = color.r;

                    location += self.info.bytes_per_pixel;
                }

                location += bytes_until_next_line;
            }
        }

        if self.info.pixel_format == PixelFormat::Rgb {
            for _ in 0..height {
                for _ in 0..width {
                    self.buffer[location] = color.r;
                    self.buffer[location + 1] = color.g;
                    self.buffer[location + 2] = color.b;

                    location += self.info.bytes_per_pixel;
                }

                location += bytes_until_next_line;
            }
        }
    }

    pub fn fill_screen(&mut self, color: Color) {
        let color_slice = if self.info.pixel_format == PixelFormat::Bgr {
            [color.b, color.g, color.r, 0]
        } else {
            [color.r, color.g, color.b, 0]
        };

        for s in self.buffer.chunks_exact_mut(self.info.bytes_per_pixel) {
            s.copy_from_slice(&color_slice);
        }
    }

    pub fn draw_bitmap_rgba(&mut self, x: usize, y: usize, width: usize, bitmap: &Vec<u8>) {
        let initial_loc = (y * self.info.stride + x) * self.info.bytes_per_pixel;
        let mut location = initial_loc;
        let bytes_until_next_line = (self.info.stride - width) * self.info.bytes_per_pixel;

        let mut x_relative_to_zero = 0usize;

        if self.info.pixel_format == PixelFormat::Bgr {
            for rgba in bitmap.chunks(4) {
                if rgba[3] != 0 {
                    let color = Color {
                        r: rgba[0],
                        g: rgba[1],
                        b: rgba[2],
                    };

                    self.buffer[location] = color.b;
                    self.buffer[location + 1] = color.g;
                    self.buffer[location + 2] = color.r;
                }

                location += self.info.bytes_per_pixel;
                x_relative_to_zero += 1;

                if x_relative_to_zero % width == 0 {
                    location += bytes_until_next_line;
                    x_relative_to_zero = 0;
                }

                if location >= self.info.byte_len {
                    return;
                }
            }
        }
    }
}
