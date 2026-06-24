use std::{
    path,
    sync::mpsc,
    thread::{self, JoinHandle}
};
use anyhow::Context;
use vosk::{DecodingState, Model, Recognizer};

use crate::input;

pub const RECOGNIZER_MAX_ALTERNATIVES : u16 = 0;
pub const DATA_CHUNKS : usize = 100;

pub type AsrReceiverResult = mpsc::Receiver<anyhow::Result<String>>;

pub fn run_asr(model_path : &path::Path, cpal_receiver : input::CpalReceiverResult, sample_rate : u32, 
    print_results : bool) -> anyhow::Result<(AsrReceiverResult, JoinHandle<()>)> {

    // Sender and receiver to communicate text data
    let (sender, receiver) = mpsc::channel();

    let model_path_str = model_path.to_string_lossy();

    let model = Model::new(model_path_str.clone())
        .context(format!("Failed to create VOSK model from path {}", &model_path_str))?;

    let mut recognizer = Recognizer::new(&model, sample_rate as f32)
        .context("Error occurred while creating VOSK recognizer")?;

    recognizer.set_max_alternatives(RECOGNIZER_MAX_ALTERNATIVES);
    // recognizer.set_words(true);
    // recognizer.set_partial_words(true);

    // Start sending and processing stream data
    eprintln!("\nRunning ASR");
    eprintln!("CTRL + C to exit");

    let asr_thread = thread::spawn(move || {
        loop {
            if let Ok(data) = cpal_receiver.recv() {
                match data {
                    Ok(data) => {
                        let mut state = DecodingState::Running;

                        for sample in data.chunks(DATA_CHUNKS) {
                            state = recognizer.accept_waveform(sample).unwrap_or(DecodingState::Failed);
                        }

                        match state {
                            DecodingState::Running => {},
                            DecodingState::Finalized => { 

                                if let Some(result) = recognizer.final_result().single() {
                                    let text = result.text.trim();
                                    if !text.is_empty() {
                                        if print_results {
                                            println!("{}", text);
                                        }

                                        sender.send(Ok(text.to_string())).ok();
                                    }
                                }
                            },
                            DecodingState::Failed => {
                                eprintln!("Failed to decode!"); 
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
                eprintln!("ASR thread exited!");
                return;
            }
        }
    });

    Ok((receiver, asr_thread))
}

