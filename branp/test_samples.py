# Copyright (c) Brandon Pacewic
# SPDX-License-Identifier: MIT

import os
import sys
import subprocess

from optparse import Values
from subprocess import Popen, PIPE
from typing import List

from branp.command import Command
from branp.dbrun import DbrunCommand
from branp.util import GREEN, RED, ENDC


class TestSamplesCommand(Command):
    """
    Run competitive programming sample test cases.
    """

    usage = """
      %prog test_samples [options] <file>"""

    def run(self, options: Values, args: List[str]) -> None:
        if not len(args) or len(args) > 1:
            print("Expected 1 argument, executable to run test cases on.")
            sys.exit(1)

        file = args[0]

        if not file.endswith(".cpp"):
            file += ".cpp"

        if file not in os.listdir():
            print(f"{file} not found.")
            sys.exit(1)

        test_input_fname = f'{file.split(".")[0]}_input.txt'
        expected_output_fname = f'{file.split(".")[0]}_output.txt'

        with open(test_input_fname, "r", encoding="utf-8") as input_file:
            test_input = input_file.readlines()

        with open(expected_output_fname, "r", encoding="utf-8") as output_file:
            expected_output = output_file.readlines()

        subprocess.call(DbrunCommand.compile_command.split() + [file], bufsize=1, shell=False)

        with Popen("./a.out", stdin=PIPE, stdout=PIPE, stderr=PIPE) as program:
            for line in test_input:
                program.stdin.write(line.encode("utf-8"))

            program.stdin.flush()
            program.stdin.close()

            program_output = [line.decode("utf-8") for line in program.stdout.readlines()]

        # TODO: Add timing
        print(f"Comparing...\nCompile time: xxx\nRun time: xxx", end='')

        good_count = 0
        total_count = len(expected_output)
        mismatches = []

        for i, (actual, expected) in enumerate(zip(program_output, expected_output)):
            actual = actual.strip()
            expected = expected.strip()

            if actual == expected:
                good_count += 1
            else:
                mismatches.append((i + 1, actual, expected))

        print("\n--------------\nExpected:")

        for i, line in enumerate(expected_output):
            print(line.strip(), end='\n' if i != len(expected_output) - 1 else '')

        print("\n--------------\nActual:")

        for i, line in enumerate(program_output):
            print(line.strip(), end='\n' if i != len(program_output) - 1 else '')

        print("\n--------------")

        if len(mismatches):
            print("Mismatches:")

        for mismatch in mismatches:
            print(f'Line {mismatch[2]}: {RED}{mismatch[0]}{ENDC} != {GREEN}{mismatch[1]}{ENDC}')

        if good_count == len(expected_output):
            print(f'{GREEN}All tests passed!{ENDC}')
        elif good_count >= 1:
            print(f'{GREEN}{good_count} / {total_count} tests passed.{ENDC}')
        else:
            print(f'{RED}No tests passed.{ENDC}')
