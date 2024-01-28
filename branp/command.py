# Copyright (c) Brandon Pacewic
# SPDX-License-Identifier: MIT

import optparse

from typing import List, Tuple


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
