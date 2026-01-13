# This Python script demonstrates the application's ability to match and send
# multiple arguments

# To add this script to jarvis-asr, run jarvis-asr with the "-a" flag.
# Example input for Linux (Adjust as needed for your system):
# - Phrase: I wish I had <num> <color> cats
# - Command: python3
# - Args:
#   - /path/to/multi_demo.py
#   - <num>
#   - <color>
# - Match type: 1

# Note that argument names are not required to be unique; you could replace
# both <num> and <color> with <arg> and it will run the same.
# However, if you want the arguments from the phrase to be applied in a certain
# order, then unique names are required.
# See everything.py for an example that makes use of argument re-ordering

# Now, while running jarvis-asr, "I wish I had [num] [color] cats" should
# create a file in your working directory using those arguments

import signal
import sys


def main():
    if sys.platform == "linux":
        signal.signal(signal.SIGCHLD, signal.SIG_IGN)

    num = sys.argv[1]
    color = sys.argv[2]
    message = "I will give you {} {} cats\n".format(num, color)

    with open("multi_output.txt", "w", encoding="utf-8") as file:
        file.write(message)


if __name__ == "__main__":
    main()
