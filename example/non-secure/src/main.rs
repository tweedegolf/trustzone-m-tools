#![no_std]
#![no_main]

extern crate trustzone_m_nonsecure_rt;
use nrf9160_hal::{gpio, Uarte, uarte, prelude::OutputPin};
use nrf9160_pac::{P0_NS, UARTE0_NS};
use trustzone_m_macros::secure_callable;

mod other_private_thing;
pub mod other_public_thing;

include!(concat!(env!("OUT_DIR"), "/trustzone_bindings.rs"));

static mut THING: u32 = 99;

#[secure_callable]
pub extern "C" fn write_thing(val: u32) {
    unsafe {
        THING = trustzone_bindings::double(val + trustzone_bindings::return_5());
    }
}

#[secure_callable]
pub extern "C" fn read_thing() -> u32 {
    unsafe { THING }
}

#[secure_callable]
pub extern "C" fn blink_led_with_uart(data: [u8; 8]) {    
    let p0: P0_NS = unsafe { core::mem::transmute(()) };
    let p0 = gpio::p0::Parts::new(p0);
    let uarte0: UARTE0_NS = unsafe { core::mem::transmute(()) };
    
    let pins = uarte::Pins {
        txd: p0.p0_02.into_push_pull_output(gpio::Level::High).degrade(),
        rxd: p0.p0_00.into_floating_input().degrade(),
        cts: None,
        rts: None,
    };
    
    let mut uarte = Uarte::new(uarte0, pins, uarte::Parity::EXCLUDED, uarte::Baudrate::BAUD1200);
    
    uarte.write(&data).unwrap();
}

/// Called when our code panics.
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    cortex_m::asm::udf();
}
