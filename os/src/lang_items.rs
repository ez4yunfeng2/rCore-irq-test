use crate::sbi::{shutdown, console_putchar};
use crate::task::current_kstack_top;
use core::fmt::{Write, Arguments};
use core::panic::PanicInfo;
use core::sync::atomic::{AtomicBool, Ordering};
struct Stderr;

impl Write for Stderr {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for ch in s.chars() {
            console_putchar(ch as usize)
        }
        Ok(())
    }
}

fn print(args: Arguments) {
    Stderr.write_fmt(args).unwrap();
}

#[macro_export]
macro_rules! debug_println {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::lang_items::print(format_args!(concat!($fmt, "\n") $(, $($arg)+)?))
    }
}

static mut PANICLOCK: AtomicBool = AtomicBool::new(false);

#[panic_handler]
unsafe fn panic(info: &PanicInfo) -> ! {
    riscv::register::sstatus::clear_sie();
    while PANICLOCK.compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed).is_err() {
        while PANICLOCK.load(Ordering::Relaxed) {}
    }
    match info.location() {
        Some(location) => {
            println!(
                "[kernel] panicked at '{}', {}:{}:{}",
                info.message().unwrap(),
                location.file(),
                location.line(),
                location.column()
            );
        }
        None => println!("[kernel] panicked at '{}'", info.message().unwrap()),
    }
    PANICLOCK.store(false, Ordering::Release);
    loop {}
    // unsafe {
    //     backtrace();
    // }
    // shutdown()
}

#[allow(unused)]
unsafe fn backtrace() {
    let mut fp: usize;
    let stop = current_kstack_top();
    asm!("mv {}, s0", out(reg) fp);
    println!("---START BACKTRACE---");
    for i in 0..10 {
        if fp == stop {
            break;
        }
        println!("#{}:ra={:#x}", i, *((fp - 8) as *const usize));
        fp = *((fp - 16) as *const usize);
    }
    println!("---END   BACKTRACE---");
}
