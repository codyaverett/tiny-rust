#![no_std]
#![no_main]

use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    unsafe {
        libc::exit(1);
    }
}

#[inline(never)]
fn run() {
    let msg = b"y\n";
    unsafe {
        loop {
            let ret = libc::write(1, msg.as_ptr() as *const libc::c_void, msg.len());
            if ret < 0 {
                libc::exit(0);
            }
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn _start() -> ! {
    core::arch::asm!(
        "and rsp, -16",
        "call {run}",
        run = sym run,
        options(noreturn),
    );
}
