#![no_std]

#[cfg(feature = "generic")]
use cortex_m::peripheral::sau::SauRegionAttribute;

#[cfg(feature = "_nrf")]
mod nrf;

#[cfg(feature = "generic")]
mod generic;

#[cfg(feature = "_nrf")]
pub use nrf::initialize;

#[cfg(feature = "generic")]
pub use generic::initialize;

#[cfg(not(any(feature = "_nrf", feature = "generic", not(target_arch = "arm"))))]
compile_error!("Select a trustzone runtime with the feature flags. Pick the feature of your chip or `generic`.");

#[cfg(feature = "generic")]
pub fn read_address_permissions(address: u32) -> SauRegionAttribute {
    let value = cortex_m::asm::tt(address as *mut u32);

    let s = value & (1 << 22) > 0;
    let nsrw = value & (1 << 21) > 0;

    match (s, nsrw) {
        (_, true) => SauRegionAttribute::NonSecureCallable,
        (true, false) => SauRegionAttribute::Secure,
        (false, false) => SauRegionAttribute::NonSecure,
    }
}
