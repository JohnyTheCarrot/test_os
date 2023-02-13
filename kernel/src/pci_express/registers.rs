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
        }
    }
}
