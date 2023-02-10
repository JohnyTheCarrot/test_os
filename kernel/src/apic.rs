use crate::PHYSICAL_MEMORY_OFFSET;
use acpi::platform::interrupt::Apic;
use conquer_once::spin::OnceCell;
use log::{debug, warn};
use x86_64::instructions::port::Port;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};

fn irq_fallback(_stack_frame: InterruptStackFrame, index: u8, _code: Option<u64>) {
    warn!("Unhandled IRQ {}", index);
}

fn irq_page_fault(stack_frame: InterruptStackFrame, _index: u8, code: Option<u64>) {
    panic!(
        "EXCEPTION: PAGE FAULT WITH CODE {:?},\n{:#?}",
        code, stack_frame
    );
}

static IDT: OnceCell<InterruptDescriptorTable> = OnceCell::uninit();

pub fn init(apic: Apic) {
    let base_addr = *PHYSICAL_MEMORY_OFFSET.get().unwrap() as usize;

    if apic.also_has_legacy_pics {
        debug!("Has legacy PIC, disabling..");
        // disable pic8259
        let mut pic1 = Port::<u8>::new(0xa1);
        let mut pic2 = Port::<u8>::new(0x21);
        unsafe {
            pic1.write(u8::MAX);
            pic2.write(u8::MAX);
        }
        debug!("Legacy PIC disabled.");
    }

    debug!("Creating interrupt descriptor table..");
    let mut idt = InterruptDescriptorTable::new();
    x86_64::set_general_handler!(&mut idt, irq_fallback);
    x86_64::set_general_handler!(&mut idt, irq_page_fault, 14);

    let idt = IDT.get_or_init(move || idt);
    idt.load();

    debug!("Loaded interrupt descriptor table, writing 0x1FF to the Spurious Interrupt Vector Register.");

    unsafe {
        let reg = (apic.local_apic_address + 0xf0) as *mut u32;

        debug!(
            "Phys. local APIC address = {:?}, + {:08x} = virtaddr {:?}, writing 0x1FF",
            apic.local_apic_address as *mut u32,
            *PHYSICAL_MEMORY_OFFSET.get().unwrap() as usize,
            reg
        );

        reg.write_volatile(0x1FF);
    };

    debug!("Interrupts set up.");
}
