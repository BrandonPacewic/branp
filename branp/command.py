# Copyright (c) Brandon Pacewic
# SPDX-License-Identifier: MIT

import optparse
import os
import sys
import subprocess

from typing import List, Set, Tuple


class Command:
    usage: str = ""

    def __init__(self, name: str, summary: str) -> None:
        self.name = name
        self.summary = summary

        self.parser = optparse.OptionParser(
            prog=f"branp {self.name}",
            usage=self.usage,
            description=self.__doc__,
            add_help_option=True  # TODO: Change this?
        )

        option_group_name = f"{self.name} Options"
        self.cmd_options = optparse.OptionGroup(self.parser, option_group_name)

        self.add_options()

    def add_options(self) -> None:
        pass

    def run(self, options: optparse.Values, args: List[str]) -> None:
        raise NotImplementedError

    def parse(self, args: List[str]) -> Tuple[optparse.Values, List[str]]:
        return self.parser.parse_args(args)

    def main(self, args: List[str]) -> None:
        options, args = self.parse(args)
        self.run(options, args)


class FormatCommand(Command):
    config_file: str = ""
    file_targets: Set[str] = {}

    def get_format_command(self, options: optparse.Values) -> str:
        raise NotImplementedError

    def run(self, options: optparse.Values, args: List[str]) -> None:
        format_command = self.get_format_command(options)
        files: List[str] = []

        print("Indexing...")

        if not len(args):
            args.append(".")

        for arg in args:
            if os.path.isdir(arg):
                for dir, _, filenames in os.walk(arg):
                    for filename in filenames:
                        if filename.split(".")[-1] in self.file_targets:
                            files.append(f"{dir}/{filename}")
            elif os.path.isfile(arg):
                # TODO: Add warning if file suffix is not found within file targets
                if arg.split(".")[-1] in self.file_targets:
                    files.append(arg)
            else:
                print(f"{arg} is not a valid file or directory.")
                sys.exit(1)

        print(f"Formatting {len(files)} file(s)...")

        # TODO: Progress bar
        for file in files:
            subprocess.call(
                format_command.split() + [file],
                bufsize=1,
                shell=False,
            )

        print("Done")
