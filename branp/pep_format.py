# Copyright (c) Brandon Pacewic
# SPDX-License-Identifier: MIT

from optparse import Values

from branp.command import FormatCommand


class PepFormatCommand(FormatCommand):
    """
    Call autopep8 over a collection of files.
    """

    file_targets = {"py"}

    usage = """
      %prog pep8 [options] <format path(s)>"""

    def add_options(self) -> None:
        self.cmd_options.add_option(
            "-a",
            "--aggressive",
            dest="aggro",
            action="count",
            default=2,
            help="Enable non-whitespace changes, multiple -a will result in "
            "more aggressive changes; default is 2.",
        )

        self.cmd_options.add_option(
            "-p",
            "--passive",
            dest="passive",
            action="count",
            default=0,
            help="Reduce the aggressiveness of autopep8, multiple -p will "
            "result in even less aggressive changes.",
        )

    def get_format_command(self, options: Values) -> str:
        aggressiveness = options.aggro - options.passive
        command = " ".join(["autopep8 --in-place --max-line-length 120"] + ["--aggressive"] * aggressiveness)
        return command
