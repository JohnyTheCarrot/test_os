use acpi::{AcpiHandler, PhysicalMapping};
use core::ptr::NonNull;
use log::debug;

#[derive(Clone)]
pub struct AcpiMapper {
    pub physical_memory_offset: u64,
}

impl AcpiHandler for AcpiMapper {
    unsafe fn map_physical_region<T>(
        &self,
        physical_address: usize,
        size: usize,
    ) -> PhysicalMapping<Self, T> {
        let mapping: PhysicalMapping<Self, T> = PhysicalMapping::new(
            physical_address,
            NonNull::new((physical_address + self.physical_memory_offset as usize) as *mut _)
                .unwrap(),
            size,
            size,
            self.clone(),
        );

        debug!(
            "mapping physical address 0x{:08x} to virtual address {:?}",
            physical_address,
            mapping.virtual_start()
        );

        mapping
    }

    fn unmap_physical_region<T>(_region: &PhysicalMapping<Self, T>) {}
}
