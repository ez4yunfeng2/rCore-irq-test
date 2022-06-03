use super::BlockDevice;
use crate::drivers::{complete, IRQ_TASKS};
use crate::sync::UPSafeCell;
use crate::task::awake_by_irq_and_run;
use crate::{
    mm::{
        frame_alloc, frame_dealloc, kernel_token, FrameTracker, PageTable, PhysAddr, PhysPageNum,
        StepByOne, VirtAddr,
    },
    task::wait_irq_and_run_next,
};
use alloc::vec::Vec;
use lazy_static::*;
use virtio_drivers::{VirtIOBlk, VirtIOHeader};

#[allow(unused)]
const VIRTIO0: usize = 0x10008000;

pub struct VirtIOBlock(UPSafeCell<VirtIOBlk<'static>>);

lazy_static! {
    static ref QUEUE_FRAMES: UPSafeCell<Vec<FrameTracker>> = unsafe { UPSafeCell::new(Vec::new()) };
}

impl BlockDevice for VirtIOBlock {
    fn read_block(&self, block_id: usize, buf: &mut [u8]) {
        self.0
            .exclusive_access()
            .read_block_irq(block_id, buf, &|| wait_irq_and_run_next(8))
            .expect("Error when reading VirtIOBlk");
    }

    fn write_block(&self, block_id: usize, buf: &[u8]) {
        self.0
            .exclusive_access()
            .write_block_irq(block_id, buf, &|| wait_irq_and_run_next(8))
            .expect("Error when writing VirtIOBlk");
    }

    fn handler_interrupt(&self) {
        complete(8);
        awake_by_irq_and_run(8);
    }
}

impl VirtIOBlock {
    #[allow(unused)]
    pub fn new() -> Self {
        IRQ_TASKS.init_queue(8);
        unsafe {
            Self(UPSafeCell::new(
                VirtIOBlk::new(&mut *(VIRTIO0 as *mut VirtIOHeader)).unwrap(),
            ))
        }
    }
}

#[no_mangle]
pub extern "C" fn virtio_dma_alloc(pages: usize) -> PhysAddr {
    let mut ppn_base = PhysPageNum(0);
    for i in 0..pages {
        let frame = frame_alloc().unwrap();
        if i == 0 {
            ppn_base = frame.ppn;
        }
        assert_eq!(frame.ppn.0, ppn_base.0 + i);
        QUEUE_FRAMES.exclusive_access().push(frame);
    }
    ppn_base.into()
}

#[no_mangle]
pub extern "C" fn virtio_dma_dealloc(pa: PhysAddr, pages: usize) -> i32 {
    let mut ppn_base: PhysPageNum = pa.into();
    for _ in 0..pages {
        frame_dealloc(ppn_base);
        ppn_base.step();
    }
    0
}

#[no_mangle]
pub extern "C" fn virtio_phys_to_virt(paddr: PhysAddr) -> VirtAddr {
    VirtAddr(paddr.0)
}

#[no_mangle]
pub extern "C" fn virtio_virt_to_phys(vaddr: VirtAddr) -> PhysAddr {
    PageTable::from_token(kernel_token())
        .translate_va(vaddr)
        .unwrap()
}
