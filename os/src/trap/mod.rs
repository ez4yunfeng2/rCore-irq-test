mod context;

pub use context::TrapContext;
use core::arch::global_asm;
use core::arch::asm;
use crate::config::TRAMPOLINE;
use crate::drivers::{KEYBOARD_DEVICE, MOUSE_DEVICE};
use crate::task::{
    current_trap_cx, current_trap_cx_user_va, current_user_token, exit_current_and_run_next,
    suspend_current_and_run_next,
};
use crate::timer::{check_timer, set_next_trigger};
use crate::{
    drivers::{complete, next, BLOCK_DEVICE, UART_DEVICE},
    syscall::syscall,
    task::IRQ_FLAG,
};
use riscv::register::{
    mtvec::TrapMode,
    scause::{self, Exception, Interrupt, Trap},
    sie, stval, stvec,
};

global_asm!(include_str!("trap.S"));

pub fn init() {
    set_kernel_trap_entry();
}

fn set_kernel_trap_entry() {
    extern "C" {
        fn __from_kernel_save();
    }
    unsafe {
        stvec::write(__from_kernel_save as usize, TrapMode::Direct);
    }
}

fn set_user_trap_entry() {
    unsafe {
        stvec::write(TRAMPOLINE as usize, TrapMode::Direct);
    }
}

pub fn enable_timer_interrupt() {
    unsafe {
        sie::set_stimer();
    }
}

#[no_mangle]
pub fn trap_handler() -> ! {
    set_kernel_trap_entry();
    let scause = scause::read();
    let stval = stval::read();
    match scause.cause() {
        Trap::Exception(Exception::UserEnvCall) => {
            // jump to next instruction anyway
            let mut cx = current_trap_cx();
            cx.sepc += 4;
            // get system call return value
            let result = syscall(cx.x[17], [cx.x[10], cx.x[11], cx.x[12]]);
            // cx is changed during sys_exec, so we have to call it again
            cx = current_trap_cx();
            cx.x[10] = result as usize;
        }
        Trap::Exception(Exception::StoreFault)
        | Trap::Exception(Exception::StorePageFault)
        | Trap::Exception(Exception::InstructionFault)
        | Trap::Exception(Exception::InstructionPageFault)
        | Trap::Exception(Exception::LoadFault)
        | Trap::Exception(Exception::LoadPageFault) => {
            println!(
                "[kernel] {:?} in application, bad addr = {:#x}, bad instruction = {:#x}, core dumped.",
                scause.cause(),
                stval,
                current_trap_cx().sepc,
            );
            // page fault exit code
            exit_current_and_run_next(-2);
        }
        Trap::Exception(Exception::IllegalInstruction) => {
            println!("[kernel] IllegalInstruction in application, core dumped.");
            // illegal instruction exit code
            exit_current_and_run_next(-3);
        }
        Trap::Interrupt(Interrupt::SupervisorTimer) => {
            set_next_trigger();
            check_timer();
            suspend_current_and_run_next();
        }
        Trap::Interrupt(Interrupt::SupervisorExternal) => {
            if let Some(irq) = next() {
                println!("irq {}",irq);
                match irq {
                    10 => {
                        UART_DEVICE.handler_interrupt();
                    }
                    8 => BLOCK_DEVICE.handler_interrupt(),
                    6 => {
                        complete(irq);
                        MOUSE_DEVICE.handler_interrupt();
                    }
                    5 => {
                        complete(irq);
                        KEYBOARD_DEVICE.handler_interrupt();                            
                    }
                    _ => {
                        panic!("unknow irq");
                    }
                }
            }
        }
        _ => {
            panic!(
                "Unsupported trap {:?}, stval = {:#x}!",
                scause.cause(),
                stval
            );
        }
    }
    trap_return();
}

#[no_mangle]
pub fn trap_return() -> ! {
    set_user_trap_entry();
    let trap_cx_user_va = current_trap_cx_user_va();
    let user_satp = current_user_token();
    extern "C" {
        fn __alltraps();
        fn __restore();
    }
    let restore_va = __restore as usize - __alltraps as usize + TRAMPOLINE;
    unsafe {
        asm!(
            "fence.i",
            "jr {restore_va}",
            restore_va = in(reg) restore_va,
            in("a0") trap_cx_user_va,
            in("a1") user_satp,
            options(noreturn)
        );
    }
}

#[no_mangle]
pub fn trap_from_kernel() {
    let scause = scause::read();
    match scause.cause() {
        Trap::Interrupt(Interrupt::SupervisorTimer) => {
            set_next_trigger();
        }
        Trap::Interrupt(Interrupt::SupervisorExternal) => {
            if let Some(irq) = next() {
                match irq {
                    10 => {
                        UART_DEVICE.append();
                    }
                    8 => {
                        complete(irq);
                        *IRQ_FLAG.exclusive_access() = true;
                    }
                    6 => {
                        complete(irq);
                        MOUSE_DEVICE.handler_interrupt();
                    }
                    5 => {
                        complete(irq);
                        KEYBOARD_DEVICE.handler_interrupt();                            
                    }
                    _ => {
                        panic!("Unsupported irq")
                    }
                }
            }
        }
        _ => {
            panic!("error trap from kernel {:?}", scause.cause())
        }
    }
}

