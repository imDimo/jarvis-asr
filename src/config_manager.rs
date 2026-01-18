use std::{
    fs,
    path
};
use anyhow::Context;
use crate::autocomplete;
use crate::execute;

#[derive(serde::Deserialize, serde::Serialize)]
struct PhrasesStruct {
    executables: Vec<execute::Executable>
}

pub fn init_config_directory() -> anyhow::Result<path::PathBuf> {
    let project_dirs = directories::ProjectDirs::from("com", "imdimo", "jarvis-asr");

    let config_path = project_dirs.context("Could not obtain location of project config directory. Is your home set?")?
        .config_dir().to_owned();

    let config_path_exists = config_path.try_exists()
        .context(format!("Failed to verify existence of config directory path '{}'", config_path.to_string_lossy()))?;

    if !config_path_exists {
        fs::create_dir_all(&config_path)
            .context(format!("Failed to create config directory '{}'", config_path.to_string_lossy()))?
    }

    let commands_file_path = &config_path.join(path::Path::new("commands.json"));

    let commands_file_exists = commands_file_path.try_exists()
        .context(format!("Failed to verify existence of commands config path '{}'", commands_file_path.to_string_lossy()))?;

    if !commands_file_exists {
        let commands_file = fs::File::create(commands_file_path)
            .context("Error initializing commands config file")?;

        // Write initial JSON data
        let commands_file_structure = PhrasesStruct {
            executables: vec!()
        };

        let writer = std::io::BufWriter::new(commands_file);
        serde_json::to_writer(writer, &commands_file_structure)
            .context("Error initializing commands file")?;
    }

    Ok(config_path.to_owned())
}

pub fn load_executables(commands_dir : &path::Path) -> anyhow::Result<Vec<execute::Executable>> {

    let commands_file_path = &commands_dir.join(path::Path::new("commands.json"));

    let commands_file_exists = commands_file_path.try_exists()
        .context(format!("Failed to verify existence of command data path '{}'", commands_file_path.to_string_lossy()))?;

    anyhow::ensure!(commands_file_exists, format!("Commands config directory '{}' does not exist", commands_file_path.to_string_lossy()));

    let command_file_contents = fs::read_to_string(commands_file_path)
        .context("Failed to read contents of commands config file")?;

    let command_data : serde_json::Value = serde_json::from_str(&command_file_contents)
        .context("Failed to parse JSON data from commands.json file")?;

    let root_obj = command_data.as_object()
        .context(String::from("JSON root must be an object"))?;

    let executables_arr = root_obj.get("executables")
        .context(String::from("Expected 'executables' in root JSON object"))?
        .as_array()
        .context(String::from("Expected 'executables' to be array type"))?;

    let mut executables = get_executables(executables_arr);

    // Sort executables so longest phrases are matched first
    executables.sort_by(|a, b| { b.phrase.len().cmp(&a.phrase.len()) });

    Ok(executables)
}

fn get_executables(executables_arr : &[serde_json::Value]) -> Vec<execute::Executable> {

    executables_arr.iter().map(|ex| {
        // Get phrase
        let phrase = ex.get("phrase")
            .ok_or(String::from("Could not find 'phrase' data in an executables array object"))?
            .as_str()
            .ok_or(String::from("'phrase' data in executables array should have been a string type"))?
            .to_lowercase();

        // Get command
        let command = ex.get("command")
            .ok_or(String::from("Could not find 'command' data in an executables array object"))?
            .as_str()
            .ok_or(String::from("'command' data in executables array object should have been a string type"))?
            .to_owned();

        // Get arguments
        let args_arr = ex.get("args")
            .ok_or(String::from("Could not find 'args' data in an executables array object"))?
            .as_array()
            .ok_or(String::from("'args' data in executables array should have been an array type"))?
            .to_owned();

        let mut arg_res : Result<(), String> = Ok(());

        let args = args_arr.iter().map(|arg| {
            if let Some(arg) = arg.as_str() {
                arg.to_owned()
            }
            else {
                arg_res = Err(String::from("Non-string data in arguments list"));
                String::new()
            }
        }).collect::<Vec<String>>();

        arg_res?;

        // Position requirements
        let phrase_position_str = ex.get("phrase_position")
            .ok_or(String::from("Could not find 'phrase_position' data in an executables array object"))?
            .as_str()
            .ok_or(String::from("'phrase_position' data in executables array should have been a string type"))?;

        let phrase_position = match phrase_position_str {
            "any" => execute::PhrasePosition::Any,
            "at_start" => execute::PhrasePosition::Start,
            "match_exact" => execute::PhrasePosition::Exact,
            _ => Err(String::from("Invalid phrase_position type encountered. Should have been 'any', 'at_start', or 'match_exact'"))?
        };

        Ok(execute::Executable {
            phrase,
            command,
            args,
            phrase_position
        })
    })
        .filter(|ex| { // Remove executables that produced errors
            match ex {
                Ok(_) => true,
                Err(e) => {
                    eprintln!("{}", e);
                    false
                }
            }
        })
        .map(|ex : Result<execute::Executable, String>| ex.unwrap()) // Get executables from results
        .collect::<Vec<execute::Executable>>()
}

pub fn add_executable(executables : &mut Vec<execute::Executable>) -> anyhow::Result<()> {
    println!("----- Add a command -----");

    println!("Leave input empty to cancel");

    println!("Phrase:");
    let mut phrase = String::new();
    std::io::stdin().read_line(&mut phrase)?;

    phrase = phrase.trim().to_owned();

    if phrase.is_empty() {
        return Ok(());
    }

    println!("Command:");
    let command_opt = autocomplete::search_fs()?;

    let command = if let Some(command) = command_opt {
        command.trim().to_owned()
    }
    else {
        return Ok(());
    };

    if command.is_empty() {
        return Ok(());
    }

    let mut args : Vec<String> = Vec::new();

    println!("Arguments (Enter one at a time, leave empty to finish):");
    let mut arg_opt = autocomplete::search_fs()?;

    let mut arg = if let Some(arg) = arg_opt {
        arg.trim().to_owned()
    } 
    else {
        return Ok(());
    };

    while !arg.is_empty() {
        args.push(arg.clone());
        arg.clear();

        arg_opt = autocomplete::search_fs()?;

        arg = if let Some(arg) = arg_opt {
            arg.trim().to_owned()
        } 
        else {
            return Ok(());
        };
    }

    println!("When should this command be executed?");
    println!("  [1] When the phrase is said at any point in a sentence\
        \n  [2] When the phrase is said at the start of a sentence\
        \n  [3] When the phrase exactly matches a sentence");
    println!("Your choice (1 - 3):");

    let mut phrase_pos_input = String::new();
    std::io::stdin().read_line(&mut phrase_pos_input)?;

    phrase_pos_input = phrase_pos_input.trim().to_owned();

    if phrase_pos_input.is_empty() {
        return Ok(());
    }

    let i : i32 = phrase_pos_input.parse().unwrap_or(-1);

    anyhow::ensure!((1..=3).contains(&i), "Invalid option");

    let phrase_position = [
        execute::PhrasePosition::Any,
        execute::PhrasePosition::Start,
        execute::PhrasePosition::Exact
    ][(i - 1) as usize].to_owned();

    let executable = execute::Executable {
        phrase,
        command,
        args,
        phrase_position
    };

    execute::validate_executable(&executable)?;

    executables.push(executable);

    Ok(())
}

pub fn remove_executable(executables : &mut Vec<execute::Executable>) -> anyhow::Result<()> {

    anyhow::ensure!(!executables.is_empty(), "No executables to be removed");

    println!("----- Remove a command -----");

    println!("[0] Cancel");
    execute::print_executables(executables);

    let mut index_str = String::new();

    println!("Index to remove: ");
    std::io::stdin().read_line(&mut index_str)?;

    let index : i32 = index_str.trim().parse().unwrap_or(0);

    if index == 0 {
        println!("Canceled");
        return Ok(())
    }

    anyhow::ensure!((1..=executables.len() as i32).contains(&index), "Invalid index");

    println!("Removing executable at index [{}]", index);
    executables.remove((index - 1) as usize);

    Ok(())
}

pub fn write_executables(executables : Vec<execute::Executable>, commands_dir : &path::Path) -> anyhow::Result<()> {
    let commands_file_path = &commands_dir.join(path::Path::new("commands.json"));

    let commands_file_exists = commands_file_path.try_exists()
        .context(format!("Failed to verify existence of commands data path {}",
            commands_file_path.to_string_lossy()))?;

    anyhow::ensure!(commands_file_exists,
        format!("Commands config directory '{}' does not exist",
        commands_file_path.to_string_lossy()));

    // Recreate commands file with updated data
    let commands_file = fs::File::create(commands_file_path)
        .context(format!("Error creating file {}", commands_file_path.to_string_lossy()))?;

    // Create structure of JSON data
    let commands_file_structure = PhrasesStruct {
        executables
    };

    let writer = std::io::BufWriter::new(commands_file);
    serde_json::to_writer_pretty(writer, &commands_file_structure)
        .context("Error saving commands to config file")?;

    Ok(())
}
