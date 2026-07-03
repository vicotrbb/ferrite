const FNV64_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const FNV64_PRIME: u64 = 0x0000_0100_0000_01b3;

pub(crate) fn fnv64_bytes(bytes: &[u8]) -> u64 {
    bytes.iter().fold(FNV64_OFFSET_BASIS, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(FNV64_PRIME)
    })
}

pub(crate) fn format_fnv64(hash: u64) -> String {
    format!("fnv64:{hash:016x}")
}
