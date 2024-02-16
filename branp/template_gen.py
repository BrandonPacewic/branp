# Copyright (c) Brandon Pacewic
# SPDX-License-Identifier: MIT

import os
import sys
import subprocess

from optparse import Values
from string import ascii_uppercase
from typing import List

from branp.command import Command
from branp.core import CONFIG_DIR
from branp.util import get_similar


class TemplateGenCommand(Command):
    """
    Generate templates from global template files.
    """

    usage = """
      %prog template_gen [options] <file>"""

    def add_options(self) -> None:
        self.cmd_options.add_option(
            "-l",
            "--link",
            action="store_true",
            dest="link",
        )

    def run(self, options: Values, args: List[str]) -> None:
        if not len(args) or len(args) > 2:
            print("Expected a different organization of arguments.")  # TODO: Make better
            sys.exit(1)

        templates: List[str] = []

        if os.path.isdir(CONFIG_DIR) and os.path.exists(f"{CONFIG_DIR}/templates"):
            for dir, _, filenames in os.walk(f"{CONFIG_DIR}/templates"):
                for filename in filenames:
                    templates.append(os.path.join(dir, filename))

        if not templates:
            print("No templates found.")
            sys.exit(1)

        template_names = [x.split("/")[-1] for x in templates]
        template = get_similar(args[0], template_names)

        if template:
            matched_template = [x for x in templates if x.split("/")[-1] == template]
        else:
            matched_template = []

        if len(matched_template) > 1:
            msg = ["Multiple matching templates found."]

            for match in matched_template:
                msg.append(match)

            print("\n".join(msg))
            sys.exit(1)

        if not len(matched_template):
            print("No matching template found.")
            sys.exit(1)

        template = matched_template[0]

        command_prefix = "ln" if options.link else "cp"

        if len(args) == 1:
            subprocess.call([command_prefix, template, "."], bufsize=1, shell=False)
            sys.exit(0)

        name_count: int | str

        try:
            name_count = int(args[1])
        except ValueError:
            name_count = args[1]

        if isinstance(name_count, int):
            if options.link:
                print("Invalid option: --link for multiple file generation.")
                sys.exit(1)

            for i in range(name_count):
                file_name = f"{ascii_uppercase[i]}.{template.split('.')[-1]}"
                subprocess.call([command_prefix, template, f"{os.getcwd()}/{file_name}"], bufsize=1, shell=False)
        elif isinstance(name_count, str):

            file_name = name_count if name_count.endswith(template.split(
                ".")[-1]) else f"{name_count}.{template.split('.')[-1]}"
            subprocess.call([command_prefix, template, f"{os.getcwd()}/{file_name}"], bufsize=1, shell=False)

        else:
            print(f"Your input was evaluated to {type(name_count)}.")
            sys.exit(1)
