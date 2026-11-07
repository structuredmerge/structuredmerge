#!/usr/bin/env python3
"""Run a command, retrying with registry backoff only while a registry lags.

    retry_with_backoff.py --retry-on REGEX -- cargo install smorg --version 0.2.1

The command is retried (kettle-dev availability-probe schedule, see
check_pypi_published.Schedule) only when its combined output matches REGEX, which
should describe "the thing I just published is not visible yet". Any other
failure, such as a compile error, exits immediately with the command's status.
"""

import argparse
import os
import re
import subprocess
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from check_pypi_published import FailureError, RetryError, Schedule  # noqa: E402


def run_with_backoff(command, retry_on, schedule, run=subprocess.run):
    """Run one command with retries limited to matching registry-lag output."""
    pattern = re.compile(retry_on)
    final = {}

    def step():
        """Handle step."""
        completed = run(command, capture_output=True, text=True, check=False)
        output = (completed.stdout or "") + (completed.stderr or "")
        print(output, end="", flush=True)
        if completed.returncode == 0:
            return 0
        if not pattern.search(output):
            final["code"] = completed.returncode
            return completed.returncode
        raise RetryError(f"exit {completed.returncode}; registry has not caught up (matched {retry_on!r})")

    try:
        return schedule.retry("Running " + " ".join(command[:3]), step)
    except FailureError as error:
        print(f"retry_with_backoff: {error}", file=sys.stderr)
        return 1


def main(argv=None):
    """Parse arguments and run the requested command."""
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--retry-on", required=True, help="regex of output that means the registry is lagging")
    parser.add_argument("command", nargs=argparse.REMAINDER)
    args = parser.parse_args(argv)
    command = args.command[1:] if args.command[:1] == ["--"] else args.command
    if not command:
        parser.error("a command is required after --")
    return run_with_backoff(command, args.retry_on, Schedule())


if __name__ == "__main__":
    sys.exit(main())
