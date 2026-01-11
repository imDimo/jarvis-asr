# This Python script opens any application passed to it as a cli argument.
# To make this work on Windows, you may have to add the directories of the
# applications you wish to run to your system path.
# Alternatively, you could run the argument through an if-elif/switch statement
# and provide the full path to the desired executable.
# (ex. map [app name] to "C:\Program Files\...\[app name].exe")

# To add this script to jarvis-asr, run jarvis-asr with the "-a" flag.
# Example input for Linux (Adjust as needed for your system):
# - Phrase: jarvis run <app>
# - Command: python3
# - Args:
#   - /path/to/open_app.py
#   - <app>

# Now, while running jarvis-asr, "jarvis run [app]" should open the application

import subprocess
import signal
import sys


def main():
    platform = sys.platform

    if platform == "linux":
        signal.signal(signal.SIGCHLD, signal.SIG_IGN)
        _ = subprocess.Popen(sys.argv[1:])

    elif platform == "windows":
        _ = subprocess.Popen(sys.argv[1:])


if __name__ == "__main__":
    main()
