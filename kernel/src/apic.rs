use acpi::platform::interrupt::Apic;
use conquer_once::spin::OnceCell;
use log::{debug, error, warn};
use x86_64::instructions::port::Port;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};

extern "x86-interrupt" fn irq_page_fault(
    stack_frame: InterruptStackFrame,
    code: PageFaultErrorCode,
) {
    panic!(
        "EXCEPTION: PAGE FAULT WITH CODE {:?},\n{:#?}",
        code, stack_frame
    );
}

static IDT: OnceCell<InterruptDescriptorTable> = OnceCell::uninit();

pub fn init(mut apic: Apic) {
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
    // x86_64::set_general_handler!(&mut idt, irq_fallback);
    idt.page_fault.set_handler_fn(irq_page_fault);

    let idt = IDT.get_or_init(move || idt);
    idt.load();

    unsafe {
        (apic.local_apic_address as *mut u32)
            .add(0xf0)
            .write_volatile(0xff)
    };
}
