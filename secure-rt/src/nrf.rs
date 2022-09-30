use rtt_target::rprintln;

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
    let nsc_flash = nsc_flash_start..nsc_flash_end;
    assert_eq!((nsc_flash_start - FLASH_REGION_SIZE + 4096) % FLASH_REGION_SIZE, 0);
    assert!(nsc_flash.clone().len() <= 4096);

    let nsc_ram_start = unsafe { core::mem::transmute::<_, u32>(&_nsc_ram_start) };
    let nsc_ram_end = unsafe { core::mem::transmute::<_, u32>(&_nsc_ram_end) };
    let nsc_ram = nsc_ram_start..nsc_ram_end;
    assert_eq!((nsc_ram_start - RAM_REGION_SIZE + 4096) % RAM_REGION_SIZE, 0);
    assert!(nsc_ram.clone().len() <= 4096);

    let ns_flash_start = unsafe { core::mem::transmute::<_, u32>(&_ns_flash_start) };
    let ns_flash_end = unsafe { core::mem::transmute::<_, u32>(&_ns_flash_end) };
    let ns_flash = ns_flash_start..ns_flash_end;
    assert_eq!(ns_flash_start % FLASH_REGION_SIZE, 0);
    assert_eq!(ns_flash_end % FLASH_REGION_SIZE, 0);

    let ns_ram_start = unsafe { core::mem::transmute::<_, u32>(&_ns_ram_start) };
    let ns_ram_end = unsafe { core::mem::transmute::<_, u32>(&_ns_ram_end) };
    let ns_ram = ns_ram_start..ns_ram_end;
    assert_eq!(ns_ram_start % RAM_REGION_SIZE, 0);
    assert_eq!(ns_ram_end % RAM_REGION_SIZE, 0);

    // We're gonna use Nordic's SPU instead of the default SAU. To do that we must disable the SAU and
    // set the ALLNS (All Non-secure) bit.
    let sau = unsafe { core::mem::transmute::<_, cortex_m::peripheral::SAU>(()) };
    unsafe {
        sau.ctrl.modify(|mut ctrl| {
            ctrl.0 = 0x00000002;
            ctrl
        });

        // Also set the stack pointer of nonsecure
        cortex_m::register::msp::write_ns(ns_ram_end);
    }

    let spu = unsafe { core::mem::transmute::<_, SPU>(()) };

    for (index, address, region) in spu
        .flashregion
        .iter()
        .enumerate()
        .map(|(index, region)| (index, index as u32 * FLASH_REGION_SIZE, region))
    {
        if nsc_flash.contains(&(address + FLASH_REGION_SIZE - 4096)) && !ns_flash.contains(&address) {
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

            spu.flashnsc[0]
                .region
                .write(|w| unsafe { w.region().bits(index as u8) });
            // It's really weird that we can only use 4096 bytes per region as NSC
            spu.flashnsc[0].size.write(|w| w.size()._4096());

            rprintln!("Flash {} @ {:X}..={:X} = NSC", index, address, address + FLASH_REGION_SIZE);
        } else if ns_flash.contains(&address) {
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
            rprintln!("Flash {} @ {:X}..={:X} = NS", index, address, address + FLASH_REGION_SIZE);
        } else {
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
            rprintln!("Flash {} @ {:X}..={:X} = S", index, address, address + FLASH_REGION_SIZE);
        }
    }

    for (index, address, region) in spu
        .ramregion
        .iter()
        .enumerate()
        .map(|(index, region)| (index, 0x20000000 + index as u32 * RAM_REGION_SIZE, region))
    {
        if nsc_ram.contains(&address) {
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

            spu.ramnsc[0]
                .region
                .write(|w| unsafe { w.region().bits(index as u8) });
            // It's really weird that we can only use 4096 bytes per region as NSC
            spu.ramnsc[0].size.write(|w| w.size()._4096());
        } else if ns_ram.contains(&address) {
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
        } else {
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
    }
}
