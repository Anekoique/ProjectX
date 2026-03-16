#![no_std]

extern crate xhal;

unsafe extern "C" {
    fn halt(code: i32) -> !;
}

fn check(cond: bool) {
    if !cond {
        unsafe { halt(1) }
    }
}

const N: usize = 10;

#[unsafe(no_mangle)]
pub extern "C" fn main() -> i32 {
    let mut a = [[0i32; N]; N];
    let mut b = [[0i32; N]; N];

    // Initialize: a[i][j] = i + j, b[i][j] = i * j
    for i in 0..N {
        for j in 0..N {
            a[i][j] = (i + j) as i32;
            b[i][j] = (i * j) as i32;
        }
    }

    // c = a * b
    let mut c = [[0i32; N]; N];
    for i in 0..N {
        for j in 0..N {
            for k in 0..N {
                c[i][j] = c[i][j].wrapping_add(a[i][k].wrapping_mul(b[k][j]));
            }
        }
    }

    // Recompute expected and check
    for i in 0..N {
        for j in 0..N {
            let mut expected = 0i32;
            for k in 0..N {
                expected = expected.wrapping_add(((i + k) as i32).wrapping_mul((k * j) as i32));
            }
            check(c[i][j] == expected);
        }
    }
    0
}
