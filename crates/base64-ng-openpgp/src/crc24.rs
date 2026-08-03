pub(crate) const CRC24_INIT: u32 = 0xB7_04_CE;
const CRC24_POLY: u32 = 0x86_4C_FB;

pub(crate) fn crc24(bytes: &[u8]) -> u32 {
    let mut crc = CRC24_INIT;
    for byte in bytes {
        crc ^= u32::from(*byte) << 16;
        for _ in 0..8 {
            crc <<= 1;
            if crc & 0x01_00_00_00 != 0 {
                crc ^= CRC24_POLY;
            }
        }
    }
    crc & 0x00_FF_FF_FF
}

pub(crate) fn bytes(crc: u32) -> [u8; 3] {
    [
        ((crc >> 16) & 0xff) as u8,
        ((crc >> 8) & 0xff) as u8,
        (crc & 0xff) as u8,
    ]
}

#[cfg(test)]
mod tests {
    #[test]
    fn standard_check_value_matches_rfc_code() {
        assert_eq!(super::crc24(b"123456789"), 0x21_CF_02);
    }
}
