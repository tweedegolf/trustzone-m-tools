use cortex_m::peripheral::sau::{SauError, SauRegion, SauRegionAttribute};

use crate::read_address_permissions;

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

    let mut current_region_number = 0;

    if !matches!(
        read_address_permissions(nsc_flash_start),
        SauRegionAttribute::NonSecureCallable
    ) {
        // Set nsc flash
        sau.set_region(
            current_region_number,
            SauRegion {
                base_address: nsc_flash_start,
                limit_address: nsc_flash_end - 1,
                attribute: SauRegionAttribute::NonSecureCallable,
            },
        ).unwrap();

        current_region_number += 1;
    }

    if !matches!(
        read_address_permissions(nsc_ram_start),
        SauRegionAttribute::NonSecureCallable
    ) {
        // Set nsc ram
        sau.set_region(
            current_region_number,
            SauRegion {
                base_address: nsc_ram_start,
                limit_address: nsc_ram_end - 1,
                attribute: SauRegionAttribute::NonSecureCallable,
            },
        ).unwrap();

        current_region_number += 1;
    }

    if !matches!(
        read_address_permissions(ns_flash_start),
        SauRegionAttribute::NonSecure
    ) {
        // Set ns flash
        sau.set_region(
            current_region_number,
            SauRegion {
                base_address: ns_flash_start,
                limit_address: ns_flash_end - 1,
                attribute: SauRegionAttribute::NonSecure,
            },
        ).unwrap();

        current_region_number += 1;
    }

    if !matches!(
        read_address_permissions(ns_ram_start),
        SauRegionAttribute::NonSecure
    ) {
        // Set ns ram
        sau.set_region(
            current_region_number,
            SauRegion {
                base_address: ns_ram_start,
                limit_address: ns_ram_end - 1,
                attribute: SauRegionAttribute::NonSecure,
            },
        ).unwrap();

        current_region_number += 1;
    }

    debug_assert!(current_region_number <= sau.region_numbers());

    sau.enable();
}
