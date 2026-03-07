#![no_std]

use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    core::arch::wasm32::unreachable()
}

#[no_mangle]
pub extern "C" fn add(a: i32, b: i32) -> i32 {
    a + b
}

#[no_mangle]
pub extern "C" fn fib(n: i32) -> i32 {
    let (mut a, mut b) = (0, 1);
    let mut i = 0;
    while i < n {
        let tmp = b;
        b = a + b;
        a = tmp;
        i += 1;
    }
    a
}
