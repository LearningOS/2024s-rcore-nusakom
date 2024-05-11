//! Process management syscalls
use crate::{
    timer::get_time_us,
    task::current_user_token,
};

// Implement a function to copy data from user space to kernel space
fn translated_struct_ptr<T>(token: usize, ptr: *mut T) -> Option<*mut T> {
    // Your implementation here
    // For example:
    // Some(kernel_ptr)
    // or None if ptr is NULL or translation fails
}

pub fn sys_get_time(_ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    
    // Check if _ts pointer is valid
    if _ts.is_null() {
        return -1;
    }

    let us = get_time_us();
    let ts = match translated_struct_ptr(current_user_token(), _ts) {
        Some(ptr) => ptr,
        None => return -1, // Translation failed
    };

    // Copy TimeVal struct from kernel space to user space
    unsafe {
        // Dereference ts and write TimeVal struct into user space
        (*ts).sec = us / 1_000_000;
        (*ts).usec = us % 1_000_000;
    }

    0
}
