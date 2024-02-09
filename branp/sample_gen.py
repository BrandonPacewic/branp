# Copyright (c) Brandon Pacewic
# SPDX-License-Identifier: MIT

import os
import sys
import subprocess

from optparse import Values
from typing import List

from branp.command import Command


class SampleGenCommand(Command):
    """
    Generate sample test cases for competitive programming problems.
    """

    usage = """
      %prog sample_gen [options] <file>"""

    def run(self, options: Values, args: List[str]) -> None:
        if not len(args) or len(args) > 1:
            print("Expected 1 argument, file to generate samples for.")
            sys.exit(1)

        file = args[0]

        if file.endswith(".cpp"):
            file = file[:-4]

        test_input = f'{file}_input.txt'
        test_output = f'{file}_output.txt'

        subprocess.call(["touch", test_input], bufsize=1, shell=False)
        subprocess.call(["touch", test_output], bufsize=1, shell=False)

        subprocess.call([os.environ['EDITOR'], test_input], bufsize=1, shell=False)
        subprocess.call([os.environ['EDITOR'], test_output], bufsize=1, shell=False)
