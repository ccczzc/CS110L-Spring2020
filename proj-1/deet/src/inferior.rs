// use libc::WNOWAIT;
use nix::sys::ptrace;
use nix::sys::signal;
use nix::sys::signal::Signal;
use nix::sys::wait::{waitpid, WaitPidFlag, WaitStatus};
use nix::unistd::Pid;
use std::os::unix::process::CommandExt;
use std::process::Child;
use std::process::Command;
use std::mem::size_of;
use crate::dwarf_data::DwarfData;
use std::collections::HashMap;

pub enum Status {
    /// Indicates inferior stopped. Contains the signal that stopped the process, as well as the
    /// current instruction pointer that it is stopped at.
    Stopped(signal::Signal, usize),

    /// Indicates inferior exited normally. Contains the exit status code.
    Exited(i32),

    /// Indicates the inferior exited due to a signal. Contains the signal that killed the
    /// process.
    Signaled(signal::Signal),
}

/// This function calls ptrace with PTRACE_TRACEME to enable debugging on a process. You should use
/// pre_exec with Command to call this in the child process.
fn child_traceme() -> Result<(), std::io::Error> {
    ptrace::traceme().or(Err(std::io::Error::new(
        std::io::ErrorKind::Other,
        "ptrace TRACEME failed",
    )))
}

pub struct Inferior {
    child: Child,
    breakpoints: HashMap<usize, u8>,
}

fn align_addr_to_word(addr: usize) -> usize {
    addr & (-(size_of::<usize>() as isize) as usize)
}

impl Inferior {
    /// Attempts to start a new inferior process. Returns Some(Inferior) if successful, or None if
    /// an error is encountered.
    pub fn new(target: &str, args: &Vec<String>, breakpoints: HashMap<usize, u8>) -> Option<Inferior> {
        // TODO: implement me!
        let mut cmd = Command::new(target);
        cmd.args(args);
        unsafe {
            cmd.pre_exec(child_traceme);
        }
        match cmd.spawn() {
            Ok(child) => {
                match waitpid(nix::unistd::Pid::from_raw(child.id() as i32), Some(WaitPidFlag::WUNTRACED)).ok()? {
                    WaitStatus::Stopped(_, signal) => {
                        if signal != Signal::SIGTRAP {
                            println!("WaitStatus::Stopped : Not signaled by SIGTRAP!");
                            return None;
                        }
                    },
                    _ => {
                        println!("Other Status!");
                        return None;
                    },
                }
                let mut res = Inferior {child, breakpoints};
                // Install breakpoints
                for breakpoint in res.breakpoints.clone() {
                    if let Err(_) = res.insert_breakpoint(breakpoint.0) {
                        eprintln!("Breakpoint Install failed!");
                    }
                }
                // println!("Check signal SIGTRAP succeed!");
                Some(res)
            },
            Err(_) => None,
        }
    }

    /// Returns the pid of this inferior.
    pub fn pid(&self) -> Pid {
        nix::unistd::Pid::from_raw(self.child.id() as i32)
    }

    /// Calls waitpid on this inferior and returns a Status to indicate the state of the process
    /// after the waitpid call.
    pub fn wait(&self, options: Option<WaitPidFlag>) -> Result<Status, nix::Error> {
        Ok(match waitpid(self.pid(), options)? {
            WaitStatus::Exited(_pid, exit_code) => Status::Exited(exit_code),
            WaitStatus::Signaled(_pid, signal, _core_dumped) => Status::Signaled(signal),
            WaitStatus::Stopped(_pid, signal) => {
                let regs = ptrace::getregs(self.pid())?;
                Status::Stopped(signal, regs.rip as usize)
            }
            other => panic!("waitpid returned unexpected status: {:?}", other),
        })
    }

    pub fn continue_execute(&mut self) -> Result<Status, nix::Error> {
        let mut regs = ptrace::getregs(self.pid())?;
        let instruction_ptr = (regs.rip as usize) - 1;
        if let Some(rip_original_byte) = self.breakpoints.get(&instruction_ptr) {
            if self.write_byte(instruction_ptr, *rip_original_byte).is_ok() {
                regs.rip = instruction_ptr as u64;
                if let Err(er) = ptrace::setregs(self.pid(), regs) { 
                    eprintln!("Breakpoint: set rip = rip-1 failed!");
                    return Err(er);
                }
                if let Err(er) = ptrace::step(self.pid(), None) {
                    eprintln!("Breakpoint: go to next instruction failed!");
                    return Err(er);
                }
                match self.wait(None) {
                    Ok(status) => {
                        if let Status::Stopped(_, _) = status {
                            if let Err(er) = self.write_byte(instruction_ptr, 0xcc) {
                                return Err(er);
                            };
                        }
                    } ,
                    Err(er) => return Err(er),
                }
            } else {
                eprintln!("Breakpoint: restore first byte failed!");
            }
        }
        ptrace::cont(self.pid(), None)?;
        self.wait(None)
    }

    pub fn kill(&mut self) {
        if self.child.kill().is_ok() {
            self.wait(None).ok();
            println!("Killing running inferior (pid {})", self.pid());
        }
    }

    pub fn print_backtrace(&self, debug_data: &DwarfData) -> Result<(), nix::Error> {
        let regs = ptrace::getregs(self.pid())?;
        let mut instruction_ptr = regs.rip as usize;
        let mut base_ptr = regs.rbp as usize;
        let mut debug_current_line = debug_data.get_line_from_addr(instruction_ptr);
        let mut debug_current_func = debug_data.get_function_from_addr(instruction_ptr);
        loop {
            // println!("%rip register: {:#x}, %rbp register: {:#x}", instruction_ptr, base_ptr);
            let func_name = debug_current_func.as_ref().unwrap();
            let file_name = &debug_current_line.as_ref().unwrap().file;
            let code_line = debug_current_line.as_ref().unwrap().number;
            println!("{} ({}:{})", func_name, file_name, code_line);
            if func_name == "main" { break; }
            instruction_ptr = ptrace::read(self.pid(), (base_ptr + 8) as ptrace::AddressType)? as usize;
            base_ptr = ptrace::read(self.pid(), base_ptr as ptrace::AddressType)? as usize;
            debug_current_line = debug_data.get_line_from_addr(instruction_ptr);
            debug_current_func = debug_data.get_function_from_addr(instruction_ptr);
        }        
        Ok(())
    }

    pub fn write_byte(&mut self, addr: usize, val: u8) -> Result<u8, nix::Error> {
        let aligned_addr = align_addr_to_word(addr);
        let byte_offset = addr - aligned_addr;
        let word = ptrace::read(self.pid(), aligned_addr as ptrace::AddressType)? as u64;
        let orig_byte = (word >> 8 * byte_offset) & 0xff;
        let masked_word = word & !(0xff << 8 * byte_offset);
        let updated_word = masked_word | ((val as u64) << 8 * byte_offset);
        unsafe { ptrace::write(
            self.pid(),
            aligned_addr as ptrace::AddressType,
            updated_word as *mut std::ffi::c_void,
        ) }?;
        Ok(orig_byte as u8)
    }

    pub fn insert_breakpoint(&mut self, addr: usize) -> Result<(), ()> {
        if let Ok(original_byte) = self.write_byte(addr, 0xcc) {
            self.breakpoints.insert(addr, original_byte);
            Ok(())
        } else {
            Err(())
        }
    }
}
