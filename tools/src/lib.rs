pub mod secure_callable_macro;

pub const fn hash_vector_name(name: &str) -> u32 {
    crc::Crc::<u32>::new(&crc::CRC_32_CKSUM).checksum(name.as_bytes())
}

