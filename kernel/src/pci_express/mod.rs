use crate::pci_express::device::PciDevice;
use acpi::mcfg::PciConfigEntry;
use acpi::{AcpiHandler, AcpiTables, PciConfigRegions};
use alloc::alloc::Global;
use alloc::vec;
use alloc::vec::Vec;
use log::debug;

mod device;
mod registers;

pub struct PCIe {
    devices: Option<Vec<PciDevice>>,
}

impl PCIe {
    pub fn new() -> Self {
        Self { devices: None }
    }

    fn scan_device_for_functions(
        &mut self,
        config_regions: &PciConfigRegions<Global>,
        segment_group_number: u16,
        bus: u8,
        device: u8,
    ) -> Vec<PciDevice> {
        let mut devices = vec![];

        for function in 1..8u8 {
            let address = match config_regions.physical_address(
                segment_group_number,
                bus,
                device,
                function,
            ) {
                Some(address) => address,
                None => {
                    debug!("No entry in MCFG that manages device.");
                    continue;
                }
            };

            let device = PciDevice::new(address);

            if !device.exists() {
                continue;
            }

            debug!(
                "Found device ({:04x}:{:04x}).",
                device.vendor_id(),
                device.device_id()
            );

            devices.push(device);
        }

        devices
    }

    fn scan_pci_config_entry(
        &mut self,
        config_regions: &PciConfigRegions<Global>,
        entry: PciConfigEntry,
    ) -> Vec<PciDevice> {
        let mut devices = vec![];

        debug!(
            "Scanning bus range {:?} for region with segment group {}, address {:08x}",
            entry.bus_range, entry.segment_group, entry.physical_address
        );

        for bus in entry.bus_range {
            for device in 0..31u8 {
                let address =
                    match config_regions.physical_address(entry.segment_group, bus, device, 0) {
                        Some(address) => address,
                        None => {
                            debug!("No entry in MCFG that manages device.");
                            continue;
                        }
                    };

                let pci_device = PciDevice::new(address);

                if !pci_device.exists() {
                    continue;
                }

                debug!(
                    "Found device ({:04x}:{:04x}).",
                    pci_device.vendor_id(),
                    pci_device.device_id()
                );

                if pci_device.has_multiple_functions() {
                    debug!("Device has multiple functions:");
                    let mut other_functions = self.scan_device_for_functions(
                        config_regions,
                        entry.segment_group,
                        bus,
                        device,
                    );

                    devices.append(&mut other_functions);
                    debug!("End of device's other functions.")
                }

                devices.push(pci_device);
            }
        }

        devices
    }

    pub fn scan<H>(&mut self, tables: &AcpiTables<H>)
    where
        H: AcpiHandler,
    {
        let config_regions = match PciConfigRegions::new_in(tables, &Global) {
            Ok(config_regions) => config_regions,
            Err(e) => {
                panic!("Couldn't get PCI config regions because of the following error. Make sure the device supports PCIe. {:#?}", e);
            }
        };

        let mut devices = vec![];
        for entry in config_regions.iter() {
            let mut found_devices = self.scan_pci_config_entry(&config_regions, entry);

            devices.append(&mut found_devices);
        }

        self.devices = Some(devices);
    }
}
