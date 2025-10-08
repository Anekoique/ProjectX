pub const IMG: [u32; 3] = [
    0x00000297, // auipc t0,0
    0x00100073, // ebreak (used as nemu_trap)
    0xdeadbeef, // some data
];
