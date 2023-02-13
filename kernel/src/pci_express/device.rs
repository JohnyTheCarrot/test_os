use crate::pci_express::registers::ConfigRegionHeaderRegister;
use crate::PHYSICAL_MEMORY_OFFSET;
use bit_field::BitField;
use core::fmt::Debug;
use core::mem::size_of;
use core::ops::BitAnd;

pub const PCI_DEVICE_NOT_EXIST_VENDOR_ID: u16 = 0xFFFF;

pub struct PciDevice(u64);

impl PciDevice {
    pub fn new(pci_address: u64) -> Self {
        Self(pci_address)
    }

    fn read_register<T>(&self, register: ConfigRegionHeaderRegister) -> T
    where
        T: BitAnd + TryFrom<u32>,
        <T as TryFrom<u32>>::Error: Debug,
    {
        let (offset, bit_range) = register.register_location_info();

        let ptr = (PHYSICAL_MEMORY_OFFSET.get().unwrap() + self.0 + offset as u64) as *mut u32;
        let bits = unsafe { ptr.read_volatile() }.get_bits(bit_range);
        let mask = (1 << (size_of::<T>() * 8)) as u32 - 1;

        T::try_from(bits & mask).unwrap()
    }

    pub fn vendor_id(&self) -> u16 {
        self.read_register(ConfigRegionHeaderRegister::VendorId)
    }

    pub fn device_id(&self) -> u16 {
        self.read_register(ConfigRegionHeaderRegister::DeviceId)
    }

    pub fn has_multiple_functions(&self) -> bool {
        self.read_register::<u8>(ConfigRegionHeaderRegister::HeaderType)
            .get_bit(7)
    }

    pub fn exists(&self) -> bool {
        self.vendor_id() != PCI_DEVICE_NOT_EXIST_VENDOR_ID
    }
}
