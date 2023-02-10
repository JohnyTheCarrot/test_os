use crate::framebuffer::FrameBufferWrapper;
use crate::logger::Logger;
use crate::text_writer::FrameBufferTextWriter;
use core::fmt;
use spinning_top::Spinlock;

pub struct Screen {
    frame_buffer_wrapper: Spinlock<FrameBufferWrapper<'static>>,
    text_writer: Spinlock<FrameBufferTextWriter>,
}

impl Screen {
    pub fn new(wrapper: FrameBufferWrapper<'static>) -> Self {
        let screen = Self {
            frame_buffer_wrapper: Spinlock::new(wrapper),
            text_writer: Spinlock::new(FrameBufferTextWriter::new()),
        };

        screen
            .text_writer
            .lock()
            .clear(&mut screen.frame_buffer_wrapper.lock());

        screen
    }

    pub fn init(&self) {
        Logger::init();
    }

    pub fn use_frame_buffer<F>(&self, func: F)
    where
        F: FnOnce(&mut FrameBufferWrapper),
    {
        let mut wrapper = self.frame_buffer_wrapper.lock();
        func(&mut wrapper);
    }

    pub fn use_text_writer<F>(&self, func: F)
    where
        F: FnOnce(&mut FrameBufferTextWriter),
    {
        let mut wrapper = self.text_writer.lock();
        func(&mut wrapper);
    }
}

impl fmt::Write for Screen {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let mut writer = self.text_writer.lock();
        let mut buffer = self.frame_buffer_wrapper.lock();

        writer.write_str(s, &mut buffer);

        Ok(())
    }
}
