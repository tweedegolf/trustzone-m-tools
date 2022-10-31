use trustzone_m_macros::secure_callable;

static mut THING: u32 = 102;

#[secure_callable]
pub extern "C" fn write_private_thing(val: u32) {
    unsafe {
        THING = val;
    }
}

#[secure_callable]
pub extern "C" fn read_private_thing() -> u32 {
    unsafe { THING }
}
