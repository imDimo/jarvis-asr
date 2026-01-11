# This Python script will search the given cli argument(s) in the browser
# specified below.
# On Linux, you may be able to reference the browser by name, assuming it has
# been installed as a package.
# On Windows, you must provide the path to the executable of your browser, or
# add the directory containing the browser's executable to your system path.

# The URL should include some search engine prefix. This example uses
# DuckDuckGo, but anything works.

# To add this script to jarvis-asr, run jarvis-asr with the "-a" flag.
# Example input for Linux (Adjust as needed for your system):
# - Phrase: jarvis search <query...>
# - Command: python3
# - Args:
#   - /path/to/search.py
#   - <query...>

# Now, while running jarvis-asr, "jarvis search [query]" should bring up the
# search in your specified browser

import subprocess
import signal
import sys


def main():
    BROWSER = ""  # Enter path to browser here
    URL = 'https://duckduckgo.com/' + ' '.join(sys.argv[1:])

    platform = sys.platform

    if platform == "linux":
        signal.signal(signal.SIGCHLD, signal.SIG_IGN)
        _ = subprocess.Popen([BROWSER, URL])

    elif platform == "windows":
        _ = subprocess.Popen([BROWSER, URL])


if __name__ == "__main__":
    main()
