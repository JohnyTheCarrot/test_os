#![feature(int_roundings)]
#![feature(abi_x86_interrupt)]
#![feature(panic_info_message)]
#![no_std]
#![no_main]

extern crate alloc;

mod acpi;
mod apic;
mod color;
mod framebuffer;
mod logging;
mod memory;

use crate::acpi::AcpiMapper;
use crate::framebuffer::FrameBufferWrapper;
use crate::logging::{init_logger, LOGGER};
use ::acpi::madt::Madt;
use ::acpi::sdt::Signature;
use ::acpi::{AcpiTable, AcpiTables, InterruptModel};
use bootloader_api::config::Mapping;
use bootloader_api::info::Optional;
use bootloader_api::{entry_point, BootInfo, BootloaderConfig};
use conquer_once::spin::OnceCell;
use core::panic::PanicInfo;
use log::debug;

pub static BOOTLOADER_CONFIG: BootloaderConfig = {
    let mut config = BootloaderConfig::new_default();
    config.mappings.physical_memory = Some(Mapping::Dynamic);
    config
};

entry_point!(kernel_main, config = &BOOTLOADER_CONFIG);

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    LOGGER.get().map(|logger| logger.force_unlock());

    info.message().map(|message| {
        log::error!("Kernel panic:\n\n{}\n\n", message);
    });

    log::error!("{:?}", info.location());

    loop {
        x86_64::instructions::hlt();
    }
}

pub static PHYSICAL_MEMORY_OFFSET: OnceCell<u64> = OnceCell::uninit();

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
        init_logger(wrapper);
    }

    if let Optional::Some(offset) = boot_info.physical_memory_offset {
        let handler = AcpiMapper {
            physical_memory_offset: offset,
        };

        let acpi = unsafe {
            AcpiTables::from_rsdp(
                handler,
                boot_info.rsdp_addr.into_option().expect("no rsdp address") as usize,
            )
        }
        .expect("ACPI failed");

        let multi_apic_description_table = unsafe { acpi.get_sdt::<Madt>(Signature::MADT) }
            .expect("Couldn't get MADT")
            .unwrap();

        let (interrupt_model, processor_info) = multi_apic_description_table
            .parse_interrupt_model()
            .expect("error parsing interrupt model");

        let apic = if let InterruptModel::Apic(apic) = interrupt_model {
            apic
        } else {
            panic!("Unknown interrupt model");
        };

        debug!("Found APIC {:?}", apic);
        apic::init(apic);

        loop {
            debug!("Waiting for interrupt..");
            x86_64::instructions::hlt();
        }
    } else {
        panic!("No physical memory offset");
    }

    loop {
        x86_64::instructions::interrupts::disable();
        x86_64::instructions::hlt();
    }
}
