use crate::framebuffer::FrameBufferWrapper;
use crate::text_writer::font_constants::BACKUP_CHAR;
use core::cmp::max;
use noto_sans_mono_bitmap::{get_raster, RasterizedChar};

mod font_constants {
    use noto_sans_mono_bitmap::{get_raster_width, FontWeight, RasterHeight};

    pub const CHAR_RASTER_HEIGHT: RasterHeight = RasterHeight::Size16;

    pub const CHAR_RASTER_WIDTH: usize = get_raster_width(FontWeight::Regular, CHAR_RASTER_HEIGHT);

    pub const BACKUP_CHAR: char = '?';

    pub const FONT_WEIGHT: FontWeight = FontWeight::Regular;
}

fn get_char_raster(c: char) -> RasterizedChar {
    fn get(c: char) -> Option<RasterizedChar> {
        get_raster(
            c,
            font_constants::FONT_WEIGHT,
            font_constants::CHAR_RASTER_HEIGHT,
        )
    }

    get(c).unwrap_or_else(|| get(BACKUP_CHAR).expect("Wasn't able to get backup char raster."))
}

const LINE_SPACING: usize = 8;
const BORDER_PADDING: usize = 8;
const LETTER_SPACING: usize = 0;
const ROWS: usize = 24;
const COLUMNS: usize = 24;
const TERMINAL_PAGES: usize = 1;
const CHAR_BUFFER_LENGTH: usize = ROWS * COLUMNS * TERMINAL_PAGES;
const LINE_HEIGHT: usize = font_constants::CHAR_RASTER_HEIGHT.val() + LINE_SPACING;

pub struct FrameBufferTextWriter {
    text_buffer: [char; CHAR_BUFFER_LENGTH],
    window_scroll: i32,
    cursor_x: usize,
    cursor_y: usize,
    render_x: usize,
    render_y: usize,
}

impl FrameBufferTextWriter {
    pub fn new() -> Self {
        Self {
            text_buffer: [' '; CHAR_BUFFER_LENGTH],
            window_scroll: 0,
            cursor_x: 0,
            cursor_y: 0,
            render_x: BORDER_PADDING,
            render_y: BORDER_PADDING,
        }
    }

    fn newline(&mut self) {
        self.render_y += LINE_HEIGHT;
        self.cursor_y += 1;
        self.carriage_return();
    }

    fn full_render(&mut self, mut frame_buffer_wrapper: &mut FrameBufferWrapper) {
        self.clear(frame_buffer_wrapper);

        for char_index in max(-self.window_scroll * COLUMNS as i32, 0) as usize..CHAR_BUFFER_LENGTH
        {
            let c = self.text_buffer[char_index];
            self.render_char(c, &mut frame_buffer_wrapper);
        }
    }

    fn scroll(&mut self, diff: i32, frame_buffer_wrapper: &mut FrameBufferWrapper) {
        self.window_scroll += diff;

        self.full_render(frame_buffer_wrapper);
    }

    fn carriage_return(&mut self) {
        self.render_x = BORDER_PADDING;
        self.cursor_x = 0;
    }

    pub fn clear(&mut self, frame_buffer_wrapper: &mut FrameBufferWrapper) {
        frame_buffer_wrapper.buffer.fill(0);
        self.render_x = BORDER_PADDING;
        self.render_y = BORDER_PADDING;
    }

    fn render_char(&mut self, c: char, frame_buffer_wrapper: &mut FrameBufferWrapper) {
        let raster = get_char_raster(c);

        // todo: optimize
        for (y, row) in raster.raster().iter().enumerate() {
            for (x, byte) in row.iter().enumerate() {
                if byte == &0 {
                    continue;
                }

                frame_buffer_wrapper.write_format_agnostic_pixel(
                    self.render_x + x,
                    self.render_y + y,
                    *byte,
                );
            }
        }
        self.render_x += raster.width() + LETTER_SPACING;
    }

    pub fn write_char(&mut self, c: char, frame_buffer_wrapper: &mut FrameBufferWrapper) {
        match c {
            '\n' => self.newline(),
            '\r' => self.carriage_return(),
            c => {
                let new_x = self.render_x + font_constants::CHAR_RASTER_WIDTH;

                if new_x >= frame_buffer_wrapper.info.width - BORDER_PADDING {
                    self.newline()
                }

                if self.render_y >= frame_buffer_wrapper.info.height - BORDER_PADDING {
                    // self.scroll(-1);
                    self.clear(frame_buffer_wrapper);
                }

                self.render_char(c, frame_buffer_wrapper);
            }
        }
    }

    pub fn write_str(&mut self, text: &str, frame_buffer_wrapper: &mut FrameBufferWrapper) {
        for c in text.chars() {
            self.write_char(c, frame_buffer_wrapper);
        }
    }
}