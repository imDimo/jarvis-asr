# This Python script demonstrates the extent of the application's
# argument-handling abilities.

# To add this script to jarvis-asr, run jarvis-asr with the "-a" flag.
# Example input for Linux (Adjust as needed for your system):
# - Phrase: the <adj> <adj> <noun> <verb> over the <objects...>
# - Command: python3
# - Args:
#   - /path/to/everything.py
#   - <noun>
#   - <verb>
#   - <adj>
#   - <adj>
#   - <objects...>

# Unique and duplicate arguments can be mixed together, and each set of
# duplicates will be taken from the phrase left-to-right.
# However, a phrase may only have one variable-length (list) argument, and it
# must be placed at the end of the phrase.
# The variable-length argument can be placed anywhere in the args list, but
# it's most convenient to keep it at the end.

# Try saying something like "The quick brown fox jumps over the lazy dog, the
# sleeping cat, and the red porcupine" to see what it produces.
# Like multi_demo.py, this will create a file in your working directory

# (If you are having trouble getting this one working, run with the -p flag to
# verify that it is detecting your voice properly)

import signal
import sys


def main():
    if sys.platform == "linux":
        signal.signal(signal.SIGCHLD, signal.SIG_IGN)

    noun = sys.argv[1]
    verb = sys.argv[2]
    adj1 = sys.argv[3]
    adj2 = sys.argv[4]
    objects = " ".join(sys.argv[5:])

    message = "What {} over the {}? The {}!\nThe {} is {} and {}."
    message = message.format(verb, objects, noun, noun, adj1, adj2)

    with open("multi_output.txt", "w", encoding="utf-8") as file:
        file.write(message)


if __name__ == "__main__":
    main()
