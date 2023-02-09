use crate::memory::MemoryBlockRegionType::{Free, Used};
use crate::PHYSICAL_MEMORY_OFFSET;
use core::alloc::{GlobalAlloc, Layout};
use core::cell::UnsafeCell;
use core::ptr::null_mut;
use core::sync::atomic::AtomicUsize;
use log::{debug, warn};

const ORDER_0_BLOCK_AMOUNT: usize = 512;
const ORDER_0_BLOCK_SIZE: usize = 1024;
const NUMBER_0F_BYTES: usize = ORDER_0_BLOCK_AMOUNT * ORDER_0_BLOCK_SIZE;
const MAX_BLOCK_ORDER: usize = usize::ilog2(NUMBER_0F_BYTES) as usize;
const MAX_SUPPORTED_ALIGN: usize = 4096;

#[derive(Copy, Clone, Debug, PartialEq)]
enum MemoryBlockRegionType {
    Free,
    Used,
}

#[derive(Copy, Clone, Debug)]
pub struct MemoryBlockRegion {
    region_type: MemoryBlockRegionType,
    start_ptr: *mut u8,
    order: usize,
    left: *mut MemoryBlockRegion,
    right: *mut MemoryBlockRegion,
}

#[repr(C, align(4096))] // 4096 == MAX_SUPPORTED_ALIGN
pub struct Allocator {
    blocks: UnsafeCell<[u8; NUMBER_0F_BYTES]>,
    region: UnsafeCell<[Option<MemoryBlockRegion>; ORDER_0_BLOCK_AMOUNT]>,
    remaining: AtomicUsize,
}

unsafe impl Send for Allocator {}
unsafe impl Sync for Allocator {}

#[global_allocator]
static ALLOCATOR: Allocator = Allocator {
    blocks: UnsafeCell::new([0; NUMBER_0F_BYTES]),
    region: UnsafeCell::new([None; ORDER_0_BLOCK_AMOUNT]),
    remaining: AtomicUsize::new(ORDER_0_BLOCK_AMOUNT),
};

impl Allocator {
    unsafe fn find_block_region(
        &self,
        node_ptr: *mut MemoryBlockRegion,
        desired_order: usize,
    ) -> Option<MemoryBlockRegion> {
        if node_ptr == null_mut() {
            log::trace!("Node is null");
            return None;
        }

        let mut node = *node_ptr;

        if node.order == desired_order && node.region_type == Free {
            log::trace!("Found block of desired order at {:?}", node.start_ptr);
            return Some(node);
        }

        let mut left_node_ptr = node.left;
        let mut right_node_ptr = node.right;

        if left_node_ptr == null_mut() {
            // debug!("We need to split, jesse");
            // we need to split
            let left_node = MemoryBlockRegion {
                region_type: Free,
                start_ptr: node.start_ptr,
                order: node.order - 1,
                left: null_mut(),
                right: null_mut(),
            };
            // debug!("attempting to insert left node {:?}", left_node);
            let mut left_node_opt_ptr =
                &mut (*self.region.get())[0] as *mut Option<MemoryBlockRegion>;

            // debug!("first {:?}", left_node_opt_ptr);

            while let Some(mem) = *left_node_opt_ptr {
                // debug!("checked {:?}", mem);
                left_node_opt_ptr = left_node_opt_ptr.add(1);
                // debug!("moving to {:?}", left_node_opt_ptr);
            }

            // debug!("finished with ptr {:?}", left_node_opt_ptr);

            left_node_opt_ptr.write(Some(left_node));
            left_node_ptr = &mut (*left_node_opt_ptr).unwrap() as *mut MemoryBlockRegion;

            // debug!("*ptr = {:?}", *left_node_opt_ptr);
        }

        // if right_node_ptr == null_mut() {
        //     let right_node = MemoryBlockRegion {
        //         region_type: Free,
        //         start_ptr: (node.start_ptr as usize + 2usize.pow((node.order - 1) as u32))
        //             as *mut u8,
        //         order: node.order - 1,
        //         left: null_mut(),
        //         right: null_mut(),
        //     };
        //
        //     debug!("attempting to insert right node {:?}", right_node);
        //
        //     let right_node_opt_ptr = left_node_opt_ptr.add(1);
        //
        //     *right_node_opt_ptr = Some(right_node);
        //
        //     right_node_ptr = &mut (*right_node_opt_ptr).unwrap() as *mut MemoryBlockRegion;
        //
        //     // debug!("that worked");
        // }

        if let Some(mut block_region) = self.find_block_region(left_node_ptr, desired_order) {
            block_region.region_type = Used;
            return Some(block_region);
        }

        if right_node_ptr == null_mut() {}

        // if let Some(mut block_region) = self.find_block_region(right_node_ptr, desired_order) {
        //     block_region.region_type = Used;
        //     return Some(block_region);
        // }

        None
    }
}

unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let number_requested_blocks = layout.size().div_ceil(ORDER_0_BLOCK_SIZE);
        let align = layout.align();

        debug!(
            "Alloc of {} blocks with align {} requested",
            number_requested_blocks, align
        );
        debug!("MAX_BLOCK_ORDER = {}", MAX_BLOCK_ORDER);

        if align > MAX_SUPPORTED_ALIGN {
            return null_mut();
        }

        let order = number_requested_blocks.ilog2() as usize;

        if order > MAX_BLOCK_ORDER {
            log::warn!(
                "Requested allocation order exceeded MAX_BLOCK_ORDER ({} > {})",
                order,
                MAX_BLOCK_ORDER
            );
            return null_mut();
        }

        let mut root = match (*self.region.get()).first().unwrap() {
            Some(mut root) => &mut root as *mut MemoryBlockRegion,
            None => {
                let region = MemoryBlockRegion {
                    region_type: Free,
                    start_ptr: 2usize.pow(MAX_BLOCK_ORDER as u32) as *mut u8,
                    order: MAX_BLOCK_ORDER,
                    left: null_mut(),
                    right: null_mut(),
                };
                (*self.region.get())[0] = Some(region);

                &mut (*self.region.get())[0].unwrap() as *mut MemoryBlockRegion
            }
        };

        let align_mask_to_round_down = !(align - 1);

        let block_region = self.find_block_region(root, order);
        debug!("{:?}", block_region);
        if let Some(block_region) = block_region {
            block_region
                .start_ptr
                .add(*PHYSICAL_MEMORY_OFFSET.get().unwrap() as usize)
        } else {
            debug!("allocation returned null");
            null_mut()
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        warn!("REMINDER: IMPLEMENT DEALLOC!!!");
    }
}
