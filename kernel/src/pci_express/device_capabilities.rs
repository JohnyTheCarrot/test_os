use alloc::format;
use core::fmt::{Debug, Formatter};
use x86_64::VirtAddr;

#[allow(unused)]
#[derive(Debug)]
pub enum PciDeviceCapability {
    MSI(PciDeviceMsiCapability),
    MSIX,
}

#[derive(Copy, Clone)]
pub struct PciDeviceMsiCapability {
    device_address: VirtAddr,
    capability_offset: usize,
}

impl PciDeviceMsiCapability {
    pub fn new(device_address: VirtAddr, capability_offset: u8) -> Option<Self> {
        let capability_id = unsafe { *device_address.as_ptr::<u8>() };

        if capability_id == 0x05 {
            let capability = Self {
                device_address,
                capability_offset: capability_offset as usize,
            };

            Some(capability)
        } else {
            None
        }
    }

    #[allow(unused)]
    pub fn next(&self) -> PciDeviceMsiCapability {
        let next_offset: u8 = unsafe {
            *self
                .device_address
                .as_ptr::<u8>()
                .byte_add(self.capability_offset)
        };

        unsafe {
            *self
                .device_address
                .as_ptr::<PciDeviceMsiCapability>()
                .byte_add(next_offset as usize)
        }
    }
}

impl Debug for PciDeviceMsiCapability {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        let regs = unsafe {
            *self
                .device_address
                .as_ptr::<[u32; 6]>()
                .byte_add(self.capability_offset)
        };

        let to_write = regs.map(|reg| format!("{:08x}", reg)).join(", ");

        write!(f, "{}", to_write)
    }
}
