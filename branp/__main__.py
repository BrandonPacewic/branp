# Copyright (c) Brandon Pacewic
# SPDX-License-Identifier: MIT

import sys

from typing import List

from branp.parser import create_command, parse_command


def main(args: List[str] = sys.argv[1:]) -> None:
    command_name, command_args = parse_command(args)
    command = create_command(command_name)
    command.main(command_args)


if __name__ == "__main__":
    main()
