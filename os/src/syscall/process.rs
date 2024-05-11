//! Process management syscalls
use core::{mem::size_of, slice::from_raw_parts};

use crate::{
    config::{CLOCK_FREQ, MAX_SYSCALL_NUM},
    mm::{translated_byte_buffer, MapPermission},
    task::{
        change_program_brk, current_start_time, current_syscall_times, current_user_token,
        exit_current_and_run_next, map_current_task, suspend_current_and_run_next,
        unmap_current_task, TaskStatus,
    },
    timer::{get_time, get_time_us},
};

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// Task information
#[allow(dead_code)]
pub struct TaskInfo {
    /// Task status in it's life cycle
    status: TaskStatus,
    /// The numbers of syscall called by task
    syscall_times: [u32; MAX_SYSCALL_NUM],
    /// Total running time of task
    time: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(_exit_code: i32) -> ! {
    trace!("kernel: sys_exit");
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(_ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    let us = get_time_us();
    let tv = TimeVal {
        sec: us / 1_000_000,
        usec: us % 1_000_000,
    };
    let len = size_of::<TimeVal>();
    let _ts = translated_byte_buffer(current_user_token(), _ts as usize as *const u8, len);
    if let Ok(_ts) = _ts {
        let tv_ptr = &tv as *const TimeVal as *const u8;
        for i in _ts {
            let src = unsafe { from_raw_parts(tv_ptr, i.len()) };
            i.copy_from_slice(src);
            unsafe {
                let _ = tv_ptr.add(i.len());
            }
        }
        0
    } else {
        -1
    }
}

/// YOUR JOB: Finish sys_task_info to pass testcases
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TaskInfo`] is splitted by two pages ?
pub fn sys_task_info(_ti: *mut TaskInfo) -> isize {
    trace!("kernel: sys_task_info NOT IMPLEMENTED YET!");
    let ti = TaskInfo {
        status: TaskStatus::Running,
        syscall_times: current_syscall_times(),
        time: (get_time() - current_start_time()) / (CLOCK_FREQ / 1000),
    };
    let len = size_of::<TaskInfo>();
    let _ti = translated_byte_buffer(current_user_token(), _ti as usize as *const u8, len);
    if let Ok(_ti) = _ti {
        let ti_ptr = &ti as *const TaskInfo as *const u8;
        for i in _ti {
            let src = unsafe { from_raw_parts(ti_ptr, i.len()) };
            i.copy_from_slice(src);
            unsafe {
                let _ = ti_ptr.add(i.len());
            }
        }
        0
    } else {
        -1
    }
}

// YOUR JOB: Implement mmap.
pub fn sys_mmap(_start: usize, _len: usize, _port: usize) -> isize {
    trace!("kernel: sys_mmap NOT IMPLEMENTED YET!");
    let mut perm = MapPermission::U;
    if _start & 0xfff != 0 {
        return -1;
    }
    if _port & 0b111 == 0 || _port & !0b111 != 0 {
        return -1;
    }
    if _port & 0b1 != 0 {
        perm = perm | MapPermission::R;
    }
    if _port & 0b10 != 0 {
        perm = perm | MapPermission::W;
    }
    if _port & 0b100 != 0 {
        perm = perm | MapPermission::X;
    }
    let start = _start.into();
    let end = (_start + _len).into();

    if let Ok(()) = map_current_task(start, end, perm) {
        0
    } else {
        -1
    }
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(_start: usize, _len: usize) -> isize {
    trace!("kernel: sys_munmap NOT IMPLEMENTED YET!");
    let start = _start.into();
    let end = (_start + _len).into();
    if _start & 0xfff != 0 {
        return -1;
    }
    if let Ok(()) = unmap_current_task(start, end) {
        0
    } else {
        -1
    }
}
/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel: sys_sbrk");
    if let Some(old_brk) = change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}