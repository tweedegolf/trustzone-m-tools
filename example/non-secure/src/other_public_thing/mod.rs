use trustzone_m_macros::secure_callable;

static mut THING: u32 = 101;

#[secure_callable]
pub extern "C" fn write_public_thing(val: u32) {
    unsafe {
        THING = val;
    }
}

#[secure_callable]
pub extern "C" fn read_public_thing() -> u32 {
    unsafe { THING }
}
