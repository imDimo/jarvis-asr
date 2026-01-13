use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::{sync::{Arc, atomic::{AtomicBool, Ordering}, mpsc::Receiver}};

pub fn get_cpal_default_input_device() -> Result<cpal::Device, String> {
    cpal::default_host().default_input_device().ok_or(String::from("No default input device available"))
}

pub type CpalReceiverResult = Option<Receiver<anyhow::Result<Vec<i16>, String>>>;

pub fn init_cpal_input_stream(input_dev : cpal::Device, is_running : Arc<AtomicBool>) -> Result<(cpal::Stream, u32, CpalReceiverResult), String> {

    eprintln!("Using input device: {}\n", input_dev.description().unwrap().name());
    
    // Sender and receiver to communicate audio data
    let (sender, receiver) = std::sync::mpsc::channel();
    let err_sender = sender.clone();

    // let mut config_range = input_dev.supported_input_configs().map_err(|e| e.to_string())?;
    // let config = config_range.find(|conf| conf.channels() == 1 && conf.max_sample_rate() >= (SAMPLE_RATE as u32))
    //    .ok_or(String::from("No config found with 1 channel and at least 16k sample rate"))?
    //    .with_sample_rate(SAMPLE_RATE as u32);

    // config_range.clone().into_iter().for_each(|conf| eprintln!("{:?}", conf));

    let config = input_dev.default_input_config()
    .map_err(|e| format!("Error getting default input device configuration: {}", e))?;

    let num_channels = config.channels();
    let sample_rate = config.sample_rate();

    let stream_res = input_dev.build_input_stream(&config.into(),
        move |data : &[i16], _: &_| {
            if is_running.load(Ordering::Relaxed) {
                let mut c : i32 = -1;
                let mut data = data.to_vec();
                
                data.retain(|_| {
                    c += 1;
                    // Read audio from only one channel
                    (c as u16).is_multiple_of(num_channels)
                });
                sender.send(Ok(data)).ok();
            }
        }, 
        move |e| { 
            err_sender.send(Err(e.to_string())).ok(); 
        }, None);

    let stream = stream_res.map_err(|e| format!("Error creating cpal input stream {}", e))?;
    stream.play()
    .map_err(|e| format!("Error starting clap input stream {}", e))?;

    Ok((stream, sample_rate, Some(receiver)))
}


pub fn get_cpal_input_devices() -> anyhow::Result<Vec<cpal::Device>, String> {

    let cpal_host = cpal::default_host();
    let input_devices = cpal_host.input_devices().map_err(|e| format!("Error querying input devices {}", e))?
        .collect::<Vec<cpal::Device>>();

    eprintln!("Got available devices\n");
    Ok(input_devices)
}

pub fn print_cpal_device_descriptions(input_devices : &[cpal::Device]) -> anyhow::Result<(), String> {
    let mut desc_errs : Vec<String> = vec!();
    let descriptions = input_devices.iter().filter_map( |d| {
        match d.description() {
            Ok(desc) => Some(desc),
            Err(e) => {
                desc_errs.push(e.to_string());
                None
            }
        }
    }).collect::<Vec<_>>();
    
    if !desc_errs.is_empty() {
        desc_errs.iter().for_each(|e| eprintln!("Error getting device description: {}", e));
        Err(String::from("One or more input devices could not retrieve a device description"))?;
    }

    let default_description = cpal::default_host().default_input_device().unwrap()
        .description()
        .map_err(|e| format!("Error getting default device description: {}", e))?;

    println!("0: {:?}", default_description.name());

    descriptions.iter().enumerate().for_each(|(i, desc)| {
        println!("{:?}: {:?}", i + 1, desc.name());
        let all = desc.extended();

        all.iter().skip(1).for_each(|e| {
            println!("\t{:?}", e);
        });
    });

    Ok(())
}
