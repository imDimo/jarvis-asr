# This Python script will dump spoken text into a text file.

# To add this script to jarvis-asr, run jarvis-asr with the "-a" flag.
# Example input for Linux (Adjust as needed for your system):
# - Phrase: <words...>
# - Command: python3
# - Args:
#   - /path/to/transcribe.py
#   - <words...>
# - Match type: 1

# Now, while running jarvis-asr, a transcription should be generated

import signal
import sys


def main():
    if sys.platform == "linux":
        signal.signal(signal.SIGCHLD, signal.SIG_IGN)

    FILE_NAME = "transcription.txt"

    with open(FILE_NAME, 'a') as file:
        for word in sys.argv[1:]:
            file.write(f"{word} ")
        file.write('\n')


if __name__ == "__main__":
    main()
