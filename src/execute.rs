use std::{
    cmp,
    process,
    sync::mpsc, 
    thread::{self, JoinHandle}
};

use crate::{execute, phrase_matcher as pm};

#[derive(serde::Deserialize, cmp::Eq, cmp::Ord, cmp::PartialEq, cmp::PartialOrd, Clone)]
pub enum PhrasePosition {
    Any, Start, Exact, Err
}

impl serde::Serialize for PhrasePosition {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer {
        // Use snakecase representations of enum values for JSON
        let str = match self {
            PhrasePosition::Any => "any",
            PhrasePosition::Start => "at_start",
            PhrasePosition::Exact => "match_exact",
            PhrasePosition::Err => "unknown"
        };

        serializer.serialize_str(str)
    }
}

impl std::fmt::Display for PhrasePosition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let pos = match self {
            PhrasePosition::Any => "anywhere",
            PhrasePosition::Start => "at start",
            PhrasePosition::Exact => "only exact matches",
            PhrasePosition::Err => "unknown"
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

pub type ExecuteReceiver = mpsc::Receiver<anyhow::Result<String>>;

pub fn run_command_executor(match_receiver : pm::PhraseMatchReceiver, 
    executables : Vec<execute::Executable>) -> anyhow::Result<(ExecuteReceiver, JoinHandle<()>)> {

    let (sender, receiver) = mpsc::channel();

    let executer_thread = thread::spawn(move || {
        loop {
            if let Ok(data) = match_receiver.recv() {
                match data {
                    Ok((i, phrase_args)) => {
                        let command_data = match apply_arguments(&executables[i], &phrase_args) {
                            Ok(data) => data,
                            Err(e) => {
                                eprintln!("Error processing command: {e}");
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
                    },
                    Err(e) => {
                        sender.send(Err(e)).ok();
                        return;
                    }
                }
            }
            else { // Senders have been dropped
                eprintln!("Execute thread exited!");
                return;
            }
        }
    });

    Ok((receiver, executer_thread))
}

fn apply_arguments(executable : &Executable, arguments : &pm::PhraseArgs) -> anyhow::Result<Executable> {
    
    let mut wildcard_args = arguments.wildcard_args.clone();
    let list_args = arguments.list_args.clone();
    
    let num_wildcards = executable.args.iter()
        .filter(|arg| pm::arg_type(arg) == pm::ArgumentType::Wildcard).count();

    let has_multi_arg = executable.args.iter().any(|arg| pm::arg_type(arg) == pm::ArgumentType::List);

    anyhow::ensure!(
        // If the command doesn't have variable-length arguments, its arguments
        // list and the supplied arguments list should be the same length
        (!has_multi_arg && arguments.wildcard_args.len() == num_wildcards) ||
        // If the command has a variable-length argument, the supplied arguments
        // list should be the same length or longer
        (has_multi_arg && arguments.wildcard_args.len() >= num_wildcards),
        "Argument length of phrase and executable did not match");

    // Add default and parameterized arguments
    let mut applied_args : Vec<String> = vec!();
    executable.args.iter().for_each(|arg| {
        if pm::arg_type(arg) == pm::ArgumentType::Wildcard {
            let wildcard_opt = wildcard_args.iter().enumerate()
                .find(|(_index, (label, _data))| label == arg);

            if let Some((arg_index, arg_data)) = wildcard_opt {
                applied_args.push(arg_data.1.clone());
                wildcard_args.remove(arg_index);
            }
            else {
                eprintln!("Mismatched arguments detected!");
            }
        }
        else if pm::arg_type(arg) == pm::ArgumentType::List {
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

pub fn validate_executable(executable : &Executable) -> anyhow::Result<()> {

    // Verify that there is at most one variable-length arg, at the end of the phrase
    let mut num_list_args = executable.phrase.split(' ')
        .filter(|str| pm::arg_type(str) == pm::ArgumentType::List).count();

    if num_list_args > 1 || (num_list_args == 1 && !executable.phrase.ends_with("...>")) {
        anyhow::bail!("Executable may only contain a singular variable-length argument, and it must be at the end of the phrase");
    }
    
    // Verify that there is at most one variable-length arg in the arguments list
    num_list_args = executable.args.iter()
        .filter(|arg| pm::arg_type(arg) == pm::ArgumentType::List)
        .count();

    if num_list_args > 1 {
        anyhow::bail!("Executable may only contain a singular variable-length argument in the arguments list");
    }
    
    // Verify that each argument in the arguments list matches to at least one in the phrase
    for arg in executable.args.iter()
        .filter(|arg| pm::arg_type(arg) != pm::ArgumentType::Default) {
        if !executable.phrase.contains(arg) {
            anyhow::bail!("Expected phrase to reference argument \"{}\"", arg);
        }
    }

    // Verify that each argument label in the phrase matches to at least one in the arguments list
    for arg in executable.phrase.split(' ')
        .filter(|str| pm::arg_type(str) != pm::ArgumentType::Default) {
        if !executable.args.contains(&arg.to_owned()) {
            anyhow::bail!("Phrase references unknown argument \"{}\"", arg);
        }
    }

    if executable.phrase_position == execute::PhrasePosition::Err {
        anyhow::bail!("Invalid match type");
    }

    Ok(())
}

pub fn print_executables(executables : &[crate::execute::Executable]) {
    executables.iter().enumerate().for_each(|(i, ex)| {
        print!("[{}] {{\
            \n        phrase: {}\
            \n        command: {}\
            \n        arguments: [", i + 1, ex.phrase, ex.command);

        if ex.args.is_empty() {
            println!("]");
        }
        else {
            println!();

            ex.args.iter().for_each(|arg| {
                println!("            {}", arg);
            });

            println!("        ]");
        }

        println!("        match type: {}\n    }}\n", ex.phrase_position);
    });
}
