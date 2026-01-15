use std::{
    sync::mpsc,
    thread::{self, JoinHandle}
};

use crate::{asr_handler as asr, execute};

#[derive(Debug, PartialEq, Eq)]
pub enum ArgumentType {
    Default, Wildcard, List
}

pub struct PhraseArgs {
    pub wildcard_args : Vec<(String, String)>,
    pub list_args : Option<(String, Vec<String>)>
}

pub type PhraseMatchReceiver = mpsc::Receiver<anyhow::Result<(usize, PhraseArgs)>>;

pub fn run_phrase_matcher(asr_receiver : asr::AsrReceiverResult, 
    executables : Vec<execute::Executable>) -> anyhow::Result<(PhraseMatchReceiver, JoinHandle<()>)> {
    let (sender, receiver) = mpsc::channel();

    let matcher_thread = thread::spawn(move || {
        loop {
            if let Ok(data) = asr_receiver.recv() {
                match data {
                    Ok(data) => {
                        // Wildcard arguments given in the phrase
                        let mut phrase_args = PhraseArgs {
                            wildcard_args : vec!(),
                            list_args : None
                        };

                        // Find a matching phrase and get any wildcard arguments
                        let phrase_match_result = executables.iter().enumerate().find(|(_, ex)| {
                            let (phrase_matches, temp_phrase_args) = match_phrase(&data, ex);

                            if phrase_matches {
                               phrase_args = temp_phrase_args;
                            }

                            phrase_matches
                        });

                        // On phrase match, pass index and arguments to execution thread
                        if let Some((i, _)) = &phrase_match_result {
                            sender.send(Ok((*i, phrase_args))).ok();
                        }
                    },
                    Err(e) => {
                        sender.send(Err(e)).ok();
                        return;
                    }
                }
            }
            else { // Senders have dropped
                eprintln!("Phrase match thread exited!");
                return;
            }
        }
    });

    Ok((receiver, matcher_thread))
}

fn match_phrase(input : &str, executable : &execute::Executable) -> (bool, PhraseArgs) {
    let mut phrase_args = PhraseArgs {
        wildcard_args : vec!(),
        list_args : None
    };

    let phrase_parts = executable.phrase.split(' ').map(String::from).collect::<Vec<String>>();
    let words = input.split(' ').map(String::from).collect::<Vec<String>>();
    
    // For exact matches, phrase word length should be equal to sentence word length, unless the
    // phrase uses a wildcard, in which case Exact will behave like Start
    if executable.phrase_position == execute::PhrasePosition::Exact 
    && phrase_parts.len() != words.len() && !executable.phrase.ends_with("...>") {
        return (false, phrase_args);
    }

    // Get points in the sentence where the phrase could start
    let start_points = get_start_points(&words, &phrase_parts, &executable.phrase_position);

    if start_points.is_empty() {
        return (false, phrase_args);
    }

    let mut phrase_matches = true;

    for start in start_points {
        phrase_args.wildcard_args.clear();
        phrase_args.list_args = None;
        phrase_matches = true;
        
        // Stop if there are not enough words left to match the phrase
        if phrase_parts.len() > words.len() - start {
            return (false, phrase_args);
        }

        // Match spoken words to wildcard/list arguments
        for i in 0..phrase_parts.len() {
            if words[start + i] != phrase_parts[i] {
                let arg_type = arg_type(&phrase_parts[i]);

                if arg_type == ArgumentType::Wildcard {
                    phrase_args.wildcard_args.push((phrase_parts[i].clone(), words[start + i].clone()));
                }
                else if arg_type == ArgumentType::List {
                    phrase_args.list_args = Some((phrase_parts[i].clone(), words[(start + i)..].to_vec()));
                    return (phrase_matches, phrase_args);
                }
                else {
                    phrase_matches = false;
                    break;
                }
            }

            // Return if fully matched
            if i == phrase_parts.len() - 1 {
                return (phrase_matches, phrase_args);
            }
        }
    }

    (phrase_matches, phrase_args)
}

// Find indices of spoken words that may match the start of the given phrase
fn get_start_points(words : &[String], phrase_parts : &[String], phrase_pos : &execute::PhrasePosition) -> Vec<usize> {
    if *phrase_pos == execute::PhrasePosition::Any {
        words.iter().enumerate().filter_map(|(i, word)| { 
            if **word == phrase_parts[0] {
                Some(i)
            }
            else {
                None
            }
        }).collect::<Vec<usize>>()
    }
    else {
        // We only need to check the first word
        if let Some(start) = words.first() && *start == phrase_parts[0] {
            vec!(0)
        }
        else {
            vec!()
        }
    }
}

pub fn arg_type(arg : &str) -> ArgumentType {
    if arg.starts_with('<') {
        if arg.ends_with("...>") {
            return ArgumentType::List;
        }
        else if arg.ends_with('>') {
            return ArgumentType::Wildcard;
        }
    }

    ArgumentType::Default
}
