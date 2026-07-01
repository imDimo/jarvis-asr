mod autocomplete;
mod config_manager;
mod input;
mod asr_handler;
mod phrase_matcher;
mod execute;

use std::{
    path, 
    sync::{Arc, atomic::{AtomicBool, Ordering}}
};
use anyhow::Context;

use config_manager as config;
use phrase_matcher as pm;

struct ProgramArgs {
    add_command : bool,
    clean_command : bool,
    command_path : Option<String>,
    help : bool,
    input_device_index_str : Option<String>,
    model_path : Option<String>,
    print_asr_results : bool,
    query_devices : bool,
    remove_command : bool
}

struct ProgramData {
    input_device : cpal::Device,
    model_path : path::PathBuf,
    executables : Vec<execute::Executable>,
    print_asr_results : bool
}

fn main() -> anyhow::Result<()> {
    let program_args = process_cli_args()?;

    if program_args.help {
        print_help();
        return Ok(());
    }

    let config_path = config::init_config_directory()?;

    let commands_file = program_args.command_path.unwrap_or("commands.json".to_owned());
    let commands_path = config::init_commands(&config_path, path::Path::new(&commands_file))?;

    let mut dirty_executables = config::load_executables(&commands_path)?;

    eprintln!("Read {} executables from {}", dirty_executables.len(), commands_path.to_string_lossy());

    if program_args.clean_command {
        eprintln!("Cleaning executables...");
        let cleaned_executables = dirty_executables.iter().filter_map(|ex| { 
            match execute::validate_executable(ex) { 
                Ok(_) => Some(ex.clone()),
                Err(e) => {
                    eprintln!("Removing executable with phrase \"{}\": {}", ex.phrase, e);
                    None 
                }
            }
        }).collect::<Vec<execute::Executable>>();

        config::write_executables(cleaned_executables, &commands_path)?;
        return Ok(());
    }

    if program_args.add_command {
        if let Err(e) = config::add_executable(&mut dirty_executables) {
            eprintln!("Error adding executable: {}", e);
        }
        else {
            config::write_executables(dirty_executables, &commands_path)?;
        }
        
        return Ok(());
    }
    else if program_args.remove_command {
        config::remove_executable(&mut dirty_executables)?;
        config::write_executables(dirty_executables, &commands_path)?;
        return Ok(());
    }

    let executables = dirty_executables.iter().filter_map(|ex| { 
        match execute::validate_executable(ex) { 
            Ok(_) => Some(ex.clone()),
            Err(e) => {
                eprintln!("Error loading executable with phrase \"{}\": {}", ex.phrase, e);
                None 
            }
        }
    }).collect::<Vec<execute::Executable>>();

    eprintln!("Loaded valid executables");

    let input_devices = input::get_cpal_input_devices()?;
    let mut input_device_index : usize = 0;

    if program_args.query_devices {
        if !input_devices.is_empty() {
            println!("Available Devices:");
            input::print_cpal_device_descriptions(&input_devices)?;
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
        let model_path_str = std::env::var("VOSK_MODEL_PATH")
            .context("Missing path to VOSK model. See 'jarvis-asr --help' for proper usage")?;

        model_path = Some(path::PathBuf::from(model_path_str));
    }

    let model_path = model_path.unwrap();

    let input_device = if input_device_index == 0 { 
        input::get_cpal_default_input_device()?
    } 
    else {
        input_devices.get(input_device_index - 1)
            .expect("Device index out of bounds").clone() // This shouldn't happen
    };

    let data = ProgramData {
        input_device,
        model_path,
        executables,
        print_asr_results : program_args.print_asr_results
    };

    run(data)
}

fn process_cli_args() -> anyhow::Result<ProgramArgs> {

    let mut program_args = ProgramArgs {
        add_command : false,
        clean_command : false,
        command_path : None,
        help : false,
        input_device_index_str : None,
        model_path : None,
        print_asr_results : false,
        query_devices : false,
        remove_command : false
    };

    let args = std::env::args().skip(1) 
        .collect::<Vec<String>>();

    let mut opts = getargs::Options::new(args.iter().map(String::as_str));

    loop {
        let opt_read_result = opts.next_opt();
        anyhow::ensure!(opt_read_result.is_ok(), "Could not read CLI arguments");

        if let Some(opt) = opt_read_result.unwrap() {
            match opt {
                getargs::Opt::Short('a') | getargs::Opt::Long("add-command") => {
                    program_args.add_command = true;
                },
                getargs::Opt::Short('c') | getargs::Opt::Long("clean-commands") => {
                    program_args.clean_command = true;
                },
                getargs::Opt::Short('C') | getargs::Opt::Long("commands") => {
                    let arg_c = opts.value();
                    anyhow::ensure!(arg_c.is_ok(), "Argument 'C'/'commands' expected a commands path");
                    program_args.command_path = Some(arg_c.unwrap().to_owned());
                },
                getargs::Opt::Short('d') | getargs::Opt::Long("device") => {
                    let arg_d = opts.value();
                    anyhow::ensure!(arg_d.is_ok(), "Argument 'd'/'device' expected a device index");
                    program_args.input_device_index_str = Some(arg_d.unwrap().to_owned());
                },
                getargs::Opt::Short('h') | getargs::Opt::Long("help") => {
                    program_args.help = true;
                },
                getargs::Opt::Short('m') | getargs::Opt::Long("model") => {
                    let arg_m = opts.value();
                    anyhow::ensure!(arg_m.is_ok(), "Argument 'm'/'model' expected a model path");
                    program_args.model_path = Some(arg_m.unwrap().to_owned());
                },
                getargs::Opt::Short('p') | getargs::Opt::Long("print") => {
                    program_args.print_asr_results = true;
                },
                getargs::Opt::Short('q') | getargs::Opt::Long("query-devices") => {
                    program_args.query_devices = true;
                },
                getargs::Opt::Short('r') | getargs::Opt::Long("remove-command") => {
                    program_args.remove_command = true;
                },
                _ => {
                    anyhow::bail!(format!("Unknown argument {:?}", opt.to_string())); 
                }
            }
        }
        else {
            break;
        }
    }
    
    check_arg_compatibility(&program_args)?;

    Ok(program_args)
}

fn check_arg_compatibility(program_args : &ProgramArgs) -> anyhow::Result<()> {
    // Check if the program has any normal runtime arguments
    let has_normal_args = program_args.print_asr_results 
        || program_args.model_path.is_some()
        || program_args.input_device_index_str.is_some();

    // List of booleans describing which arguments are present
    // Only one should be true
    let arg_compat = [
        program_args.add_command, 
        program_args.remove_command,
        program_args.query_devices,
        program_args.clean_command,
        has_normal_args
    ];

    // If more than one special argument is specified, or any special arguments
    // are specified along with the normal arguments, the arguments are invalid
    let args_are_valid = arg_compat.iter().filter(|arg| **arg).count() <= 1;

    anyhow::ensure!(args_are_valid, "Incompatible combination of arguments");

    Ok(())
}

fn run(data : ProgramData) -> anyhow::Result<()> { 

    let ProgramData { input_device, model_path, executables, print_asr_results } = data;

    let is_running : Arc<AtomicBool> = Arc::new(AtomicBool::new(true));
    
    let is_running_copy = is_running.clone();
    ctrlc::set_handler(move || {
        is_running_copy.store(false, Ordering::Relaxed);
    }).context("Could not set handler for ctrl-c: {}")?;

    let (cpal_stream, sample_rate, cpal_receiver) = input::init_cpal_input_stream(input_device)
        .context("Error obtaining connection to recording thread: {}")?;

    // Receiver to catch audio data from stream
    let cpal_receiver = cpal_receiver
        .context("Error obtaining connection to recording thread")?;

    let (asr_receiver, asr_thread) = asr_handler::run_asr(
        &model_path, cpal_receiver, sample_rate, print_asr_results, is_running.clone()
    ).context("Error obtaining connection to ASR thread: {}")?;

    let match_res = pm::run_phrase_matcher(
        asr_receiver, executables.clone(), is_running.clone()
    );
    let (match_receiver, match_thread) = match_res
        .context("Error obtaining connection to phrase matching thread")?;
    
    let executor_res = execute::run_command_executor(
        match_receiver, executables.clone(), is_running.clone()
    );
    let (execute_receiver, execute_thread) = executor_res
        .context("Error obtaining connection to execution thread")?;

    // Main program loop - Controls when the program exits
    while is_running.load(Ordering::Relaxed) {
        if let Ok(data) = execute_receiver.try_recv() {
            match data {
                Ok(data) => {
                        eprintln!("{}", data);
                },
                Err(e) => {
                    eprintln!("{}", e);
                    eprintln!("A stream error occurred. Try using a different input device?");

                    is_running.store(false, Ordering::SeqCst);
                    break;
                }
            }
        }

        // Wait a short while to prevent constant CPU consuption
        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    eprintln!("\nDropping cpal stream");
    drop(cpal_stream);

    eprintln!("Stopping ASR thread");
    asr_thread.join().expect("ASR thread panicked");

    eprintln!("Stopping phrase-matching thread");
    match_thread.join().expect("Match thread panicked");

    eprintln!("Stopping execution thread");
    execute_thread.join().expect("Execute thread panicked");        

    Ok(())
}

fn print_help() {
    println!("Usage: \
        \n\tjarvis-asr [options] \
        \n\nRun phrase detection using a user-provided VOSK model. \
        \n\nOptions: \
        \n -a, --add-command\t\trun the add command utility \
        \n -c, --clean\t\t\tremove invalid commands \
        \n -C, --commands <path-to-json>\tuse the commands file located at the specified path \
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

fn check_device_index(i : i32, input_devices : &[cpal::Device]) -> anyhow::Result<usize> {
    // 0 = default, 1 to len() = specific device
    anyhow::ensure!((0..=input_devices.len() as i32).contains(&i), "Invalid device index");
    Ok(i as usize)
}
