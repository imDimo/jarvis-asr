# Jarvis-ASR

Fully configurable voice command application using [VOSK](https://alphacephei.com/vosk/)

Capabilities:

- Match user-defined phrases to run any application
- Wildcard and variable-length arguments allow dynamic command processing
- Easy command configuration
- Compatible with any VOSK model

## Dependencies

VOSK provides shared libraries for Linux and Windows systems. Generic copies of these libraries are included in this project's `/lib` directory, but these may lack hardware-dependent features (ex. CUDA).

## VOSK Models

A VOSK model is required to perform automated speech recognition. There are several available on [their website](https://alphacephei.com/vosk/models).

Larger and more complex models will take noticeably longer to initialize, and can require very large amounts of RAM. Try a smaller model first to see if its accuracy is suitable.

## Building

Pre-built executables may be uploaded to the Releases category, but do not expect them to be up-to-date.

This project is written in [Rust](https://rust-lang.org/tools/install/), and can be built with [Cargo](https://doc.rust-lang.org/cargo/).

For Linux builds, this project also depends on `libasound2-dev` (Debian/Ubuntu/etc) or `alsa-lib-devel` (Fedora/etc).

Because this project links to VOSK's shared libraries, these libraries must be present for both compiling and running the application. The libraries located under `/lib` will be used for compilation, and you should remove/replace these if you wish to provide your own. If the libraries used in the compilation step are not available system-wide, they must also be copied into the directory where the binary is located in order for the application to run.

For Windows users, `libvosk.lib` is only used during compilation and is not required for running the application.

Build the program with `cargo build --release`

The binary will be placed under `/target/release/`

## Running

For normal usage, a path to a VOSK model must be provided, either through a command-line argument `jarvis-asr -m </path/to/model/>` or by setting the environment variable `VOSK_MODEL_PATH`.

A configuration file `commands.json` maps user-defined phrases to commands and their arguments. This file is located in:

- Windows: `~/AppData/Roaming/imdimo/jarvis-asr/config/`

- Linux: `~/.config/jarvis-asr/`

## Creating/Editing Commands

Jarvis-ASR can be run with the `-a` or `-r` flag to add or remove phrase-command mappings through the terminal, but it may be more convenient to make changes in a text editor. As long as the JSON is properly formatted, invalid entries will simply generate warnings and be ignored.

The `-C` or `--commands` argument can be used to specify a different path for the commands file, which may be useful for maining multiple command sets.

- To use a different file within the default config directory: `jarvis-asr -C some_commands.json` or `jarvis-asr -C <relative/path/to/some_commands.json>`

- To use a file outside of the default config directory: `jarvis-asr -C </absolute/path/to/some_other_commands.json>`

## Other Help

Run `jarvis-asr --help` to see all available arguments.

Lastly, check out the python scripts in `/examples` to get a feel for how this program may be used.
