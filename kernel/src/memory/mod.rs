use crate::PHYSICAL_MEMORY_OFFSET;
use x86_64::VirtAddr;

pub mod frame_allocator;
pub mod heap;

pub fn map_physical_to_virtual(address: u64) -> VirtAddr {
    let offset = PHYSICAL_MEMORY_OFFSET.get().unwrap();

    VirtAddr::new(offset + address)
}

#[allow(unused)]
pub fn map_physical_to_virtual_mut<T>(address: u64) -> *mut T {
    map_physical_to_virtual(address).as_mut_ptr()
}

#[allow(unused)]
pub fn map_physical_to_virtual_const<T>(address: u64) -> *const T {
    map_physical_to_virtual(address).as_ptr()
}
