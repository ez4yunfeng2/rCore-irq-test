use alloc::{collections::{BTreeMap, VecDeque}, sync::Arc};
use riscv::register::sie;
use crate::{task::TaskControlBlock, sync::UPSafeCell};
use lazy_static::lazy_static;

const PLIC_PRIORITY: usize = 0x0c00_0000;
const PLIC_INT_ENABLE: usize = 0x0c00_2080;
const PLIC_THRESHOLD: usize = 0x0c20_1000;
const PLIC_CLAIM: usize = 0x0c20_1004;

lazy_static! {
    pub static ref IRQ_TASKS:Arc<IrqWait> = Arc::new(IrqWait::new());
}
pub struct IrqWait(UPSafeCell<BTreeMap<usize,VecDeque<Arc<TaskControlBlock>>>>);
impl IrqWait{
    pub fn new() -> Self {
        Self(unsafe{UPSafeCell::new(BTreeMap::new())})
    }
    pub fn init_queue(&self, irq:usize) {
        self.0.exclusive_access().insert(irq, VecDeque::new());
    }
    pub fn add_irq_task(&self,key:usize,task: Arc<TaskControlBlock>) {
        if let Some(queue) = self.0.exclusive_access().get_mut(&key){
            queue.push_back(task);
        }
    }
    pub fn fetch_irq_task(&self, key:usize,) -> Option<Arc<TaskControlBlock>>{
        if let Some(queue) = self.0.exclusive_access().get_mut(&key){
            queue.pop_front()
        } else {
            None
        }
    }
}
pub fn next() -> Option<u32> {
    let claim_reg = PLIC_CLAIM as *const u32;
    let claim_no;
    unsafe {
        claim_no = claim_reg.read_volatile();
    }
    if claim_no == 0 {
        None
    } else {
        Some(claim_no)
    }
}

pub fn complete(id: u32) {
    let complete_reg = PLIC_CLAIM as *mut u32;
    unsafe {
        complete_reg.write_volatile(id);
    }
}

fn set_threshold(tsh: u8) {
    let actual_tsh = tsh & 7;
    let tsh_reg = PLIC_THRESHOLD as *mut u32;
    unsafe {
        tsh_reg.write_volatile(actual_tsh as u32);
    }
}

fn enable(id: u32) {
    let enables = PLIC_INT_ENABLE as *mut u32;
    let actual_id = 1 << id;
    unsafe {
        enables.write_volatile(enables.read_volatile() | actual_id);
    }
}

fn set_priority(id: u32, prio: u8) {
    let actual_prio = prio as u32 & 7;
    let prio_reg = PLIC_PRIORITY as *mut u32;
    unsafe {
        prio_reg.add(id as usize).write_volatile(actual_prio);
    }
}

pub fn plic_init(){
    unsafe{ sie::set_sext() }
    set_threshold(0);
    for i in [8,10]{
        enable(i);
        set_priority(i, 1 as u8);
    }
}