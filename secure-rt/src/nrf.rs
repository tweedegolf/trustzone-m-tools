#[cfg(feature = "nrf5340")]
pub use nrf5340_app_pac::SPU_S as SPU;
#[cfg(feature = "nrf9160")]
pub use nrf9160_pac::SPU_S as SPU;

#[cfg(feature = "nrf5340")]
pub const FLASH_REGION_SIZE: u32 = 16 * 1024;
#[cfg(feature = "nrf9160")]
pub const FLASH_REGION_SIZE: u32 = 32 * 1024;

#[cfg(feature = "nrf5340")]
pub const RAM_REGION_SIZE: u32 = 8 * 1024;
#[cfg(feature = "nrf9160")]
pub const RAM_REGION_SIZE: u32 = 8 * 1024;

pub fn initialize<const PERIPHERALS_LEN: usize, const PINS_LEN: usize, const DPPI_LEN: usize>(
    nonsecure_peripherals: [NonSecurePeripheral; PERIPHERALS_LEN],
    nonsecure_pins: [(usize, u32); PINS_LEN],
    nonsecure_dppi: [(usize, u32); DPPI_LEN],
) {
    extern "C" {
        static _s_flash_start: u32;
        static _s_flash_end: u32;

        static _nsc_flash_start: u32;
        static _nsc_flash_end: u32;

        static _ns_flash_start: u32;
        static _ns_flash_end: u32;

        static _s_ram_start: u32;
        static _s_ram_end: u32;

        static _ns_ram_start: u32;
        static _ns_ram_end: u32;
    }

    let s_flash_start = unsafe { core::ptr::addr_of!(_s_flash_start) as u32 };
    let s_flash_end = unsafe { core::ptr::addr_of!(_s_flash_end) as u32 };
    let s_flash = s_flash_start..s_flash_end;

    let nsc_flash_start = unsafe { core::ptr::addr_of!(_nsc_flash_start) as u32 };
    let nsc_flash_end = unsafe { core::ptr::addr_of!(_nsc_flash_end) as u32 };
    let nsc_flash = nsc_flash_start..nsc_flash_end;
    #[cfg(feature = "memory_region_assertions")]
    assert_eq!(s_flash_start % FLASH_REGION_SIZE, 0, "The start of the flash region must be on a region boundary: val % {FLASH_REGION_SIZE:#X} must be 0");
    #[cfg(feature = "memory_region_assertions")]
    assert_eq!(nsc_flash_end % FLASH_REGION_SIZE, 0, "The end of the nsc_flash region must be on a region boundary: val % {FLASH_REGION_SIZE:#X} must be 0");

    let ns_flash_start = unsafe { core::ptr::addr_of!(_ns_flash_start) as u32 };
    let ns_flash_end = unsafe { core::ptr::addr_of!(_ns_flash_end) as u32 };
    let ns_flash = ns_flash_start..ns_flash_end;
    #[cfg(feature = "memory_region_assertions")]
    assert_eq!(ns_flash_start % FLASH_REGION_SIZE, 0, "The start of the ns flash region must be on a region boundary: val % {FLASH_REGION_SIZE:#X} must be 0");
    #[cfg(feature = "memory_region_assertions")]
    assert_eq!(ns_flash_end % FLASH_REGION_SIZE, 0, "The end of the ns flash region must be on a region boundary: val % {FLASH_REGION_SIZE:#X} must be 0");

    let s_ram_start = unsafe { core::ptr::addr_of!(_s_ram_start) as u32 };
    let s_ram_end = unsafe { core::ptr::addr_of!(_s_ram_end) as u32 }; 
    let s_ram = s_ram_start..s_ram_end;
    #[cfg(feature = "memory_region_assertions")]
    assert_eq!(s_ram_start % RAM_REGION_SIZE, 0, "The start of the ram region must be on a region boundary: val % {RAM_REGION_SIZE:#X} must be 0");
    #[cfg(feature = "memory_region_assertions")]
    assert_eq!(s_ram_end % RAM_REGION_SIZE, 0, "The end of the ram region must be on a region boundary: val % {RAM_REGION_SIZE:#X} must be 0");

    let ns_ram_start = unsafe { core::ptr::addr_of!(_ns_ram_start) as u32 };
    let ns_ram_end = unsafe { core::ptr::addr_of!(_ns_ram_end) as u32 }; 
    let ns_ram = ns_ram_start..ns_ram_end;
    #[cfg(feature = "memory_region_assertions")]
    assert_eq!(ns_ram_start % RAM_REGION_SIZE, 0, "The start of the ns ram region must be on a region boundary: val % {RAM_REGION_SIZE:#X} must be 0");
    #[cfg(feature = "memory_region_assertions")]
    assert_eq!(ns_ram_end % RAM_REGION_SIZE, 0, "The end of the ns ram region must be on a region boundary: val % {RAM_REGION_SIZE:#X} must be 0");

    let spu = unsafe { core::mem::transmute::<_, SPU>(()) };

    for (address, region) in spu
        .flashregion
        .iter()
        .enumerate()
        .map(|(index, region)| (index as u32 * FLASH_REGION_SIZE, region))
    {
        if s_flash.contains(&address) || nsc_flash.contains(&address) {
            region.perm.write(|w| {
                w.execute()
                    .enable()
                    .read()
                    .enable()
                    .write()
                    .enable()
                    .secattr()
                    .secure()
            });
        }
        else if ns_flash.contains(&address) {
            region.perm.write(|w| {
                w.execute()
                    .enable()
                    .read()
                    .enable()
                    .write()
                    .enable()
                    .secattr()
                    .non_secure()
            });
        }
    }

    set_nsc_region(&spu, nsc_flash_start..nsc_flash_end);

    for (address, region) in spu
        .ramregion
        .iter()
        .enumerate()
        .map(|(index, region)| (0x20000000 + index as u32 * RAM_REGION_SIZE, region))
    {
        if s_ram.contains(&address) {
            region.perm.write(|w| {
                w.execute()
                    .enable()
                    .read()
                    .enable()
                    .write()
                    .enable()
                    .secattr()
                    .secure()
            });
        }
        else if ns_ram.contains(&address) {
            region.perm.write(|w| {
                w.execute()
                    .enable()
                    .read()
                    .enable()
                    .write()
                    .enable()
                    .secattr()
                    .non_secure()
            });
        }
    }

    // Set all given peripherals to nonsecure
    for peripheral in nonsecure_peripherals {
        spu.periphid[peripheral.id]
            .perm
            .write(|w| w.secattr().non_secure().dmasec().non_secure());
    }

    // Set all given pins to nonsecure
    for (pin_port, pin) in nonsecure_pins {
        spu.gpioport[pin_port]
            .perm
            .modify(|r, w| unsafe { w.bits(r.bits() & !(1 << pin)) })
    }

    // Set all given dppi channels to nonsecure
    for (port, channel) in nonsecure_dppi {
        spu.dppi[port]
            .perm
            .modify(|r, w| unsafe { w.bits(r.bits() & !(1 << channel)) })
    }

    // We're using Nordic's SPU instead of the default SAU. To do that we must disable the SAU and
    // set the ALLNS (All Non-secure) bit.
    let sau = unsafe { core::mem::transmute::<_, cortex_m::peripheral::SAU>(()) };
    unsafe {
        sau.ctrl.modify(|mut ctrl| {
            ctrl.0 = 0b10;
            ctrl
        });

        // Also set the stack pointer of nonsecure
        cortex_m::register::msp::write_ns(ns_ram_end);
    }

    cortex_m::asm::isb();
    cortex_m::asm::dsb();

    unsafe {
        crate::initialize_ns_data();
    }
}

fn set_nsc_region(spu: &SPU, region: core::ops::Range<u32>) {
    let sg_start = region.start;
    let nsc_size = FLASH_REGION_SIZE - (sg_start % FLASH_REGION_SIZE);
    let size_reg = (31 - nsc_size.leading_zeros()) - 4;
    let region_reg = (sg_start as u32 / FLASH_REGION_SIZE) & 0x3F; // x << SPU_FLASHNSC_REGION_REGION_Pos & SPU_FLASHNSC_REGION_REGION_Msk
    spu.flashnsc[0].size.write(|w| {
        unsafe {
            w.bits(size_reg);
        }
        w
    });
    spu.flashnsc[0].region.write(|w| {
        unsafe {
            w.bits(region_reg);
        }
        w
    });
}

pub struct NonSecurePeripheral {
    id: usize,
}

macro_rules! impl_ns_peripheral {
    ($peripheral:ty, $id:expr) => {
        impl From<$peripheral> for NonSecurePeripheral {
            fn from(_: $peripheral) -> Self {
                Self { id: $id }
            }
        }
    };
}

#[cfg(feature = "nrf9160")]
mod nrf9160_peripheral_impl {
    use super::*;

    impl_ns_peripheral!(nrf9160_pac::REGULATORS_S, 4);
    impl_ns_peripheral!((nrf9160_pac::CLOCK_S, nrf9160_pac::POWER_S), 5);
    impl_ns_peripheral!(
        (
            nrf9160_pac::SPIM0_S,
            nrf9160_pac::SPIS0_S,
            nrf9160_pac::TWIM0_S,
            nrf9160_pac::TWIS0_S,
            nrf9160_pac::UARTE0_S
        ),
        8
    );
    impl_ns_peripheral!(
        (
            nrf9160_pac::SPIM1_S,
            nrf9160_pac::SPIS1_S,
            nrf9160_pac::TWIM1_S,
            nrf9160_pac::TWIS1_S,
            nrf9160_pac::UARTE1_S
        ),
        9
    );
    impl_ns_peripheral!(
        (
            nrf9160_pac::SPIM2_S,
            nrf9160_pac::SPIS2_S,
            nrf9160_pac::TWIM2_S,
            nrf9160_pac::TWIS2_S,
            nrf9160_pac::UARTE2_S
        ),
        10
    );
    impl_ns_peripheral!(
        (
            nrf9160_pac::SPIM3_S,
            nrf9160_pac::SPIS3_S,
            nrf9160_pac::TWIM3_S,
            nrf9160_pac::TWIS3_S,
            nrf9160_pac::UARTE3_S
        ),
        11
    );
    impl_ns_peripheral!(nrf9160_pac::SAADC_S, 14);
    impl_ns_peripheral!(nrf9160_pac::TIMER0_S, 15);
    impl_ns_peripheral!(nrf9160_pac::TIMER1_S, 16);
    impl_ns_peripheral!(nrf9160_pac::TIMER2_S, 17);
    impl_ns_peripheral!(nrf9160_pac::RTC0_S, 20);
    impl_ns_peripheral!(nrf9160_pac::RTC1_S, 21);
    impl_ns_peripheral!(&nrf9160_pac::DPPIC_S, 23);
    impl_ns_peripheral!(nrf9160_pac::WDT_S, 24);
    impl_ns_peripheral!(nrf9160_pac::EGU0_S, 27);
    impl_ns_peripheral!(nrf9160_pac::EGU1_S, 28);
    impl_ns_peripheral!(nrf9160_pac::EGU2_S, 29);
    impl_ns_peripheral!(nrf9160_pac::EGU3_S, 30);
    impl_ns_peripheral!(nrf9160_pac::EGU4_S, 31);
    impl_ns_peripheral!(nrf9160_pac::EGU5_S, 32);
    impl_ns_peripheral!(nrf9160_pac::PWM0_S, 34);
    impl_ns_peripheral!(nrf9160_pac::PWM1_S, 35);
    impl_ns_peripheral!(nrf9160_pac::PWM2_S, 36);
    impl_ns_peripheral!(nrf9160_pac::PDM_S, 38);
    impl_ns_peripheral!(nrf9160_pac::I2S_S, 40);
    impl_ns_peripheral!(nrf9160_pac::IPC_S, 42);
    impl_ns_peripheral!(nrf9160_pac::FPU_S, 44);
    impl_ns_peripheral!((&nrf9160_pac::KMU_S, &nrf9160_pac::NVMC_S), 57);
    impl_ns_peripheral!(nrf9160_pac::VMC_S, 58);
    impl_ns_peripheral!(&nrf9160_pac::P0_S, 66);
}
