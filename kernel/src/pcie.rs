use crate::PHYSICAL_MEMORY_OFFSET;
use acpi::mcfg::PciConfigEntry;
use acpi::{AcpiHandler, AcpiTables, PciConfigRegions};
use alloc::alloc::Global;
use log::debug;
use pci_types::{ConfigRegionAccess, EndpointHeader, HeaderType, PciAddress, PciHeader};

struct PciConfigRegionAccess<'a> {
    config_regions: &'a PciConfigRegions<'a, Global>,
}

impl<'a> PciConfigRegionAccess<'a> {
    pub fn new(config_regions: &'a PciConfigRegions<'a, Global>) -> PciConfigRegionAccess {
        Self { config_regions }
    }
}

impl<'a> ConfigRegionAccess for PciConfigRegionAccess<'_> {
    fn function_exists(&self, _address: PciAddress) -> bool {
        true
    }

    unsafe fn read(&self, address: PciAddress, offset: u16) -> u32 {
        let addr = self
            .config_regions
            .physical_address(
                address.segment(),
                address.bus(),
                address.device(),
                address.function(),
            )
            .unwrap();

        let ptr = (PHYSICAL_MEMORY_OFFSET.get().unwrap() + addr + offset as u64) as *mut u32;

        ptr.read_volatile()
    }

    unsafe fn write(&self, address: PciAddress, offset: u16, value: u32) {
        let addr = self
            .config_regions
            .physical_address(
                address.segment(),
                address.bus(),
                address.device(),
                address.function(),
            )
            .unwrap();

        let ptr = (PHYSICAL_MEMORY_OFFSET.get().unwrap() + addr + offset as u64) as *mut u32;

        ptr.write_volatile(value);
    }
}

pub struct PCIe<'a> {
    config_regions: PciConfigRegions<'a, Global>,
}

impl<'a> PCIe<'_> {
    pub fn new<H>(tables: &AcpiTables<H>) -> Self
    where
        H: AcpiHandler,
    {
        let config_regions = match PciConfigRegions::new_in(tables, &Global) {
            Ok(config_regions) => config_regions,
            Err(e) => {
                panic!("Couldn't get PCI config regions because of the following error. Make sure the device supports PCIe. {:#?}", e);
            }
        };

        let config_region_access = PciConfigRegionAccess::new(&config_regions);

        for entry in config_regions.iter() {
            PCIe::scan(&config_regions, &config_region_access, &entry);
        }

        Self { config_regions }
    }

    fn scan(
        config_regions: &PciConfigRegions<Global>,
        region_access: &PciConfigRegionAccess,
        entry: &PciConfigEntry,
    ) {
        debug!(
            "Scanning bus range {:?} for region with segment group {}, address {:08x}",
            entry.bus_range, entry.segment_group, entry.physical_address
        );

        // todo: replace with recursive scan
        for bus in entry.bus_range.clone() {
            for device in 0..31u8 {
                let address = PciAddress::new(entry.segment_group, bus, device, 0);

                let header = PciHeader::new(address);
                if header.id(region_access).0 == 0xFFFF {
                    continue;
                }

                if header.has_multiple_functions(region_access) {
                    debug!("PCIe device has multiple functions, iterating...");
                    for function in 0..8u8 {
                        let address = PciAddress::new(entry.segment_group, bus, device, function);

                        let header = PciHeader::new(address);

                        if let Some(endpoint) = EndpointHeader::from_header(header, region_access) {
                            let interrupt_pin =
                                unsafe { (region_access.read(address, 0x3c) & 0xFF00) >> 8 };

                            debug!(
                                "- Found PCIe endpoint-type device ({:02x}, {:02x}) with IRQ pin {:?}",
                                endpoint.header().id(region_access).0,
                                endpoint.header().id(region_access).1,
                                interrupt_pin
                            );
                        }
                    }
                    continue;
                }

                if let Some(endpoint) = EndpointHeader::from_header(header, region_access) {
                    let interrupt_pin =
                        unsafe { (region_access.read(address, 0x3c) & 0xFF00) >> 8 };

                    debug!(
                        "Found PCIe endpoint-type device ({:02x}, {:02x}) with IRQ pin {:?}",
                        endpoint.header().id(region_access).0,
                        endpoint.header().id(region_access).1,
                        interrupt_pin
                    );
                }
            }
        }
    }
}
