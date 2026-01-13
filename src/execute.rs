use std::{cmp, process, sync::{Arc, atomic::{AtomicBool, Ordering}, mpsc}, thread::{self, JoinHandle}};

use crate::phrase_matcher::{ArgumentType, PhraseArgs, arg_type};

#[derive(serde::Deserialize, cmp::Eq, cmp::Ord, cmp::PartialEq, cmp::PartialOrd, Clone)]
pub enum PhrasePosition {
    Any, Start, Exact
}

impl serde::Serialize for PhrasePosition {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer {
        // Use snakecase representations of enum values for JSON
        let str = match self {
            PhrasePosition::Any => "any",
                PhrasePosition::Start => "at_start",
                PhrasePosition::Exact => "match_exact"
        };

        serializer.serialize_str(str)
    }
}

impl std::fmt::Display for PhrasePosition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let pos = match self {
            PhrasePosition::Any => "anywhere",
                PhrasePosition::Start => "at the start",
                PhrasePosition::Exact => "only exact matches"
        };

        write!(f, "{pos}")
    }
}

#[derive(serde::Deserialize, serde::Serialize, cmp::Eq, cmp::Ord, cmp::PartialEq, cmp::PartialOrd, Clone)]
pub struct Executable {
    pub phrase : String,
    pub command : String,
    pub args : Vec<String>,
    pub phrase_position : PhrasePosition
}

pub type ExecuteReceiver = mpsc::Receiver<anyhow::Result<String, String>>;

pub fn run_command_executor(match_receiver : crate::phrase_matcher::PhraseMatchReceiver, executables : Vec<crate::execute::Executable>, 
    is_running : Arc<AtomicBool>) -> anyhow::Result<(ExecuteReceiver, JoinHandle<()>)> {

    let (sender, receiver) = mpsc::channel();

    let executer_thread = thread::spawn(move || {
        while is_running.load(Ordering::Relaxed) {
            while let Ok(data) = match_receiver.try_recv() {
                match data {
                    Ok((i, phrase_args)) => {
                        let command_data = match apply_arguments(&executables[i], &phrase_args) {
                            Ok(data) => data,
                            Err(e) => {
                                eprintln!("Error: {e}");
                                return;
                            }
                        };

                        let command_proc = command_data.command;
                        let command_args = command_data.args;
                        let mut command = process::Command::new(&command_proc);
                        command.args(command_args);

                        // Ignore input/output streams
                        command.stdin(process::Stdio::null());
                        command.stdout(process::Stdio::null());
                        command.stderr(process::Stdio::null());

                        // Run the command
                        match command.spawn() {
                            Ok(mut child) => {
                                eprintln!("Started process '{}' ({})", command_proc, child.id());

                                // Create a new thread which waits for the closure of the running process
                                thread::spawn(move || {
                                    let _ = child.wait();
                                    eprintln!("Process '{}' completed ({}))", command_proc, child.id());
                                });
                            },
                            Err(e) => {
                                eprintln!("Failed to start process '{}'", command_proc);
                                eprintln!("{:?}", e.to_string());
                            }
                        }

                        // sender.send(Ok(command_proc)).ok();
                    },
                    Err(e) => {
                        sender.send(Err(e)).ok();
                        return;
                    }
                }
            }
        }
    });

    Ok((receiver, executer_thread))
}

fn apply_arguments(executable : &Executable, arguments : &PhraseArgs) -> anyhow::Result<Executable, String> {
    
    let mut wildcard_args = arguments.wildcard_args.clone();
    let list_args = arguments.list_args.clone();
    
    let num_wildcards = executable.args.iter().filter(|arg| arg_type(arg) == ArgumentType::Wildcard).count();
    let has_multi_arg = executable.args.iter().any(|arg| arg_type(arg) == ArgumentType::List);

    if (!has_multi_arg && num_wildcards != arguments.wildcard_args.len()) || (has_multi_arg && num_wildcards > arguments.wildcard_args.len()) {
        return Err(String::from("Argument length of phrase and executable did not match"));
    }

    // Add default and paramaterized arguments
    let mut applied_args : Vec<String> = vec!();
    executable.args.iter().for_each(|arg| {
        if arg_type(arg) == ArgumentType::Wildcard {
            let wildcard_arg = wildcard_args.iter().enumerate().find(|(_, (label, _))| label == arg)
                .expect("Invalid argument label encountered");

            applied_args.push(wildcard_arg.1.1.clone());
            wildcard_args.remove(wildcard_arg.0);
        }
        else if arg_type(arg) == ArgumentType::List {
            if let Some(mut args_list) = list_args.clone() {
                applied_args.append(&mut args_list.1);
            }
        }
        else { 
            applied_args.push(arg.clone());
        }
    });

    Ok(Executable {
        phrase: executable.phrase.clone(),
        command: executable.command.clone(),
        args: applied_args,
        phrase_position: executable.phrase_position.clone()
    })
}

pub fn validate_executable(executable : &Executable) -> bool {

    // Verify that there is at most one variable-length arg, at the end of the phrase
    let mut num_list_args = executable.phrase.split(' ').filter(|str| arg_type(str) == ArgumentType::List).count();
    if num_list_args > 1 || (num_list_args == 1 && !executable.phrase.ends_with("...>")) {
        eprintln!("Executable with phrase {} may only contain a singular variable-length argument, and it must be at the end of the phrase", executable.phrase);
        return false;
    }
    
    // Verify that there is at most one variable-length arg in the arguments list
    num_list_args = executable.args.iter().filter(|arg| arg_type(arg) == ArgumentType::List).count();
    if num_list_args > 1 {
        eprintln!("Executable with phrase {} may only contain a singular variable-length argument in the arguments list", executable.phrase);
        return false;
    }
    
    // Verify that each argument in the arguments list matches to at least one in the phrase
    for arg in executable.args.iter().filter(|arg| arg_type(arg) != ArgumentType::Default) {
        if !executable.phrase.contains(arg) {
            eprintln!("Executable with phrase \"{}\" is expected to reference argument \"{}\"", executable.phrase, arg);
            return false;
        }
    }

    // Verify that each argument label in the phrase matches to at least one in the arguments list
    for arg in executable.phrase.split(' ').filter(|str| arg_type(str) != ArgumentType::Default) {
        if !executable.args.contains(&arg.to_owned()) {
            eprintln!("Executable with phrase \"{}\" references unknown argument \"{}\"", executable.phrase, arg);
        }
    }
    
    true
}

pub fn print_executables(executables : &[crate::execute::Executable]) {
    executables.iter().enumerate().for_each(|(i, ex)| {
        println!("[{i}] {{\
            \n    phrase: {}\
            \n    command: {}\
            \n    arguments: [", ex.phrase, ex.command);

        ex.args.iter().for_each(|arg| {
            println!("      {}", arg);
        });

        println!("    ]\
            \n    match type: {}\n}}\n", ex.phrase_position);
    });
}
