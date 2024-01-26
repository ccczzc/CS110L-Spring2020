use crate::debugger_command::DebuggerCommand;
use crate::inferior::{Inferior, Status};
use rustyline::error::ReadlineError;
use rustyline::history::FileHistory;
use rustyline::Editor;
use crate::dwarf_data::{DwarfData, Error as DwarfError};

pub struct Debugger {
    target: String,
    history_path: String,
    readline: Editor<(), FileHistory>,
    inferior: Option<Inferior>,
    debug_data: DwarfData,
    breakpoints: Vec<usize>,
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
            breakpoints: Vec::new(),
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
                    if let Some(inferior) = Inferior::new(&self.target, &args, &self.breakpoints) {
                        // Create the inferior
                        self.inferior = Some(inferior);
                        // TODO (milestone 1): make the inferior run
                        // You may use self.inferior.as_mut().unwrap() to get a mutable reference
                        // to the Inferior object
                        let continue_res = self.inferior.as_ref().unwrap().continue_execute();
                        if continue_res.is_ok() {
                            match continue_res.unwrap() {
                                Status::Stopped(stopped_signal, rip) => {
                                    println!("Child stopped (signal {})", stopped_signal.as_str());
                                    let debug_current_line = self.debug_data.get_line_from_addr(rip);
                                    let debug_current_func = self.debug_data.get_function_from_addr(rip);
                                    if debug_current_line.is_some() && debug_current_func.is_some() {
                                        let func_name = debug_current_func.as_ref().unwrap();
                                        let file_name = &debug_current_line.as_ref().unwrap().file;
                                        let code_line = debug_current_line.as_ref().unwrap().number;
                                        println!("Stopped at {} ({}:{})", func_name, file_name, code_line);
                                        println!("{:#x}", rip);
                                    }
                                },
                                Status::Exited(exit_code) => {
                                    println!("Child exited (status {})", exit_code);
                                    self.inferior = None;
                                },
                                Status::Signaled(signaled_signal) => println!("Child exited exited due to signal {}", signaled_signal),
                            }
                        } else {
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
                        let continue_res = self.inferior.as_ref().unwrap().continue_execute();
                        if continue_res.is_ok() {
                            match continue_res.unwrap() {
                                Status::Stopped(stopped_signal, rip) => {
                                    println!("Child stopped (signal {})", stopped_signal.as_str());
                                    let debug_current_line = self.debug_data.get_line_from_addr(rip);
                                    let debug_current_func = self.debug_data.get_function_from_addr(rip);
                                    if debug_current_line.is_some() && debug_current_func.is_some() {
                                        let func_name = debug_current_func.as_ref().unwrap();
                                        let file_name = &debug_current_line.as_ref().unwrap().file;
                                        let code_line = debug_current_line.as_ref().unwrap().number;
                                        println!("Stopped at {} ({}:{})", func_name, file_name, code_line);
                                    }
                                },
                                Status::Exited(exit_code) => {
                                    println!("Child exited (status {})", exit_code);
                                    self.inferior = None;
                                },
                                Status::Signaled(signaled_signal) => println!("Child exited exited due to signal {}", signaled_signal),
                            }
                        } else {
                            eprintln!("Continue failed!");
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
                    println!("Set breakpoint {} at {}", self.breakpoints.len(), &breakpoint[1..]);
                    if breakpoint.starts_with("*") {
                        let bp_addr = Debugger::parse_address(&breakpoint[1..]);
                        if bp_addr.is_some() {
                            if self.inferior.is_none() {
                                self.breakpoints.push(bp_addr.unwrap());
                            } else {
                                let aa = self.inferior.as_mut().unwrap().write_byte(bp_addr.unwrap(), 0xcc);
                                println!("{:?}", aa);
                            }
                            // println!("{}", bp_addr.unwrap());
                            
                        }
                    }
                },
            }
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
                    self.readline.add_history_entry(line.as_str());
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
