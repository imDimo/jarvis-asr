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

Because this project links to VOSK's shared libraries, the libraries must be present for both compiling and running the application. The libraries located in `/lib` will be used for compilation, and you should replace these if you wish to provide your own. If the libraries used in the compilation step are not available system-wide, they must also be copied into the directory where the binary is located for the application to run.

For Windows users, `libvosk.lib` is only used during compilation as is not required for running the application.

Build the program with `cargo build --release`

The binary will be placed in `/target/release/`

## Running

For normal usage, a path to a VOSK model must be provided, either through a command-line argument `jarvis-asr -m </path/to/model/>` or by setting the environment variable `VOSK_MODEL_PATH`.

A configuration file `commands.json` maps user-defined phrases to commands and their arguments. This file is located in:

- Windows: `~/AppData/Roaming/imdimo/jarvis-asr/config/`

- Linux: `~/.config/jarvis-asr/`

Jarvis-ASR can be run with the `-a` or `-r` flag to add or remove these mappings through the terminal, but it may be more convenient to edit `commands.json` by hand. As long as the JSON is properly formatted, improper executables will simply generate warnings and be ignored.

Run `jarvis-asr --help` to see all available arguments.

Lastly, check out the python scripts in `/examples` to get a feel for how this program may be used.
