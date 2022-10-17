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

extern "C" {
    pub(crate) fn initialize_ns_data();
}
