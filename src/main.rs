mod config_manager;
mod input;
mod asr_handler;
mod phrase_matcher;
mod execute;

use config_manager as config;
use std::{path, sync::{Arc, atomic::{AtomicBool, Ordering}}};

struct ProgramArgs {
    add_command : bool,
    input_device_index_str : Option<String>,
    help : bool,
    model_path : Option<String>,
    print_asr_results : bool,
    query_devices : bool,
    remove_command : bool
}

struct ProgramData {
    input_device : cpal::Device,
    model_path : path::PathBuf,
    executables : Vec<crate::execute::Executable>,
    print_asr_results : bool
}

fn main() -> anyhow::Result<(), String> {
    let program_args = process_cli_args()?;

    if program_args.help {
        print_help();
        return Ok(());
    }

    let phrases_path = config::init_config_directory()?;
    let dirty_executables = crate::config::load_executables(&phrases_path)
        .map_err(|e| format!("Error occurred while reading executable data from phrases.json {}", e))?;

    let mut executables = dirty_executables.iter().filter_map(|ex| { 
        if execute::validate_executable(ex) { Some(ex.clone()) }
        else { None }
    }).collect::<Vec<execute::Executable>>();

    eprintln!("Loaded valid executables");

    if program_args.add_command {
        crate::config::add_executable(&mut executables)?;
        crate::config::write_executables(executables, &phrases_path)?;
        return Ok(());
    }
    else if program_args.remove_command {
        crate::config::remove_executable(&mut executables)?;
        crate::config::write_executables(executables, &phrases_path)?;
        return Ok(());
    }

    let input_devices = input::get_cpal_input_devices()?;
    let mut input_device_index : usize = 0;

    if program_args.query_devices {
        if !input_devices.is_empty() {
            println!("Available Devices:");
            input::print_cpal_device_descriptions(&input_devices);
        }
        else {
            println!("No suitable input devices detected");
        }

        return Ok(());
    }

    if let Some(index_str) = &program_args.input_device_index_str {
        let i : i32 = index_str.parse().unwrap_or(-1);
        input_device_index = check_device_index(i, &input_devices)?;
    }

    let mut model_path = program_args.model_path.map(path::PathBuf::from);

    if model_path.is_none() {
        let model_path_str = std::env::var("VOSK_MODEL_PATH").map_err(|_| String::from("Missing path to VOSK model. See 'jarvis-asr --help' for proper usage"))?;
        model_path = Some(path::PathBuf::from(model_path_str));
    }

    let model_path = model_path.unwrap();

    let input_device = if input_device_index == 0 { 
        input::get_cpal_default_input_device()?
    } 
    else {
        input_devices.get(input_device_index - 1).expect("Device index out of bounds").clone()
    };

    let data = ProgramData {
        input_device,
        model_path,
        executables,
        print_asr_results : program_args.print_asr_results
    };

    match run(data) {
        Ok(_) => {},
        Err(e) => {
            return Err(e.to_string())
        }
    };

    eprintln!("Exited");
    Ok(())
}

fn process_cli_args() -> Result<ProgramArgs, String> {

    let mut program_args = ProgramArgs {
        add_command : false,
        input_device_index_str : None,
        help : false,
        model_path : None,
        print_asr_results : false,
        query_devices : false,
        remove_command : false
    };

    let args = std::env::args().skip(1) 
        .collect::<Vec<String>>();

    let mut opts = getargs::Options::new(args.iter().map(String::as_str));

    while let Some(opt) = opts.next_opt().expect("Error parsing arguments") {

        match opt {
            getargs::Opt::Short('a') | getargs::Opt::Long("add-command") => {
                program_args.add_command = true;
            }
            getargs::Opt::Short('d') | getargs::Opt::Long("device") => {
                let arg_m = opts.value().map_err(|e| e.to_string())?;
                program_args.input_device_index_str = Some(arg_m.to_owned());
            },
            getargs::Opt::Short('h') | getargs::Opt::Long("help") => {
                program_args.help = true;
            },
            getargs::Opt::Short('m') | getargs::Opt::Long("model") => {
                let arg_m = opts.value().map_err(|e| e.to_string())?;
                program_args.model_path = Some(arg_m.to_owned());
            },
            getargs::Opt::Short('p') | getargs::Opt::Long("print") => {
                program_args.print_asr_results = true;
            }
            getargs::Opt::Short('q') | getargs::Opt::Long("query-devices") => {
                program_args.query_devices = true;
            },
            getargs::Opt::Short('r') | getargs::Opt::Long("remove-command") => {
                program_args.remove_command = true;
            },
            _ => { 
                return Err(format!("Unknown argument {:?}", opt.to_string()));
            }
        }
    }
    
    check_arg_compatibility(&program_args)?;

    Ok(program_args)
}

fn check_arg_compatibility(program_args : &ProgramArgs) -> Result<(), String> {
    
    let err = Err(String::from("Incompatible combination of arguments"));

    if program_args.add_command && (program_args.remove_command || program_args.query_devices 
    || program_args.print_asr_results || program_args.model_path.is_some() || program_args.input_device_index_str.is_some()) {
        return err;
    }

    if program_args.remove_command && (program_args.query_devices || program_args.print_asr_results
    || program_args.model_path.is_some() || program_args.input_device_index_str.is_some()) {
        return err;
    }

    if program_args.query_devices && (program_args.print_asr_results || program_args.model_path.is_some()
    || program_args.input_device_index_str.is_some()) {
        return err;
    }

    Ok(())
}

fn run(data : ProgramData) -> anyhow::Result<(), String> { 

    let ProgramData { input_device, model_path, executables, print_asr_results } = data;

    let is_running : Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
    
    let is_running_copy = is_running.clone();
    ctrlc::set_handler(move || {
        is_running_copy.store(false, Ordering::Relaxed);
    }).expect("Could not set handler for ctrl-c");

    let (_cpal_stream, sample_rate, cpal_receiver) = input::init_cpal_input_stream(input_device, is_running.clone())
    .map_err(|e| format!("Error obtaining connection to recording thread: {}", e))?;

    // Receiver to catch audio data from stream
    let cpal_receiver = cpal_receiver.expect("Error obtaining connection to recording thread");

    let (asr_receiver, asr_thread) = asr_handler::run_asr(
        &model_path, cpal_receiver, sample_rate, is_running.clone(), print_asr_results
    )
    .map_err(|e| format!("Error obtaining connection to ASR thread: {}", e))?;

    let match_res = phrase_matcher::run_phrase_matcher(asr_receiver, &executables, is_running.clone());
    let (match_receiver, match_thread) = match_res.expect("Error obtaining connection to phrase matching thread");
    
    let executor_res = execute::run_command_executor(match_receiver, executables, is_running.clone());
    let (execute_receiver, execute_thread) = executor_res.expect("Error obtaining connection to execution thread");
    
    // Main program loop
    while is_running.load(Ordering::Relaxed) {
        while let Ok(data) = execute_receiver.try_recv() {
            match data {
                Ok(data) => {
                        println!("{}", data);
                },
                Err(e) => {
                    eprintln!("Error: {:?}", e);
                    eprintln!("A stream error occurred. Try using a different input device?");

                    is_running.store(false, Ordering::SeqCst);
                    break;
                }
            }
        }
    }

    eprintln!("\nClosing execution thread...");
    execute_thread.join().expect("Error occurred while closing execution thread");

    eprintln!("Closing processing thread...");
    match_thread.join().expect("Error occurred while closing processing thread");

    eprintln!("Closing ASR thread...");
    asr_thread.join().expect("Error occurred while closing ASR thread");

    Ok(())
}

fn print_help() {
    println!("Usage: \
        \n\tjarvis-asr [options] \
        \n\nRun phrase detection using a user-provided VOSK model. \
        \n\nOptions: \
        \n -a, --add-command\t\trun the add command utility \
        \n -d, --device <num>\t\tuse the specified input device (see '--query-devices') \
        \n -h, --help\t\t\tdisplay this message \
        \n -m, --model <path-to-model>\tuse the VOSK model located at the specified path  \
        \n -p, --print\t\t\tprint detected speech \
        \n -q, --query-devices\t\tdisplay available input devices to use for recording \
        \n -r, --remove-command\t\trun the remove command utility \
        \n\nA VOSK model is required to run speech recognition in the program. \
        \nThe root directory of the model should be provided through the '--model' argument or by setting the VOSK_MODEL_PATH environment variable.
        ");
}

fn check_device_index(i : i32, input_devices : &[cpal::Device]) -> Result<usize, String> {

    if i < 0 || i > input_devices.len() as i32 { // 0 = default, 1 - len() = specific device
        return Err(String::from("Error: Invalid device index"));
    }

    Ok(i as usize)
}
