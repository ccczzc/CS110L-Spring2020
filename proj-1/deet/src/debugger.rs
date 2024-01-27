use crate::debugger_command::DebuggerCommand;
use crate::inferior::{Inferior, Status};
// use nix::sys::ptrace;
use nix::sys::signal::Signal;
use rustyline::error::ReadlineError;
use rustyline::history::FileHistory;
use rustyline::Editor;
use crate::dwarf_data::{DwarfData, Error as DwarfError};
use std::collections::HashMap;

pub struct Debugger {
    target: String,
    history_path: String,
    readline: Editor<(), FileHistory>,
    inferior: Option<Inferior>,
    debug_data: DwarfData,
    breakpoints: HashMap<usize, u8>,
}

impl Debugger {
    /// Initializes the debugger.
    pub fn new(target: &str) -> Debugger {
        // (milestone 3): initialize the DwarfData
        let debug_data = match DwarfData::from_file(target) {
            Ok(val) => val,
            Err(DwarfError::ErrorOpeningFile) => {
                println!("Could not open file {}", target);
                std::process::exit(1);
            }
            Err(DwarfError::DwarfFormatError(err)) => {
                println!("Could not debugging symbols from {}: {:?}", target, err);
                std::process::exit(1);
            }
        };
        debug_data.print();

        let history_path = format!("{}/.deet_history", std::env::var("HOME").unwrap());
        let mut readline = Editor::<(), FileHistory>::new().expect("Create Editor fail");
        // Attempt to load history from ~/.deet_history if it exists
        let _ = readline.load_history(&history_path);

        Debugger {
            target: target.to_string(),
            history_path,
            readline,
            inferior: None,
            debug_data,
            breakpoints: HashMap::new(),
        }
    }

    pub fn run(&mut self) {
        loop {
            match self.get_next_command() {
                DebuggerCommand::Run(args) => {
                    if self.inferior.is_some(){
                        // println!("There exit running process!");
                        self.inferior.as_mut().unwrap().kill();
                        self.inferior = None;
                    }
                    if let Some(inferior) = Inferior::new(&self.target, &args, self.breakpoints.clone()) {
                        // Create the inferior
                        self.inferior = Some(inferior);
                        // (milestone 1): make the inferior run
                        // You may use self.inferior.as_mut().unwrap() to get a mutable reference
                        // to the Inferior object
                        if self.inferior_continue_execute().is_err() {
                            eprintln!("Error starting subprocess");
                        }
                    } else {
                        eprintln!("Error starting subprocess");
                    }
                },
                DebuggerCommand::Quit => {
                    if self.inferior.is_some() {
                        // println!("There exit running process!");
                        self.inferior.as_mut().unwrap().kill();
                        self.inferior = None;
                    }
                    return;
                },
                DebuggerCommand::Continue => {
                    if self.inferior.is_none() {
                        eprintln!("No existing inferior is running!");
                    } else {
                        if self.inferior_continue_execute().is_err() {
                            eprintln!("Continue Execute failed!");
                        }
                    }
                },
                DebuggerCommand::Backtrace => {
                    if self.inferior.is_none() {
                        eprintln!("No existing inferior is running!");
                    } else {
                        if self.inferior.as_ref().unwrap().print_backtrace(&self.debug_data).is_err() {
                            eprintln!("Backtrace failed!");
                        }
                    }
                },
                DebuggerCommand::Breakpoint(breakpoint) => {
                    let mut breakpoint_address: usize = 0;
                    if breakpoint.starts_with("*") {    // raw address
                        if let Some(bp_addr) = Debugger::parse_address(&breakpoint[1..]) {
                            breakpoint_address = bp_addr;
                        }
                    } else if let Ok(line_number) = breakpoint.parse::<usize>() {   // line number
                        if let Some(addr) = self.debug_data.get_addr_for_line(None, line_number) {
                            breakpoint_address = addr;
                        }
                    } else if let Some(addr) = self.debug_data.get_addr_for_function(None, &breakpoint) {
                        breakpoint_address = addr;
                    } else {
                        eprintln!("{} can't be parsed to a valid breakpoint address!", breakpoint);
                        eprintln!("Usage: {{b | break | breakpoint}} *{{raw address | line number | function name}}");
                        continue;
                    }

                    self.breakpoints.insert(breakpoint_address, 0);
                    if self.inferior.is_none() {
                        println!("Set breakpoint {} at {:#x}", self.breakpoints.len() - 1, breakpoint_address);
                    } else {
                        if self.inferior.as_mut().unwrap().insert_breakpoint(breakpoint_address).is_err() {
                            eprintln!("Breakpoint Install failed!");
                        } else {
                            println!("Set breakpoint {} at {:#x}", self.breakpoints.len() - 1, breakpoint_address);
                        }
                    }
                },
            }
        }
    }

    /// This function encapsualte inferior.continue_execute() to Debugger::inferior_continue_execute
    /// can print status of inferior according to its signal 
    pub fn inferior_continue_execute(&mut self) -> Result<(), ()>{
        match self.inferior.as_mut().unwrap().continue_execute() {
            Ok(continue_res) => {
                match continue_res {
                    Status::Stopped(stopped_signal, cur_addr) => {
                        self.print_stopped_location(stopped_signal, cur_addr);
                    },
                    Status::Exited(exit_code) => {
                        println!("Child exited (status {})", exit_code);
                        self.inferior = None;
                    },
                    Status::Signaled(signaled_signal) => println!("Child exited due to signal {}", signaled_signal),
                };
                Ok(())
            },
            Err(_) => Err(()),
        }
    }

    /// This function prints the reason why the process stop(by which signal) 
    /// and current stopped location & function & line number(if is_some())
    pub fn print_stopped_location(&mut self, stopped_signal: Signal, cur_addr: usize) {
        println!("Child stopped (signal {})", stopped_signal.as_str());
        let debug_current_line = self.debug_data.get_line_from_addr(cur_addr);
        let debug_current_func = self.debug_data.get_function_from_addr(cur_addr);
        if debug_current_line.is_some() && debug_current_func.is_some() {
            let func_name = debug_current_func.as_ref().unwrap();
            let file_name = &debug_current_line.as_ref().unwrap().file;
            let code_line = debug_current_line.as_ref().unwrap().number;
            println!("Stopped at {} ({}:{})", func_name, file_name, code_line);
        }
    }

    /// This function prompts the user to enter a command, and continues re-prompting until the user
    /// enters a valid command. It uses DebuggerCommand::from_tokens to do the command parsing.
    ///
    /// You don't need to read, understand, or modify this function.
    fn get_next_command(&mut self) -> DebuggerCommand {
        loop {
            // Print prompt and get next line of user input
            match self.readline.readline("(deet) ") {
                Err(ReadlineError::Interrupted) => {
                    // User pressed ctrl+c. We're going to ignore it
                    println!("Type \"quit\" to exit");
                }
                Err(ReadlineError::Eof) => {
                    // User pressed ctrl+d, which is the equivalent of "quit" for our purposes
                    return DebuggerCommand::Quit;
                }
                Err(err) => {
                    panic!("Unexpected I/O error: {:?}", err);
                }
                Ok(line) => {
                    if line.trim().len() == 0 {
                        continue;
                    }
                    self.readline.add_history_entry(line.as_str()).ok();
                    if let Err(err) = self.readline.save_history(&self.history_path) {
                        println!(
                            "Warning: failed to save history file at {}: {}",
                            self.history_path, err
                        );
                    }
                    let tokens: Vec<&str> = line.split_whitespace().collect();
                    if let Some(cmd) = DebuggerCommand::from_tokens(&tokens) {
                        return cmd;
                    } else {
                        println!("Unrecognized command.");
                    }
                }
            }
        }
    }

    pub fn parse_address(addr: &str) -> Option<usize> {
        let addr_without_0x = if addr.to_lowercase().starts_with("0x") {
            &addr[2..]
        } else {
            &addr
        };
        usize::from_str_radix(addr_without_0x, 16).ok()
    }
}
