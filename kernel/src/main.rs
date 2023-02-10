#![feature(int_roundings)]
#![feature(abi_x86_interrupt)]
#![feature(panic_info_message)]
#![no_std]
#![no_main]

extern crate alloc;

mod acpi;
mod apic;
mod color;
mod frame_allocator;
mod framebuffer;
mod logger;
mod memory;
mod screen;
mod text_writer;

use crate::acpi::AcpiMapper;
use crate::frame_allocator::BootInfoFrameAllocator;
use crate::framebuffer::FrameBufferWrapper;
use crate::logger::Logger;
use crate::screen::Screen;
use ::acpi::madt::Madt;
use ::acpi::sdt::Signature;
use ::acpi::{AcpiTables, InterruptModel};
use bootloader_api::config::Mapping;
use bootloader_api::info::Optional;
use bootloader_api::{entry_point, BootInfo, BootloaderConfig};
use conquer_once::spin::OnceCell;
use core::fmt::{Debug, Write};
use core::panic::PanicInfo;
use lazy_static::lazy_static;
use log::{debug, error};
use spinning_top::Spinlock;
use x86_64::registers::control::Cr3;
use x86_64::structures::paging::{
    Mapper, OffsetPageTable, Page, PageTable, PageTableFlags, PhysFrame, Size2MiB, Size4KiB,
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
pub static PAGE_TABLE: OnceCell<Spinlock<OffsetPageTable>> = OnceCell::uninit();

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

    if let Optional::Some(framebuffer) = &mut boot_info.framebuffer {
        let info = framebuffer.info();
        let wrapper = FrameBufferWrapper {
            buffer: framebuffer.buffer_mut(),
            info,
        };

        SCREEN.get_or_init(|| Spinlock::new(Screen::new(wrapper)));
        Logger::init();
    }

    let offset_table = unsafe {
        OffsetPageTable::new(
            find_page_table(),
            VirtAddr::new(*PHYSICAL_MEMORY_OFFSET.get().unwrap()),
        )
    };

    let mut frame_allocator = unsafe { BootInfoFrameAllocator::init(&boot_info.memory_regions) };

    let mut page_table = PAGE_TABLE.get_or_init(move || Spinlock::new(offset_table));

    let frame_start = PhysFrame::containing_address(PhysAddr::new(
        boot_info.physical_memory_offset.into_option().unwrap(),
    ));
    let frame_end = PhysFrame::containing_address(PhysAddr::new(
        boot_info.physical_memory_offset.into_option().unwrap() + 2u64.pow(19),
    ));
    let page = Page::<Size4KiB>::containing_address(VirtAddr::new(0));

    for (i, frame) in PhysFrame::range_inclusive(frame_start, frame_end).enumerate() {
        let page = page + i as u64;

        unsafe {
            let _ = page_table
                .lock()
                .map_to(
                    page,
                    frame,
                    PageTableFlags::WRITABLE | PageTableFlags::PRESENT,
                    &mut frame_allocator,
                )
                .map(|f| f.flush());
        }
    }

    let apic_frame = PhysFrame::<Size4KiB>::containing_address(PhysAddr::new(0xFEE00000));

    unsafe {
        page_table
            .lock()
            .identity_map(
                apic_frame,
                PageTableFlags::WRITABLE | PageTableFlags::PRESENT,
                &mut frame_allocator,
            )
            .map(|f| f.flush())
            .unwrap();
    }

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

        let platform_info = acpi.platform_info().unwrap();

        let interrupt_model = platform_info.interrupt_model;

        let apic = if let InterruptModel::Apic(apic) = interrupt_model {
            apic
        } else {
            panic!("Unknown interrupt model");
        };

        debug!("Found APIC {:?}", apic);
        apic::init(apic);

        debug!("\nStartup done!\n");
    } else {
        panic!("No physical memory offset");
    }

    loop {
        x86_64::instructions::interrupts::disable();
        x86_64::instructions::hlt();
    }
}
