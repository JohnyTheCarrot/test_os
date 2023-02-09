use crate::framebuffer::FrameBufferWrapper;
use crate::logging::font_constants::BACKUP_CHAR;
use conquer_once::spin::OnceCell;
use core::cmp::max;
use core::fmt;
use core::fmt::Write;
use log::{Metadata, Record};
use noto_sans_mono_bitmap::{get_raster, RasterizedChar};
use spinning_top::Spinlock;

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
    frame_buffer_wrapper: FrameBufferWrapper<'static>,
    window_scroll: i32,
    cursor_x: usize,
    cursor_y: usize,
    render_x: usize,
    render_y: usize,
}

impl FrameBufferTextWriter {
    pub fn new(frame_buffer_wrapper: FrameBufferWrapper<'static>) -> Self {
        Self {
            text_buffer: ['/'; CHAR_BUFFER_LENGTH],
            frame_buffer_wrapper,
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

    fn full_render(&mut self) {
        self.clear();

        for char_index in max(-self.window_scroll * COLUMNS as i32, 0) as usize..CHAR_BUFFER_LENGTH
        {
            let c = self.text_buffer[char_index];
            self.render_char(c);
        }
    }

    fn scroll(&mut self, diff: i32) {
        self.window_scroll += diff;

        self.full_render();
    }

    fn carriage_return(&mut self) {
        self.render_x = BORDER_PADDING;
        self.cursor_x = 0;
    }

    pub fn clear(&mut self) {
        self.frame_buffer_wrapper.buffer.fill(0);
        self.render_x = BORDER_PADDING;
        self.render_y = BORDER_PADDING;
    }

    fn render_char(&mut self, c: char) {
        let raster = get_char_raster(c);

        // todo: optimize
        for (y, row) in raster.raster().iter().enumerate() {
            for (x, byte) in row.iter().enumerate() {
                self.frame_buffer_wrapper.write_format_agnostic_pixel(
                    self.render_x + x,
                    self.render_y + y,
                    *byte,
                );
            }
        }
        self.render_x += raster.width() + LETTER_SPACING;
    }

    pub fn write_char(&mut self, c: char) {
        match c {
            '\n' => self.newline(),
            '\r' => self.carriage_return(),
            c => {
                let new_x = self.render_x + font_constants::CHAR_RASTER_WIDTH;

                if new_x >= self.frame_buffer_wrapper.info.width {
                    self.newline()
                }

                if self.render_y >= self.frame_buffer_wrapper.info.height - BORDER_PADDING {
                    // self.scroll(-1);
                    self.clear();
                }

                self.render_char(c);
            }
        }
    }
}

impl Write for FrameBufferTextWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.chars() {
            self.write_char(c);
        }

        Ok(())
    }
}

pub struct Logger {
    framebuffer_writer: Spinlock<FrameBufferTextWriter>,
}

impl Logger {
    fn new(framebuffer_writer: FrameBufferTextWriter) -> Self {
        let framebuffer_writer = Spinlock::new(framebuffer_writer);

        Self { framebuffer_writer }
    }

    pub fn force_unlock(&self) {
        unsafe { self.framebuffer_writer.force_unlock() }
    }
}

impl log::Log for Logger {
    fn enabled(&self, _: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        let mut writer = self.framebuffer_writer.lock();

        writeln!(writer, "{:5}: {}", record.level(), record.args()).unwrap()
    }

    fn flush(&self) {}
}

pub static LOGGER: OnceCell<Logger> = OnceCell::uninit();

pub fn init_logger(framebuffer_writer: FrameBufferWrapper<'static>) {
    let mut writer = FrameBufferTextWriter::new(framebuffer_writer);
    writer.clear();

    let logger = LOGGER.get_or_init(move || Logger::new(writer));

    log::set_logger(logger).expect("logger already initialized");
    log::set_max_level(log::LevelFilter::Trace);
    log::info!("Logger enabled!");
}
