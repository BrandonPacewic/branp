# Copyright (c) Brandon Pacewic
# SPDX-License-Identifier: MIT

import sys

from collections import namedtuple
from importlib import import_module
from optparse import OptionParser
from typing import Any, List, Tuple

from branp import __version__
from branp.command import Command


CommandInfo = namedtuple("commandInfo", "module, class_name, summary")

COMMANDS_DICT: dict[str, CommandInfo] = {
    "format": CommandInfo(
        "branp.format",
        "FormatCommand",
        "Scripts used utilizing various types of formatters "
        "across multiple files in a given directory."
    ),
}


def create_main_parser() -> OptionParser:
    parser = OptionParser(
        usage="\n\tbranp <command> [options]",
        add_help_option=True,  # TODO: Change this?
        prog="branp",
    )
    parser.disable_interspersed_args()

    parser.main = True
    parser.version = __version__

    parser.description = "\n".join(
        [""] + [
            f"\t{command} {info.summary}"
            for command, info in COMMANDS_DICT.items()
        ]
    )

    return parser


def parse_command(args: List[str]) -> Tuple[str, List[str]]:
    parser = create_main_parser()
    general_options, command_args = parser.parse_args(args)

    if general_options.version:
        parser.print_version()
        sys.exit(0)

    command_name = command_args[0]

    if command_name not in COMMANDS_DICT:
        msg = [f"Unknown command '{command_name}'."]
        guess = get_similar_commands(command_name)

        if guess:
            msg.append(f"Did you mean '{guess}'?")

        print("\n\n".join(msg))
        sys.exit(1)


def create_command(name: str, **kwargs: Any) -> Command:
    module_path, class_name, summary = COMMANDS_DICT[name]
    module = import_module(module_path)
    command_class = getattr(module, class_name)
    command = command_class(name, summary, **kwargs)

    return command


def get_similar_commands(command_name: str) -> str | None:
    import difflib
    command_name = command_name.lower()
    close_commands = difflib.get_close_matches(command_name, COMMANDS_DICT.key())

    return close_commands[0] if close_commands else None
