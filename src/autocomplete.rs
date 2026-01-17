use std::sync::mpsc;

use anyhow::Context;

pub fn search_fs() -> anyhow::Result<String> {

    let (sender, receiver) = mpsc::channel();

    let t = std::thread::spawn(move || {
        let mut working_path = std::path::PathBuf::new();

        loop {
            let key_res = console_utils::read::read_key();
            if let Ok(key) = key_res {
                match key {
                    console_utils::read::Key::Tab => {

                        working_path = fs_autocomplete(working_path);

                        console_utils::control::clear_line();
                        print!("{}", working_path.to_str().unwrap());
                        console_utils::control::flush();
                    },
                    console_utils::read::Key::Backspace => {
                        let mut working_str = working_path.to_str().unwrap().to_owned();
                        working_str.pop();

                        console_utils::control::clear_line();
                        print!("{}", working_str);
                        console_utils::control::flush();

                        working_path = std::path::PathBuf::from(working_str);
                    },
                    console_utils::read::Key::Char(c) => {
                        let mut working_str = working_path.to_str().unwrap().to_owned();
                        working_str.push(c);

                        console_utils::control::clear_line();
                        print!("{}", working_str);
                        console_utils::control::flush();

                        working_path = std::path::PathBuf::from(working_str);
                    },
                    console_utils::read::Key::Enter => {
                        sender.send(working_path.to_string_lossy().to_string()).ok();
                        println!();
                        break;
                    },
                    _ => {}
                }
            }
        }
    });

    let data = receiver.recv()
        .context("Error retrieving command data")?;

    drop(receiver);
    t.join().expect("Error closing thread");

    Ok(data)
}

fn fs_autocomplete(mut working_path : std::path::PathBuf) -> std::path::PathBuf {
    let curr_search = if !working_path.is_dir() && let Some(search) = working_path.iter().next_back() {
        search.to_str().unwrap().to_owned()
    }
    else {
        String::new()
    };

    let current_dir = get_current_dir(&working_path);

    let paths_res = std::fs::read_dir(&current_dir);
    if let Ok(paths) = paths_res {
        let dir_entries = paths.filter_map(|p| {
            p.ok()
        }).collect::<Vec<std::fs::DirEntry>>();

        let file_query = get_hints(&working_path, &current_dir, &dir_entries);

        // If there is only one potential file/directory, replace the current partial search with the complete path
        if file_query.len() == 1 {
            if working_path.file_name().is_some() {
                working_path.pop();
            }

            let target_loc = &file_query[0].to_owned();
            working_path.push(target_loc);   
        }
        else {
            println!();

            print_hints(&file_query);

            if !curr_search.is_empty() {
                let file_strings = file_query.iter().map(|path| path.iter().next_back()
                    .unwrap().to_str().unwrap().to_owned()).collect::<Vec<String>>();

                modify_search(&mut working_path, &curr_search, &file_strings);
            }
        }
    }

    if working_path.is_dir() {
        let mut dir_osstr = working_path.as_os_str().to_owned();
        if !dir_osstr.to_string_lossy().ends_with("/") {
            dir_osstr.push("/");
            working_path = dir_osstr.into();
        }
    }

    working_path
}

fn get_current_dir(working_path : &std::path::PathBuf) -> std::path::PathBuf {
    if working_path.exists() {
        working_path.to_owned()
    }
    else if let Some(dir) = working_path.parent() {
        dir.to_owned()
    }
    else {
        std::path::PathBuf::from("/")
    }
}

fn get_hints(working_path : &std::path::PathBuf, current_dir : &std::path::PathBuf, dir_entries : &[std::fs::DirEntry]) -> Vec<std::path::PathBuf> {
    if working_path != current_dir && let Some(filename) = working_path.file_name() {
        dir_entries.iter().filter_map(|path| { 
            if path.file_name().to_string_lossy().starts_with(filename.to_str().unwrap())  {
                Some(path.path())
            }
            else {
                None
            }
        }).collect::<Vec<std::path::PathBuf>>()
    }
    else {
        dir_entries.iter().map(|d| d.path()).collect::<Vec<std::path::PathBuf>>()
    }
}

fn print_hints(file_query : &Vec<std::path::PathBuf>) { 
    let hint_text_color = console_utils::styled::Color::Cyan;
    for path in file_query {
        let text = path.file_name().unwrap().to_str().unwrap();
        let styled_text = console_utils::styled::StyledText::new(text)
            .fg(hint_text_color);

        let output = styled_text.format_sequence();
        println!("{}", &output);
    };
}

fn modify_search(working_path : &mut std::path::PathBuf, curr_search : &str, file_strings : &[String]) {
    if file_strings.len() > 1 {
        let max_len_query = file_strings.iter().map(|s| s.len()).min();
        if let Some(max_len) = max_len_query {
            let mut char_table : Vec<Vec<char>> = vec!();

            file_strings.iter().for_each(|str| {
                let chars = str.chars().collect::<Vec<char>>();
                char_table.push(chars);
            });

            let mut matches = true;
            let mut matched = curr_search.to_owned();

            for char_index in curr_search.len()..max_len {
                let curr_char = &char_table[0][char_index];
                for string_chars in &char_table {
                    if &string_chars[char_index] != curr_char {
                        matches = false;
                        break;
                    }
                }

                if !matches {
                    break;
                }

                matched.push(*curr_char);
            }

            if !matched.is_empty() {
                working_path.pop();
                working_path.push(matched);
            }
        }
    }
}
