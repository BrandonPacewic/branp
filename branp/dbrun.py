# Copyright (c) Brandon Pacewic
# SPDX-License-Identifier: MIT

import os
import sys
import subprocess

from optparse import Values
from typing import List

from branp.command import Command
from branp.logging import get_logger, Colors

logger = get_logger()


class DbrunCommand(Command):
    """
    Compile and execute a standalone C++ file while defining a testing macro.
    """

    usage = """
      %prog dbrun [options] <file>"""

    debug_flag = "DBG_MODE"
    compile_command = f"g++ -g -std=c++17 -Wall -D{debug_flag}"

    def run(self, options: Values, args: List[str]) -> None:
        if not len(args) or len(args) > 1:
            logger.colored_critical(Colors.RED, "Expected 1 argument, file to run on.")
            sys.exit(1)

        file = args[0]

        if not file.endswith(".cpp"):
            file += ".cpp"

        if file not in os.listdir():
            logger.colored_critical(Colors.RED, f"{file} not found.")
            sys.exit(1)

        logger.info(f"[DEBUG MODE] Compiling {file} with c++17...")
        subprocess.call(self.compile_command.split() + [file], bufsize=1, shell=False)

        # TODO: Add timing
        logger.info(f"Successfully compiled in xxx")
        logger.info("--------------------")

        subprocess.call(["./a.out"], bufsize=1, shell=False)
