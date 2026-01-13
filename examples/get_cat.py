# This Python script will display a cat in the browser specified below
# On Linux, you may be able to reference the browser by name, assuming it has
# been installed as a package.
# On Windows, you must provide the path to the executable of your browser, or
# add the directory containing the browser's executable to your system path.

# To add this script to jarvis-asr, run jarvis-asr with the "-a" flag.
# Example input for Linux (Adjust as needed for your system):
# - Phrase: jarvis give me a cat
# - Command: python3
# - Args:
#   - /path/to/get_cat.py
# - Match type: 1

# Now, while running jarvis-asr, "jarvis give me a cat" should display a cat

import subprocess
import signal
import sys


def main():
    if sys.platform == "linux":
        signal.signal(signal.SIGCHLD, signal.SIG_IGN)

    BROWSER = ""  # Enter path to browser here
    URL = "https://cataas.com/cat"

    _ = subprocess.Popen([BROWSER, URL])


if __name__ == "__main__":
    main()
