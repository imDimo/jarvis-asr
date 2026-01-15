use std::{
    sync::{Arc, atomic::{AtomicBool, Ordering}, mpsc::Receiver}
};
use anyhow::Context;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

pub fn get_cpal_default_input_device() -> anyhow::Result<cpal::Device> {
    cpal::default_host().default_input_device()
        .context(String::from("No default input device available"))
}

pub type CpalReceiverResult = Receiver<anyhow::Result<Vec<i16>>>;

pub fn init_cpal_input_stream(input_dev : cpal::Device,
    is_running : Arc<AtomicBool>) -> anyhow::Result<(cpal::Stream, u32, Option<CpalReceiverResult>)> {

    eprintln!("Using input device: {}\n", input_dev.description().unwrap().name());
    
    // Sender and receiver to communicate audio data
    let (sender, receiver) = std::sync::mpsc::channel();
    let err_sender = sender.clone();

    let config = input_dev.default_input_config()
    .context("Error getting default input device configuration")?;

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
            // TODO: Turn e into an anyhow error
            err_sender.send(Err(e.into())).ok(); 
        }, None);

    let stream = stream_res
        .context("Error creating cpal input stream")?;

    stream.play()
    .context("Error starting clap input stream")?;

    Ok((stream, sample_rate, Some(receiver)))
}


pub fn get_cpal_input_devices() -> anyhow::Result<Vec<cpal::Device>> {

    let cpal_host = cpal::default_host();
    let input_devices = cpal_host.input_devices()
        .context("Error querying input devices")?
        .collect::<Vec<cpal::Device>>();

    eprintln!("Got available devices\n");
    Ok(input_devices)
}

pub fn print_cpal_device_descriptions(input_devices : &[cpal::Device]) -> anyhow::Result<()> {
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
        anyhow::bail!("One or more input devices could not retrieve a device description");
    }

    let default_description = cpal::default_host().default_input_device()
        .context("No default input device provided")?
        .description()
        .context("Error getting default device description")?;

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
