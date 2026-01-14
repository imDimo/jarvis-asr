use std::{
    fs,
    path
};
use crate::execute;

#[derive(serde::Deserialize, serde::Serialize)]
struct PhrasesStruct {
    executables: Vec<execute::Executable>
}

pub fn init_config_directory() -> anyhow::Result<path::PathBuf, String> {
    let project_dirs = directories::ProjectDirs::from("com", "imdimo", "jarvis-asr");

    let config_path = project_dirs
        .ok_or(String::from("Could not obtain location of project config directory. Is your home set?"))?
        .config_dir().to_owned();

    let config_path_exists = config_path.try_exists()
        .map_err(|e| format!("Failed to verify existence of config directory path {} {}", config_path.to_string_lossy(), e))?;

    if !config_path_exists {
        fs::create_dir_all(&config_path)
            .map_err(|e| format!("Failed to create config directory '{}': {}", config_path.to_string_lossy(), e))?
    }

    let phrases_file_path = &config_path.join(path::Path::new("phrases.json"));

    let phrases_file_exists = phrases_file_path.try_exists()
        .map_err(|e| format!("Failed to verify existence of phrase config path {} {}", phrases_file_path.to_string_lossy(), e))?;

    if !phrases_file_exists {
        let phrases_file = fs::File::create(phrases_file_path)
            .map_err(|e| format!("Error initializing phrases file {}", e))?;

        // Write initial JSON data
        let phrases_file_structure = PhrasesStruct {
            executables: vec!()
        };

        let writer = std::io::BufWriter::new(phrases_file);
        serde_json::to_writer(writer, &phrases_file_structure)
            .map_err(|e| format!("Error initializing phrases file {}", e))?;
    }

    Ok(config_path.to_owned())
}

pub fn load_executables(phrase_dir : &path::Path) -> anyhow::Result<Vec<execute::Executable>, String> {

    let phrases_file_path = &phrase_dir.join(path::Path::new("phrases.json"));

    let phrases_file_exists = phrases_file_path.try_exists()
        .map_err(|e| format!("Failed to verify existence of phrase data path {} {}", phrases_file_path.to_string_lossy(), e))?;

    if !phrases_file_exists {
        return Err(format!("Phrases config directory '{}' does not exist", phrases_file_path.to_string_lossy()));
    }

    let phrase_file_contents = fs::read_to_string(phrases_file_path)
        .map_err(|e| format!("Failed to read file contents {}", e))?;

    let phrase_data : serde_json::Value = serde_json::from_str(&phrase_file_contents)
        .map_err(|e| format!("Failed to parse JSON data from file {}", e))?;

    let root_obj = phrase_data.as_object()
        .ok_or(String::from("JSON root must be an object"))?;

    let executables_arr = root_obj.get("executables")
        .ok_or(String::from("Expected 'executables' in root JSON object"))?
        .as_array()
        .ok_or(String::from("Expected 'executables' to be array type"))?;
    
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

pub fn add_executable(executables : &mut Vec<execute::Executable>) -> Result<(), String> {
    println!("----- Add a command -----");

    let mut phrase = String::new();
    let mut command = String::new();
    let mut arg = String::new();
    let mut args : Vec<String> = Vec::new();

    let pop_newlines = |str : &mut String| {
        if str.ends_with('\n') {
            str.pop();
            if str.ends_with('\r') {
                str.pop();
            }
        }
    };
    
    println!("Leave input empty to cancel");

    println!("Phrase:");
    std::io::stdin().read_line(&mut phrase)
        .map_err(|e| e.to_string())?;
    
    pop_newlines(&mut phrase);

    if phrase.is_empty() {
        return Ok(());
    }

    println!("Command:");
    std::io::stdin().read_line(&mut command)
        .map_err(|e| e.to_string())?;

    pop_newlines(&mut command);

    if command.is_empty() {
        return Ok(());
    }

    println!("Arguments (Enter one at a time, leave empty to finish):");
    std::io::stdin().read_line(&mut arg)
        .map_err(|e| e.to_string())?;
    arg = arg.trim().to_owned();
    
    pop_newlines(&mut arg);

    while !arg.is_empty() {
        args.push(arg.clone());
        arg.clear();
        
        std::io::stdin().read_line(&mut arg)
            .map_err(|e| e.to_string())?;
        arg = arg.trim().to_owned();

        pop_newlines(&mut arg);
    }

    println!("In what circumstances should this command be executed?");
    println!("  [1] When the phrase is said at any point in a sentence\
        \n  [2] When the phrase is said at the start of a sentence\
        \n  [3] When the phrase exactly matches the sentence");
    println!("Your choice (1 - 3):");

    let mut phrase_pos_input = String::new();
    std::io::stdin().read_line(&mut phrase_pos_input)
        .map_err(|e| e.to_string())?;
    pop_newlines(&mut phrase_pos_input);

    println!();

    let i : i32 = phrase_pos_input.parse().unwrap_or(-1);

    let phrase_position = if (1..=3).contains(&i) {
        [
            execute::PhrasePosition::Any, 
            execute::PhrasePosition::Start,
            execute::PhrasePosition::Exact
        ][(i - 1) as usize].to_owned()
    }
    else {
        return Err(String::from("Invalid option"));
    };

    let executable = execute::Executable {
        phrase,
        command,
        args,
        phrase_position
    };

    executables.push(executable);

    Ok(())
}

pub fn remove_executable(executables : &mut Vec<execute::Executable>) -> Result<(), String> {
    
    if executables.is_empty() {
        return Err(String::from("No executables to be removed"));
    }

    println!("----- Remove a command -----");

    execute::print_executables(executables);   

    let mut index_str = String::new();

    println!("Index to remove: ");
    std::io::stdin().read_line(&mut index_str)
        .map_err(|e| e.to_string())?;

    let index : i32 = index_str.trim().parse().unwrap_or(-1);

    if (1..=executables.len() as i32).contains(&index) {
        println!("Removing executable at index [{}]", index);
        executables.remove((index - 1) as usize);
    }
    else {
        return Err(String::from("Invalid index"));
    }

    Ok(())
}

pub fn write_executables(executables : Vec<execute::Executable>, phrases_dir : &path::Path) -> Result<(), String> {
    let phrases_file_path = &phrases_dir.join(path::Path::new("phrases.json"));

    let phrases_file_exists = if let Ok(exists) = phrases_file_path.try_exists() {
        exists
    }
    else {
        return Err(format!("Failed to verify existence of phrase data path {}", phrases_file_path.to_string_lossy()));
    };

    if !phrases_file_exists {
        return Err(format!("Phrases config directory '{}' does not exist", phrases_file_path.to_string_lossy()));
    }
    
    // Recreate phrases file with updated data
    let phrases_file = fs::File::create(phrases_file_path)
        .map_err(|e| format!("Error creating file {}: {}", phrases_file_path.to_string_lossy(), e))?;

    // Create structure of JSON data
    let phrases_file_structure = PhrasesStruct {
        executables
    };

    let writer = std::io::BufWriter::new(phrases_file);
    serde_json::to_writer_pretty(writer, &phrases_file_structure)
        .map_err(|e| format!("Error writing phrases to file: {}", e))?;

    Ok(())
}
