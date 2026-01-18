use std::path::{Path, PathBuf};

fn main() {

    let root = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let root = Path::new(root.as_str());

    #[cfg(target_os = "linux")]
    get_vosk_linux(root);

    #[cfg(target_os = "windows")]
    get_vosk_windows(root);
}

#[cfg(target_os = "linux")]
fn get_vosk_linux(cargo_dir : &Path) {

    let vosk = Path::new("libvosk.so");

    let local_libs = cargo_dir.join(Path::new("./lib/linux"));

    let mut local_vosk = false;

    // Use project-relative VOSK library if possible
    let vosk_path = if local_libs.join(vosk).try_exists().is_ok_and(|exists| exists) {
        local_vosk = true;
        local_libs
    }
    else {
        // Search for system VOSK library
        let system_lib_paths = std::env::var("LD_LIBRARY_PATH")
            .expect("LD_LIBRARY_PATH not set or could not be found. Fix this or include Vosk libraries under /lib/linux/")
            .split(':').map(|s| Path::new(s).to_owned()).collect::<Vec<PathBuf>>();
        system_lib_paths.iter().find(|p| p.join(vosk).try_exists().is_ok_and(|exists| exists))
            .expect("libvosk.so could not be found under /lib/linux/ or in the system library path")
            .to_owned()
    };

    if local_vosk {
        build_print::info!("Using local VOSK library ({})", vosk_path.join(vosk).to_string_lossy());
        build_print::info!("jarvis-asr must be able to find this library after compilation");
    }
    else {
        build_print::info!("Using system VOSK library ({})", vosk_path.join(vosk).to_string_lossy());
    }

    // Convert to string to use as argument
    let vosk_path = vosk_path.to_string_lossy();

    // Add VOSK library path to linker search
    println!("cargo::rustc-link-search={vosk_path}");
}

#[cfg(target_os = "windows")]
fn get_vosk_windows(cargo_dir : &Path) {
    let vosk = Path::new("libvosk.dll");

    let local_libs = cargo_dir.join(Path::new("./lib/windows"));

    let mut local_vosk = false;

    // Use project-relative VOSK library if possible
    let vosk_path = if local_libs.join(vosk).try_exists().is_ok_and(|exists| exists) {
        local_vosk = true;
        local_libs
    }
    else {
        // Search for system VOSK library
        let system_lib_paths = std::env::var("PATH")
            .expect("System PATH not set or could not be found. Fix this or include Vosk libraries under /lib/windows/")
            .split(';').map(|s| Path::new(s).to_owned()).collect::<Vec<PathBuf>>();
        system_lib_paths.iter().find(|p| p.join(vosk).try_exists().is_ok_and(|exists| exists))
            .expect("libvosk.so could not be found under /lib/windows/ or in the system PATH")
            .to_owned()
    };

    if local_vosk {
        build_print::info!("Using local VOSK library ({})", vosk_path.join(vosk).to_string_lossy());
        build_print::info!("jarvis-asr must be able to find this library after compilation");
    }
    else {
        build_print::info!("Using system VOSK library ({})", vosk_path.join(vosk).to_string_lossy());
    }

    // Convert to string to use as argument
    let vosk_path = vosk_path.to_string_lossy();

    // Add VOSK library path to linker search
    println!("cargo::rustc-link-search={vosk_path}");
}
