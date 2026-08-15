use std::io::Write;
use anyhow::Context;

pub fn get_path() -> anyhow::Result<Option<String>> {

    if let Err(e) = crossterm::terminal::enable_raw_mode() {
        return anyhow::Result::Err(e).context("Error entering raw terminal");
    }

    let result = search_fs();
    
    if let Err(e) = crossterm::terminal::disable_raw_mode() {
        return anyhow::Result::Err(e).context("Error exiting raw terminal");
    }

    println!();

    Ok(result)
}

fn search_fs() -> Option<String> {

    let mut working_path = std::path::PathBuf::new();
    let search_result : Option<String>;
    
    loop {
        if let Ok(event) = crossterm::event::read()
        && let Some(key_event) = event.as_key_event()
        && key_event.is_press() {
            match key_event.code {
                crossterm::event::KeyCode::Tab => {
                    working_path = fs_autocomplete(working_path);

                    let _clear_res = crossterm::queue!(std::io::stdout(),
                        crossterm::terminal::Clear(crossterm::terminal::ClearType::CurrentLine),
                        crossterm::cursor::MoveToColumn(0)
                    );

                    let _out_res = std::io::stdout().write_all(working_path.to_str().unwrap().as_bytes());
                    let _flush_res = std::io::stdout().flush();
                },
                crossterm::event::KeyCode::Backspace => {
                    let mut working_str = working_path.to_str().unwrap().to_owned();
                    working_str.pop();

                    let _clear_res = crossterm::queue!(std::io::stdout(),
                        crossterm::terminal::Clear(crossterm::terminal::ClearType::CurrentLine),
                        crossterm::cursor::MoveToColumn(0)
                    );

                    let _out_res = std::io::stdout().write_all(working_str.as_bytes());
                    let _flush_res = std::io::stdout().flush();

                    working_path = std::path::PathBuf::from(working_str);
                },
                crossterm::event::KeyCode::Char(c) => {
                    if c == 'c' && key_event.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) {
                        search_result = None;
                        break;
                    }

                    let mut working_str = working_path.to_str().unwrap().to_owned();
                    working_str.push(c);

                    let _clear_res = crossterm::queue!(std::io::stdout(),
                        crossterm::terminal::Clear(crossterm::terminal::ClearType::CurrentLine),
                        crossterm::cursor::MoveToColumn(0)
                    );

                    let _out_res = std::io::stdout().write_all(working_str.as_bytes());
                    let _flush_res = std::io::stdout().flush();

                    working_path = std::path::PathBuf::from(working_str);
                },
                crossterm::event::KeyCode::Enter => {

                    search_result = Some(working_path.to_string_lossy().to_string());
                    break;
                },
                _ => {}
            }
        }
    }

    search_result
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
            let _out_res = std::io::stdout().write_all("\n".as_bytes());

            print_hints(&file_query);

            if !curr_search.is_empty() {
                let file_strings = file_query.iter().map(|path| path.iter().next_back()
                    .unwrap().to_str().unwrap().to_owned()).collect::<Vec<String>>();

                modify_search(&mut working_path, &curr_search, &file_strings);
            }
        }
    }

    // Add trailing slash to autocompleted directories
    if working_path.is_dir() {
        let mut dir_osstr = working_path.as_os_str().to_owned();

        if !dir_osstr.to_string_lossy().ends_with(std::path::MAIN_SEPARATOR) {
            dir_osstr.push(std::path::MAIN_SEPARATOR_STR);
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
        std::path::PathBuf::new()
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
    let style_res = crossterm::queue!(
        std::io::stdout(),
        crossterm::style::SetForegroundColor(crossterm::style::Color::Cyan)
    );

    if style_res.is_ok() {
        for path in file_query {
            let _move_res = crossterm::queue!(
            std::io::stdout(),
            crossterm::cursor::MoveToColumn(0)
            );
            let text = path.file_name().unwrap().to_str().unwrap();
            let _out_res = std::io::stdout().write_all(text.as_bytes());
            let _out_res = std::io::stdout().write("\n".as_bytes());
        };
    }
    else {
        return;
    }

    let _style_res = crossterm::queue!(
        std::io::stdout(),
        crossterm::style::ResetColor
    );
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
