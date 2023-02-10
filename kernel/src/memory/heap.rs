use linked_list_allocator::LockedHeap;
use log::debug;
use x86_64::structures::paging::mapper::MapToError;
use x86_64::structures::paging::{FrameAllocator, Mapper, Page, PageTableFlags, Size4KiB};
use x86_64::VirtAddr;

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

pub const HEAP_START: *mut u8 = 0x_4444_4444_0000 as *mut u8;
pub const HEAP_SIZE: usize = 9000 * 1024;

fn map_heap<M, A>(mut mapper: M, frame_allocator: &mut A) -> Result<(), MapToError<Size4KiB>>
where
    M: Mapper<Size4KiB>,
    A: FrameAllocator<Size4KiB> + ?Sized,
{
    let page_range = {
        let heap_start = VirtAddr::new(HEAP_START as u64);
        let heap_end = heap_start + HEAP_SIZE as u64 - 1u64;

        let start_page = Page::containing_address(heap_start);
        let end_page = Page::containing_address(heap_end);

        Page::range_inclusive(start_page, end_page)
    };

    for page in page_range {
        let frame = frame_allocator
            .allocate_frame()
            .ok_or(MapToError::FrameAllocationFailed)?;

        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;

        unsafe { mapper.map_to(page, frame, flags, frame_allocator)?.flush() };
    }

    Ok(())
}

pub fn init_heap<M, A>(mut mapper: M, frame_allocator: &mut A)
where
    M: Mapper<Size4KiB>,
    A: FrameAllocator<Size4KiB> + ?Sized,
{
    map_heap(mapper, frame_allocator).expect("Couldn't map heap pages");

    debug!(
        "Initializing heap at {:?} of {} bytes",
        HEAP_START, HEAP_SIZE
    );
    unsafe { ALLOCATOR.lock().init(HEAP_START, HEAP_SIZE) }

    debug!("Heap initialized");
}
