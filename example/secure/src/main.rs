#![no_std]
#![no_main]
#![feature(abi_c_cmse_nonsecure_call)]
#![feature(cmse_nonsecure_entry)]
#![feature(type_alias_impl_trait)]

use core::{cell::RefCell, fmt::Write, panic::PanicInfo};
use cortex_m_rt::exception;
use embassy_executor::Spawner;
use embassy_nrf::{
    interrupt,
    peripherals::UARTETWISPI2,
    uarte::{Config, Uarte},
};
use embassy_sync::blocking_mutex::{raw::CriticalSectionRawMutex, Mutex};

include!(concat!(env!("OUT_DIR"), "/trustzone_bindings.rs"));

static UART_OUT: Mutex<CriticalSectionRawMutex, RefCell<Option<Uarte<'static, UARTETWISPI2>>>> =
    Mutex::new(RefCell::new(None));

struct Printer;
impl Write for Printer {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        UART_OUT.lock(|uart| {
            uart.borrow_mut()
                .as_mut()
                .unwrap()
                .blocking_write(s.as_bytes())
                .unwrap()
        });
        Ok(())
    }
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) -> ! {
    let ep = embassy_nrf::init(Default::default());
    let dp = unsafe { nrf9160_pac::Peripherals::steal() };

    let irq = interrupt::take!(UARTE2_SPIM2_SPIS2_TWIM2_TWIS2);
    let uart =
        embassy_nrf::uarte::Uarte::new(ep.UARTETWISPI2, irq, ep.P0_28, ep.P0_31, Config::default());
    UART_OUT.lock(|uarte| uarte.replace(Some(uart)));

    unsafe {
        (*cortex_m::peripheral::SCB::PTR)
            .shcsr
            .write((1 << 19) | (1 << 18) | (1 << 17) | (1 << 16))
    };

    writeln!(Printer, "\nInit").unwrap();

    trustzone_m_secure_rt::initialize(
        [
            (dp.SPIM0_S, dp.SPIS0_S, dp.TWIM0_S, dp.TWIS0_S, dp.UARTE0_S).into(),
            (&dp.P0_S).into(),
        ],
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
            (0, 16),
            (0, 17),
            (0, 18),
            (0, 19),
            (0, 20),
            (0, 21),
            (0, 22),
            (0, 23),
            (0, 24),
            (0, 25),
            (0, 26),
            (0, 27),
            // (0, 28),
            // (0, 29),
            (0, 30),
            // (0, 31),
        ],
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

    writeln!(Printer, "Done").unwrap();

    writeln!(Printer, 
        "Read call private: {}",
        trustzone_bindings::read_private_thing()
    ).unwrap();
    writeln!(Printer, 
        "Read call other public: {}",
        trustzone_bindings::read_public_thing()
    ).unwrap();
    writeln!(Printer, "Read call: {}", trustzone_bindings::read_thing()).unwrap();

    writeln!(Printer, "Calling 'write_thing' with 5").unwrap();
    trustzone_bindings::write_thing(5);
    writeln!(Printer, "Read call: {}", trustzone_bindings::read_thing()).unwrap();
    writeln!(Printer, "Calling 'write_thing' with 10").unwrap();
    trustzone_bindings::write_thing(10);
    writeln!(Printer, "Read call: {}", trustzone_bindings::read_thing()).unwrap();


    loop {
        trustzone_bindings::blink_led_with_uart([0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF]);
    }
}

#[trustzone_m_macros::nonsecure_callable]
pub extern "C" fn return_5() -> u32 {
    writeln!(Printer, "In return_5").unwrap();
    5
}

#[trustzone_m_macros::nonsecure_callable]
pub extern "C" fn double(x: u32) -> u32 {
    writeln!(Printer, "In double").unwrap();
    x * 2
}

#[exception]
unsafe fn HardFault(frame: &cortex_m_rt::ExceptionFrame) -> ! {
    writeln!(Printer, "{:?}", frame).unwrap();
    let sau = &*cortex_m::peripheral::SAU::PTR;
    writeln!(Printer, "Secure ctrl: {:X}", sau.ctrl.read().0).unwrap();
    writeln!(Printer, "Secure fault status register: {:X}", sau.sfsr.read().0).unwrap();
    writeln!(Printer, "Secure fault address register: {:X}", sau.sfar.read().0).unwrap();

    let scb = &*cortex_m::peripheral::SCB::PTR;
    writeln!(Printer, "Configurable Fault Status Register: {:X}", scb.cfsr.read()).unwrap();

    cortex_m::asm::delay(u32::MAX);

    cortex_m::peripheral::SCB::sys_reset();
}

#[exception]
unsafe fn DefaultHandler(irq: i16) -> ! {
    writeln!(Printer, "Default handler: {}", irq).unwrap();

    let sau = &*cortex_m::peripheral::SAU::PTR;
    writeln!(Printer, "Secure ctrl: {:X}", sau.ctrl.read().0).unwrap();
    writeln!(Printer, "Secure fault status register: {:X}", sau.sfsr.read().0).unwrap();
    writeln!(Printer, "Secure fault address register: {:X}", sau.sfar.read().0).unwrap();

    let scb = &*cortex_m::peripheral::SCB::PTR;
    writeln!(Printer, "Configurable Fault Status Register: {:X}", scb.cfsr.read()).unwrap();
    writeln!(Printer, "Bus Fault Address Register: {:X}", scb.bfar.read()).unwrap();

    cortex_m::asm::delay(u32::MAX);

    cortex_m::peripheral::SCB::sys_reset();
}

/// Called when our code panics.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    cortex_m::interrupt::disable();
    writeln!(Printer, "{}", info).unwrap();

    cortex_m::asm::udf();
}
