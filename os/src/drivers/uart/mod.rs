pub mod ns1665a;
use alloc::sync::Arc;
use lazy_static::lazy_static;
pub use ns1665a::*;
lazy_static! {
    pub static ref UART_DEVICE:Arc<dyn UartDevice> = Arc::new(UART::new());
}