use crate::pci_express::device_capabilities::{PciDeviceCapability, PciDeviceMsiCapability};
use crate::pci_express::registers::ConfigRegionHeaderRegister;
use bit_field::BitField;
use core::fmt::{Debug, Formatter};
use core::mem::size_of;
use core::ops::BitAnd;
use x86_64::VirtAddr;

pub const PCI_DEVICE_NOT_EXIST_VENDOR_ID: u16 = 0xFFFF;

pub struct PciDevice(VirtAddr);

impl PciDevice {
    pub fn new(pci_address: VirtAddr) -> Self {
        Self(pci_address)
    }

    fn read_register<T>(&self, register: ConfigRegionHeaderRegister) -> T
    where
        T: BitAnd + TryFrom<u32>,
        <T as TryFrom<u32>>::Error: Debug,
    {
        let (offset, bit_range) = register.register_location_info();

        let actual_address = (self.0 + offset as u64).as_ptr::<u32>();

        let bits = unsafe { *actual_address }.get_bits(bit_range);
        let mask = (1 << (size_of::<T>() * 8)) as u32 - 1;

        T::try_from(bits & mask).unwrap()
    }

    pub fn vendor_id(&self) -> u16 {
        self.read_register(ConfigRegionHeaderRegister::VendorId)
    }

    pub fn device_id(&self) -> u16 {
        self.read_register(ConfigRegionHeaderRegister::DeviceId)
    }

    #[allow(unused)]
    pub fn has_capabilities_list(&self) -> bool {
        self.read_register::<u16>(ConfigRegionHeaderRegister::Status)
            .get_bit(4)
    }

    pub fn has_multiple_functions(&self) -> bool {
        self.read_register::<u8>(ConfigRegionHeaderRegister::HeaderType)
            .get_bit(7)
    }

    pub fn exists(&self) -> bool {
        self.vendor_id() != PCI_DEVICE_NOT_EXIST_VENDOR_ID
    }

    #[allow(unused)]
    pub fn capabilities(&self) -> Option<PciDeviceCapability> {
        if self.has_capabilities_list() {
            let offset =
                self.read_register::<u8>(ConfigRegionHeaderRegister::CapabilitiesPointer) & !0b11u8;

            let capability_offset = unsafe { *(self.0 + offset as u64).as_ptr() };
            let capability_option = PciDeviceMsiCapability::new(self.0, capability_offset);

            capability_option.map(|capability| PciDeviceCapability::MSI(capability))
        } else {
            None
        }
    }
}

impl Debug for PciDevice {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        let vendor_id = self.vendor_id();
        let device_id = self.device_id();

        match (vendor_id, device_id) {
            (0x8086, 0x2934) => write!(f, "82801I (ICH9 Family) USB UHCI Controller #1"),
            (0x8086, 0x2935) => write!(f, "82801I (ICH9 Family) USB UHCI Controller #2"),
            (0x8086, 0x2936) => write!(f, "82801I (ICH9 Family) USB UHCI Controller #3"),
            (0x8086, 0x293a) => write!(f, "82801I (ICH9 Family) USB2 EHCI Controller #1"),
            _ => write!(f, "({:04x}:{:04x})", vendor_id, device_id),
        }
    }
}
