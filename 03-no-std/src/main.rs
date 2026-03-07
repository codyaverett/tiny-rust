#![no_std]
#![no_main]

use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    unsafe {
        libc::exit(1);
    }
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    let msg = b"Hello, tiny world!\n";
    unsafe {
        libc::write(1, msg.as_ptr() as *const libc::c_void, msg.len());
        libc::exit(0);
    }
}
