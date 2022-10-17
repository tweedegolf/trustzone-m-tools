use cortex_m::peripheral::sau::{SauError, SauRegion, SauRegionAttribute};

pub fn initialize() {
    extern "C" {
        static _nsc_flash_start: u32;
        static _nsc_flash_end: u32;
        static _nsc_ram_start: u32;
        static _nsc_ram_end: u32;

        static _ns_flash_start: u32;
        static _ns_flash_end: u32;
        static _ns_ram_start: u32;
        static _ns_ram_end: u32;
    }

    let nsc_flash_start = unsafe { core::mem::transmute::<_, u32>(&_nsc_flash_start) };
    let nsc_flash_end = unsafe { core::mem::transmute::<_, u32>(&_nsc_flash_end) };
    let nsc_ram_start = unsafe { core::mem::transmute::<_, u32>(&_nsc_ram_start) };
    let nsc_ram_end = unsafe { core::mem::transmute::<_, u32>(&_nsc_ram_end) };

    let ns_flash_start = unsafe { core::mem::transmute::<_, u32>(&_ns_flash_start) };
    let ns_flash_end = unsafe { core::mem::transmute::<_, u32>(&_ns_flash_end) };
    let ns_ram_start = unsafe { core::mem::transmute::<_, u32>(&_ns_ram_start) };
    let ns_ram_end = unsafe { core::mem::transmute::<_, u32>(&_ns_ram_end) };

    let mut sau = unsafe { core::mem::transmute::<_, cortex_m::peripheral::SAU>(()) };

    // Set nsc flash
    sau.set_region(
        0,
        SauRegion {
            base_address: nsc_flash_start,
            limit_address: nsc_flash_end - 1,
            attribute: SauRegionAttribute::NonSecureCallable,
        },
    ).unwrap();

    // Set nsc ram
    sau.set_region(
        1,
        SauRegion {
            base_address: nsc_ram_start,
            limit_address: nsc_ram_end - 1,
            attribute: SauRegionAttribute::NonSecureCallable,
        },
    ).unwrap();

    // Set ns flash
    sau.set_region(
        2,
        SauRegion {
            base_address: ns_flash_start,
            limit_address: ns_flash_end - 1,
            attribute: SauRegionAttribute::NonSecure,
        },
    ).unwrap();

    // Set ns ram
    sau.set_region(
        3,
        SauRegion {
            base_address: ns_ram_start,
            limit_address: ns_ram_end - 1,
            attribute: SauRegionAttribute::NonSecure,
        },
    ).unwrap();

    sau.enable();

    unsafe { crate::initialize_ns_data(); }
}
