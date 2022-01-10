use core::{
    any::Any,
    convert::TryInto,
    fmt::{Error, Write},
};

use alloc::collections::VecDeque;

use crate::{
    drivers::{IRQ_TASKS, complete},
    sync::UPSafeCell,
    task::{awake_by_irq_and_run, wait_irq_and_run_next}
};

pub trait UartDevice: Send + Sync + Any {
    fn put(&self, c: u8);
    fn get(&self) -> Option<u8>;
    fn append(&self);
    fn handler_interrupt(&self);
}

pub struct UART(UPSafeCell<Ns1665a>);

impl UART {
    pub fn new() -> Self {
        IRQ_TASKS.init_queue(10);
        Self(unsafe {
            let mut uart = Ns1665a::new();
            uart.init();
            UPSafeCell::new(uart)
        })
    }
}

impl UartDevice for UART {
    fn put(&self, c: u8) {
        self.0.exclusive_access().put(c)
    }

    fn get(&self) -> Option<u8> {
        self.0.exclusive_access().get()
    }

    fn handler_interrupt(&self) {
        awake_by_irq_and_run(10);
    }

    fn append(&self) {
        self.0.exclusive_access().append_char();
    }
}

pub struct Ns1665a {
    base_address: usize,
    buffer: VecDeque<u8>,
}

impl Write for Ns1665a {
    fn write_str(&mut self, out: &str) -> Result<(), Error> {
        for c in out.bytes() {
            self.put(c);
        }
        Ok(())
    }
}

impl Ns1665a {
    pub fn new() -> Self {
        let mut ns = Self {
            base_address: 0x10000000,
            buffer: VecDeque::new(),
        };
        ns.init();
        ns
    }

    pub fn init(&mut self) {
        let ptr = self.base_address as *mut u8;
        unsafe {
            let lcr: u8 = (1 << 0) | (1 << 1);
            ptr.add(3).write_volatile(lcr);
            ptr.add(2).write_volatile(1 << 0);
            ptr.add(1).write_volatile(1 << 0);
            let divisor: u16 = 592;
            let divisor_least: u8 = (divisor & 0xff).try_into().unwrap();
            let divisor_most: u8 = (divisor >> 8).try_into().unwrap();
            ptr.add(3).write_volatile(lcr | 1 << 7);
            ptr.add(0).write_volatile(divisor_least);
            ptr.add(1).write_volatile(divisor_most);
            ptr.add(3).write_volatile(lcr);
        }
    }

    pub fn append_char(&mut self) {
        let ptr = self.base_address as *mut u8;
        unsafe {
            if ptr.add(5).read_volatile() & 1 != 0 {
                self.buffer.push_back(ptr.add(0).read_volatile());
                complete(10);
            }
        }
    }

    pub fn put(&mut self, c: u8) {
        let ptr = self.base_address as *mut u8;
        unsafe {
            ptr.add(0).write_volatile(c);
        }
    }

    pub fn get(&mut self) -> Option<u8> {
        if self.buffer.len() > 0 {
            self.buffer.pop_front()
        } else {
            wait_irq_and_run_next(10);
            let ptr = self.base_address as *mut u8;
            unsafe {
                if ptr.add(5).read_volatile() & 1 == 0 {
                    None
                } else {
                    Some(ptr.add(0).read_volatile())
                }
            }
        }
    }
}
