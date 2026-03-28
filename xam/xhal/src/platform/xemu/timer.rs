const ACLINT: usize = 0x0200_0000;
const MTIME_LO: *const u32 = (ACLINT + 0xBFF8) as _;
const MTIME_HI: *const u32 = (ACLINT + 0xBFFC) as _;
const MTIMECMP_LO: *mut u32 = (ACLINT + 0x4000) as _;
const MTIMECMP_HI: *mut u32 = (ACLINT + 0x4004) as _;

/// Read 64-bit mtime using split lo/hi access (works on both RV32/RV64).
#[unsafe(no_mangle)]
pub extern "C" fn mtime() -> u64 {
    unsafe {
        loop {
            let hi1 = MTIME_HI.read_volatile();
            let lo = MTIME_LO.read_volatile();
            let hi2 = MTIME_HI.read_volatile();
            if hi1 == hi2 {
                return ((hi1 as u64) << 32) | lo as u64;
            }
        }
    }
}

/// Write 64-bit mtimecmp using split lo/hi access.
/// Sets hi to MAX first to prevent spurious timer fire during partial write.
#[unsafe(no_mangle)]
pub extern "C" fn set_mtimecmp(val: u64) {
    unsafe {
        MTIMECMP_HI.write_volatile(0xFFFF_FFFF);
        MTIMECMP_LO.write_volatile(val as u32);
        MTIMECMP_HI.write_volatile((val >> 32) as u32);
    }
}
