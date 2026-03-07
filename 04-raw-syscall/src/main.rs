#![no_std]
#![no_main]

use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    syscall_exit(1);
}

fn syscall_write(fd: u64, buf: *const u8, len: u64) {
    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") 1u64,  // sys_write
            in("rdi") fd,
            in("rsi") buf,
            in("rdx") len,
            out("rcx") _,
            out("r11") _,
            lateout("rax") _,
        );
    }
}

fn syscall_exit(code: u64) -> ! {
    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") 60u64, // sys_exit
            in("rdi") code,
            options(noreturn),
        );
    }
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    let msg = b"Hello, tiny world!\n";
    syscall_write(1, msg.as_ptr(), msg.len() as u64);
    syscall_exit(0);
}
