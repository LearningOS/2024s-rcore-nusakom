//! Process management syscalls

//!
use alloc::sync::Arc;

use crate::{
    config::{MAX_SYSCALL_NUM,PAGE_SIZE},
    fs::{open_file, OpenFlags},
    mm::{translated_refmut, translated_str,translated_byte_buffer,MapPermission},
    task::{
        add_task, current_task, current_user_token, exit_current_and_run_next,
        suspend_current_and_run_next, TaskStatus,
    },
    timer::{get_time_ms,get_time_us}
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
impl TaskInfo {
    /// 
    pub fn new()->Self{
        let syscall_times=[0u32; MAX_SYSCALL_NUM];
        Self { status: TaskStatus::UnInit, syscall_times: syscall_times, time: 0 }
    }
    ///
    pub fn call(&mut self,syscall:usize){
        self.syscall_times[syscall]+=1;
    }
    /// 
    pub fn init_time(&mut self)->&mut Self{
        if self.time==0{
            self.time=get_time_ms();
        }
        self
    }
    /// 
    pub fn set_time(&mut self,time:usize)->&mut Self{
        self.time=time;
        self
    }
    ///
    pub fn get_time(&self)->usize{
        self.time
    }
    /// 
    pub fn flush_status(&mut self,task_status:TaskStatus)->&mut Self{
        self.status=task_status;
        self
    }
}

pub fn sys_exit(exit_code: i32) -> ! {
    trace!("kernel:pid[{}] sys_exit", current_task().unwrap().pid.0);
    exit_current_and_run_next(exit_code);
    panic!("Unreachable in sys_exit!");
}

pub fn sys_yield() -> isize {
    //trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

pub fn sys_getpid() -> isize {
    trace!("kernel: sys_getpid pid:{}", current_task().unwrap().pid.0);
    current_task().unwrap().pid.0 as isize
}

pub fn sys_fork() -> isize {
    trace!("kernel:pid[{}] sys_fork", current_task().unwrap().pid.0);
    let current_task = current_task().unwrap();
    let new_task = current_task.fork();
    let new_pid = new_task.pid.0;
    // modify trap context of new_task, because it returns immediately after switching
    let trap_cx = new_task.inner_exclusive_access().get_trap_cx();
    // we do not have to move to next instruction since we have done it before
    // for child process, fork returns 0
    trap_cx.x[10] = 0;
    // add new task to scheduler
    add_task(new_task);
    new_pid as isize
}

pub fn sys_exec(path: *const u8) -> isize {
    trace!("kernel:pid[{}] sys_exec", current_task().unwrap().pid.0);
    let token = current_user_token();
    let path = translated_str(token, path);
    if let Some(app_inode) = open_file(path.as_str(), OpenFlags::RDONLY) {
        let all_data = app_inode.read_all();
        let task = current_task().unwrap();
        task.exec(all_data.as_slice());
        0
    } else {
        -1
    }
}

/// If there is not a child process whose pid is same as given, return -1.
/// Else if there is a child process but it is still running, return -2.
pub fn sys_waitpid(pid: isize, exit_code_ptr: *mut i32) -> isize {
    //trace!("kernel: sys_waitpid");
    let task = current_task().unwrap();
    // find a child process

    // ---- access current PCB exclusively
    let mut inner = task.inner_exclusive_access();
    if !inner
        .children
        .iter()
        .any(|p| pid == -1 || pid as usize == p.getpid())
    {
        return -1;
        // ---- release current PCB
    }
    let pair = inner.children.iter().enumerate().find(|(_, p)| {
        // ++++ temporarily access child PCB exclusively
        p.inner_exclusive_access().is_zombie() && (pid == -1 || pid as usize == p.getpid())
        // ++++ release child PCB
    });
    if let Some((idx, _)) = pair {
        let child = inner.children.remove(idx);
        // confirm that child will be deallocated after being removed from children list
        assert_eq!(Arc::strong_count(&child), 1);
        let found_pid = child.getpid();
        // ++++ temporarily access child PCB exclusively
        let exit_code = child.inner_exclusive_access().exit_code;
        // ++++ release child PCB
        *translated_refmut(inner.memory_set.token(), exit_code_ptr) = exit_code;
        found_pid as isize
    } else {
        -2
    }
    // ---- release current PCB automatically
}

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(_ts: *mut TimeVal, _tz: usize) -> isize {
    trace!(
        "kernel:pid[{}] sys_get_time NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    //maybe uncontinuous  
    let us=get_time_us();
    let time_val=TimeVal{
        sec:us/1_000_000,
        usec:us%1_000_000
    };
    // maybe uncontinuous  
    let user_time_val_buffer=translated_byte_buffer(current_user_token(), _ts as *const u8, core::mem::size_of::<TimeVal>());
    unsafe{
        // get timeval bytes 
        let time_val_bytes=core::slice::from_raw_parts((&time_val as *const TimeVal)as *const u8 , core::mem::size_of::<TimeVal>());    
        // copy timeval to user space
        let mut offset = 0;
        for bytes in user_time_val_buffer{
            bytes.copy_from_slice(&time_val_bytes[offset..offset+bytes.len()]);
            offset+=bytes.len();
        }
    }
    0
    // -1
}

/// YOUR JOB: Finish sys_task_info to pass testcases
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TaskInfo`] is splitted by two pages ?
pub fn sys_task_info(_ti: *mut TaskInfo) -> isize {
    trace!(
        "kernel:pid[{}] sys_task_info NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    // taskinfo maybe uncontinuous
    // taskinfo bytes 
    let user_task_info_buffer=translated_byte_buffer(current_user_token(), _ti as *const u8, core::mem::size_of::<TaskInfo>());
    unsafe{
        // get taskinfo 
        let current_task=current_task().unwrap();
        let mut task_inner=current_task.inner_exclusive_access();
        let current_task_info=task_inner.get_info();
        // set info
        let origin_time=current_task_info.get_time();
        current_task_info.set_time(get_time_ms()-origin_time);
        // get task info bytes
        let task_info_bytes=core::slice::from_raw_parts((current_task_info as *const TaskInfo)as * const u8 , core::mem::size_of::<TaskInfo>());
        // copy to user space
        let mut offset=0;
        for bytes in user_task_info_buffer{
            bytes.copy_from_slice(&task_info_bytes[offset..offset+bytes.len()]);
            offset+=bytes.len();
        }
        current_task_info.set_time(origin_time);
    }
    0   
}

/// YOUR JOB: Implement mmap.
pub fn sys_mmap(_start: usize, _len: usize, _port: usize) -> isize {
    trace!(
        "kernel:pid[{}] sys_mmap NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    if _port & 0x7 ==0 || _port & !0x7 != 0 || _start % PAGE_SIZE!=0 {
        return -1
    }
    let mut permission=MapPermission::U;
    if _port & 0x1 ==1 {
        permission|=MapPermission::R;
    }
    if _port & 0x2 ==0x2 {
        permission|=MapPermission::W;
    }
    if _port & 0x4 ==0x4{
        permission|=MapPermission::X;
    }
     // ceil(4096)=4096 
    current_task().unwrap().inner_exclusive_access().memory_set.mmap(_start.into(), (_start+_len).into(), permission)

}

/// YOUR JOB: Implement munmap.
pub fn sys_munmap(_start: usize, _len: usize) -> isize {
    trace!(
        "kernel:pid[{}] sys_munmap NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    if _start % PAGE_SIZE!=0 || _len % PAGE_SIZE!=0{
        return -1
    }
    current_task().unwrap().inner_exclusive_access().memory_set.unmap(_start.into(), (_start+_len).into());
    0
}

/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel:pid[{}] sys_sbrk", current_task().unwrap().pid.0);
    if let Some(old_brk) = current_task().unwrap().change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}

/// YOUR JOB: Implement spawn.
/// HINT: fork + exec =/= spawn
pub fn sys_spawn(_path: *const u8) -> isize {
    trace!(
        "kernel:pid[{}] sys_spawn NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    let token =current_user_token();
    let path=translated_str(token, _path);
    if let Some(data_inode)=open_file(&path.as_str(), OpenFlags::RDONLY){
        let current_task=current_task().unwrap();
        let new_task=current_task.spawn(data_inode.read_all().as_slice());
        let new_pid = new_task.pid.0;
        add_task(new_task);
        return new_pid as isize;
    }
    // let data_inode = open_file(&path.as_str(), OpenFlags::RDONLY);
    -1
}

// YOUR JOB: Set task priority.
pub fn sys_set_priority(_prio: isize) -> isize {
    trace!(
        "kernel:pid[{}] sys_set_priority NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    if _prio<=1{
        return -1;
    }
    current_task().unwrap().inner_exclusive_access().set_proority(_prio);
    _prio
}