use core::ops::RangeInclusive;

#[allow(unused)]
pub enum ConfigRegionHeaderRegister {
    VendorId,
    DeviceId,

    Command,
    Status,

    RevisionId,
    ProgrammingInterface,
    SubClass,
    ClassCode,

    CacheLineSize,
    LatencyTimer,
    HeaderType,
    BIST,

    // Header Type 0
    Bar0,
    Bar1,
    Bar2,
    Bar3,
    Bar4,
    Bar5,
    CardBusCISPointer,

    SubsystemVendorId,
    SubsystemId,

    ExpansionROMBaseAddress,

    CapabilitiesPointer,

    InterruptLine,
    InterruptPIN,
    MinGrant,
    MaxLatency,
}

impl ConfigRegionHeaderRegister {
    pub fn register_location_info(&self) -> (u8, RangeInclusive<usize>) {
        match *self {
            Self::VendorId => (0x00, 0..=15),
            Self::DeviceId => (0x00, 16..=31),
            Self::Command => (0x04, 0..=15),
            Self::Status => (0x04, 16..=31),
            Self::RevisionId => (0x08, 0..=7),
            Self::ProgrammingInterface => (0x08, 8..=15),
            Self::SubClass => (0x08, 16..=23),
            Self::ClassCode => (0x08, 24..=31),
            Self::CacheLineSize => (0x0C, 0..=7),
            Self::LatencyTimer => (0x0C, 8..=15),
            Self::HeaderType => (0x0C, 16..=23),
            Self::BIST => (0x0C, 24..=31),
            Self::Bar0 => (0x10, 0..=31),
            Self::Bar1 => (0x14, 0..=31),
            Self::Bar2 => (0x18, 0..=31),
            Self::Bar3 => (0x1C, 0..=31),
            Self::Bar4 => (0x20, 0..=31),
            Self::Bar5 => (0x24, 0..=31),
            Self::CardBusCISPointer => (0x28, 0..=31),
            Self::SubsystemVendorId => (0x2C, 0..=15),
            Self::SubsystemId => (0x2C, 16..=31),
            Self::ExpansionROMBaseAddress => (0x30, 0..=31),
            Self::CapabilitiesPointer => (0x34, 0..=7),
            Self::InterruptLine => (0x3C, 0..=7),
            Self::InterruptPIN => (0x3C, 8..=15),
            Self::MinGrant => (0x3C, 16..=23),
            Self::MaxLatency => (0x3C, 24..=31),
        }
    }
}
