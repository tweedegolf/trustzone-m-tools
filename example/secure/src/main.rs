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
    let dp = nrf9160_pac::Peripherals::take().unwrap();

    unsafe { (*cortex_m::peripheral::SCB::PTR).shcsr.write((1 << 19) | (1 << 18) | (1 << 17) | (1 << 16)) };

    rtt_target::rtt_init_print!(BlockIfFull, 32);

    rprintln!("\nInit");
    trustzone_m_secure_rt::initialize(
        [
            (dp.SPIM0_S, dp.SPIS0_S, dp.TWIM0_S, dp.TWIS0_S, dp.UARTE0_S).into(),
            (&dp.P0_S).into(),
        ],
        [(0, 2), (0, 3)],
        [
            (0, 0),
            (0, 1),
            (0, 2),
            (0, 3),
            (0, 4),
            (0, 5),
            (0, 6),
            (0, 7),
            (0, 8),
            (0, 9),
            (0, 10),
            (0, 11),
            (0, 12),
            (0, 13),
            (0, 14),
            (0, 15),
        ],
    );

    let spu = unsafe { core::mem::transmute::<_, nrf9160_pac::SPU_S>(()) };

    rprintln!("{:X}", spu.gpioport[0].perm.read().bits());
    for (i, p) in spu.periphid.iter().enumerate() {
        
        rprintln!("{}: {}", i, p.perm.read().secattr().is_non_secure());
    }

    rprintln!("Done");

    rprintln!(
        "Read call private: {}",
        trustzone_bindings::read_private_thing()
    );
    rprintln!(
        "Read call other public: {}",
        trustzone_bindings::read_public_thing()
    );
    rprintln!("Read call: {}", trustzone_bindings::read_thing());

    rprintln!("Calling 'write_thing' with 5");
    trustzone_bindings::write_thing(5);
    rprintln!("Read call: {}", trustzone_bindings::read_thing());
    rprintln!("Calling 'write_thing' with 10");
    trustzone_bindings::write_thing(10);
    rprintln!("Read call: {}", trustzone_bindings::read_thing());

    trustzone_bindings::blink_led_with_uart([0, 1, 2, 3, 4, 5, 6, 7]);

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

#[exception]
unsafe fn DefaultHandler(irq: i16) -> ! {
    rprintln!("Default handler: {}", irq);

    let sau = &*cortex_m::peripheral::SAU::PTR;
    rprintln!("Secure ctrl: {:X}", sau.ctrl.read().0);
    rprintln!("Secure fault status register: {:X}", sau.sfsr.read().0);
    rprintln!("Secure fault address register: {:X}", sau.sfar.read().0);

    let scb = &*cortex_m::peripheral::SCB::PTR;
    rprintln!("Configurable Fault Status Register: {:X}", scb.cfsr.read());
    rprintln!("Bus Fault Address Register: {:X}", scb.bfar.read());

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
