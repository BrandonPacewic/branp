# Copyright (c) Brandon Pacewic
# SPDX-License-Identifier: MIT

import os
import sys

from optparse import Values
from typing import List

from branp.core import CONFIG_DIR
from branp.command import FormatCommand
from branp.util import get_similar


class ClangFormatCommand(FormatCommand):
    """
    Call clang-format over a large collection of files.
    """

    file_targets = {"cpp", "h", "cc"}

    usage = """
      %prog clang [options] <format path>"""

    def add_options(self) -> None:
        self.cmd_options.add_option(
            "-c",
            "--config",
            action="store",
            type="string",
            dest="config",
        )

    def get_format_command(self, options: Values) -> str:
        if not options.config:
            return "clang-format -i -style=file"

        config_file_options: List[str] = []
        config_file = ""

        if os.path.isdir(CONFIG_DIR):
            for dir, _, filenames in os.walk(CONFIG_DIR):
                for filename in filenames:
                    if filename.split(".")[-1] == "clang-format":
                        config_file_options.append(os.path.join(dir, filename))

        config_option_prefixes = [x.split("/")[-1].split(".")[0].lower() for x in config_file_options]

        for option, option_prefix in zip(config_file_options, config_option_prefixes):
            if option_prefix == options.config.lower():
                config_file = option
                break

        if not config_file:
            msg = [f"Unknown clang-format preset {options.config}"]
            guess = get_similar(options.config, config_option_prefixes)

            if guess:
                msg.append(f"Did you mean '{guess}?")

            print("\n\n".join(msg))
            sys.exit(1)

        return f"clang-format -i -style=file:{config_file}"
