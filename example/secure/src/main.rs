#![no_std]
#![no_main]
#![feature(abi_c_cmse_nonsecure_call)]
#![feature(cmse_nonsecure_entry)]

use core::panic::PanicInfo;
use cortex_m_rt::exception;
use rtt_target::rprintln;

include!(concat!(env!("OUT_DIR"), "/trustzone_bindings.rs"));

#[cortex_m_rt::entry]
fn main() -> ! {
    rtt_target::rtt_init_print!(BlockIfFull, 32);
    
    rprintln!("\nInit");
    trustzone_m_secure_rt::initialize();
    rprintln!("Done");

    rprintln!("Read call private: {}", trustzone_bindings::read_private_thing());
    rprintln!("Read call other public: {}", trustzone_bindings::read_public_thing());
    rprintln!("Read call: {}", trustzone_bindings::read_thing());
    
    rprintln!("Calling 'write_thing' with 5");
    trustzone_bindings::write_thing(5);
    rprintln!("Read call: {}", trustzone_bindings::read_thing());
    rprintln!("Calling 'write_thing' with 10");
    trustzone_bindings::write_thing(10);
    rprintln!("Read call: {}", trustzone_bindings::read_thing());

    loop {
        cortex_m::asm::bkpt();
    }
}

#[trustzone_m_macros::nonsecure_callable]
pub extern "C" fn return_5() -> u32 {
    rprintln!("In return_5");
    5
}

#[trustzone_m_macros::nonsecure_callable]
pub extern "C" fn double(x: u32) -> u32 {
    rprintln!("In double");
    x * 2
}

#[exception]
unsafe fn HardFault(frame: &cortex_m_rt::ExceptionFrame) -> ! {
    rprintln!("{:?}", frame);
    let sau = &*cortex_m::peripheral::SAU::PTR;
    rprintln!("Secure ctrl: {:X}", sau.ctrl.read().0);
    rprintln!("Secure fault status register: {:X}", sau.sfsr.read().0);
    rprintln!("Secure fault address register: {:X}", sau.sfar.read().0);

    let scb = &*cortex_m::peripheral::SCB::PTR;
    rprintln!("Configurable Fault Status Register: {:X}", scb.cfsr.read());

    cortex_m::asm::bkpt();
    cortex_m::asm::delay(u32::MAX);

    cortex_m::peripheral::SCB::sys_reset();
}

/// Called when our code panics.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    cortex_m::interrupt::disable();
    rprintln!("{}", info);
    cortex_m::asm::udf();
}
