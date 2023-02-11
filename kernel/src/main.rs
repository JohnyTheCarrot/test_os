#![feature(int_roundings)]
#![feature(abi_x86_interrupt)]
#![feature(panic_info_message)]
#![feature(iter_next_chunk)]
#![feature(allocator_api)]
#![no_std]
#![no_main]

extern crate alloc;

mod acpi;
mod apic;
mod color;
mod framebuffer;
mod logger;
mod memory;
mod pcie;
mod screen;
mod text_writer;

use crate::acpi::AcpiMapper;
use crate::framebuffer::FrameBufferWrapper;
use crate::logger::Logger;
use crate::memory::frame_allocator::BootInfoFrameAllocator;
use crate::memory::heap;
use crate::screen::Screen;
use ::acpi::mcfg::Mcfg;
use ::acpi::sdt::Signature;
use ::acpi::{AcpiError, AcpiTables, InterruptModel, PciConfigRegions};
use alloc::alloc::Global;
use alloc::boxed::Box;
use alloc::format;
use bootloader_api::config::Mapping;
use bootloader_api::info::Optional;
use bootloader_api::{entry_point, BootInfo, BootloaderConfig};
use conquer_once::spin::OnceCell;
use core::fmt::Write;
use core::ops::Deref;
use core::panic::PanicInfo;
use lazy_static::lazy_static;
use log::{debug, info};
use spinning_top::Spinlock;
use x86_64::registers::control::Cr3;
use x86_64::structures::paging::{
    Mapper, OffsetPageTable, PageTable, PageTableFlags, PhysFrame, Size4KiB,
};
use x86_64::{PhysAddr, VirtAddr};

pub static BOOTLOADER_CONFIG: BootloaderConfig = {
    let mut config = BootloaderConfig::new_default();
    config.mappings.physical_memory = Some(Mapping::Dynamic);
    config
};

entry_point!(kernel_main, config = &BOOTLOADER_CONFIG);

lazy_static! {
    pub static ref SCREEN: OnceCell<Spinlock<Screen>> = OnceCell::uninit();
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    let screen = SCREEN.get().unwrap();
    unsafe { screen.force_unlock() };

    let mut screen_inner = screen.lock();
    info.message()
        .map(|message| writeln!(screen_inner, "\n\nKernel panic\n\n{}", message));

    writeln!(screen_inner, "{:?}", info.location()).unwrap();

    writeln!(screen_inner, "\n\nDisabling interrupts and halting CPU.").unwrap();

    loop {
        x86_64::instructions::interrupts::disable();
        x86_64::instructions::hlt();
    }
}

pub static PHYSICAL_MEMORY_OFFSET: OnceCell<u64> = OnceCell::uninit();
// pub static PAGE_TABLE: OnceCell<Spinlock<OffsetPageTable>> = OnceCell::uninit();

unsafe fn find_page_table() -> &'static mut PageTable {
    let (level_4_table_frame, _) = Cr3::read();

    let phys = level_4_table_frame.start_address();
    let virt = PHYSICAL_MEMORY_OFFSET.get().unwrap() + phys.as_u64();
    let page_table_ptr: *mut PageTable = virt as *mut PageTable;

    &mut *page_table_ptr
}

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    PHYSICAL_MEMORY_OFFSET.init_once(|| {
        boot_info
            .physical_memory_offset
            .into_option()
            .expect("no physical memory offset found")
    });

    let mut screen: Option<&Spinlock<Screen>> = None;

    if let Optional::Some(framebuffer) = &mut boot_info.framebuffer {
        let info = framebuffer.info();
        let wrapper = FrameBufferWrapper {
            buffer: framebuffer.buffer_mut(),
            info,
        };

        screen = Some(SCREEN.get_or_init(|| Spinlock::new(Screen::new(wrapper))));
        Logger::init();
    }

    let mut offset_table = unsafe {
        OffsetPageTable::new(
            find_page_table(),
            VirtAddr::new(*PHYSICAL_MEMORY_OFFSET.get().unwrap()),
        )
    };

    let mut frame_allocator = unsafe { BootInfoFrameAllocator::init(&boot_info.memory_regions) };

    let apic_frame = PhysFrame::<Size4KiB>::containing_address(PhysAddr::new(0xFEE00000));

    unsafe {
        offset_table
            .identity_map(
                apic_frame,
                PageTableFlags::WRITABLE | PageTableFlags::PRESENT,
                &mut frame_allocator,
            )
            .map(|f| f.flush())
            .unwrap();
    }
    let apic_frame = PhysFrame::<Size4KiB>::containing_address(PhysAddr::new(0xFEC00000));

    unsafe {
        offset_table
            .identity_map(
                apic_frame,
                PageTableFlags::WRITABLE | PageTableFlags::PRESENT,
                &mut frame_allocator,
            )
            .map(|f| f.flush())
            .unwrap();
    }

    heap::init_heap(offset_table, &mut frame_allocator);

    if let Some(offset) = PHYSICAL_MEMORY_OFFSET.get() {
        let handler = AcpiMapper {
            physical_memory_offset: *offset,
        };

        let acpi = unsafe {
            AcpiTables::from_rsdp(
                handler,
                boot_info.rsdp_addr.into_option().expect("no rsdp address") as usize,
            )
        }
        .expect("ACPI failed");

        let platform_info = acpi.platform_info_in(&Global).unwrap();

        let interrupt_model = platform_info.interrupt_model;

        let apic = if let InterruptModel::Apic(apic) = interrupt_model {
            apic
        } else {
            panic!("Unknown interrupt model");
        };

        let io_apic = apic.io_apics.first().unwrap();
        let addr = io_apic.address;

        let version_and_max_redirections = unsafe {
            (addr as *mut u32).write_volatile(0x01);
            ((addr + 0x10) as *mut u32).read_volatile()
        };

        let version = version_and_max_redirections & 0xFF;
        let max_redirections = (version_and_max_redirections >> 16) + 1;

        // debug!("Found APIC {:?}", apic);
        apic::init(apic);

        pcie::PCIe::new(&acpi);

        info!("Startup done!\n");
        info!("Loading friend...");

        let test_image_1 = include_bytes!("assets/test_image.png");

        screen.map(|locked_screen| {
            let mut screen = locked_screen.lock();

            screen.use_frame_buffer(|fb| {
                let (header, image_data) = png_decoder::decode(test_image_1).unwrap();

                fb.draw_bitmap_rgba(
                    fb.info.width - header.width as usize - 50,
                    fb.info.height - header.height as usize - 50,
                    header.width as usize,
                    &image_data,
                );
            });
        });

        info!("Friend time!");
    } else {
        panic!("No physical memory offset");
    }

    loop {
        x86_64::instructions::interrupts::disable();
        x86_64::instructions::hlt();
    }
}
