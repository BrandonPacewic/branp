# Copyright (c) Brandon Pacewic
# SPDX-License-Identifier: MIT

import os
import platform

HOME = os.getenv("HOME", os.getenv("USERPROFILE"))
XDG_CONFIG_DIR = os.getenv("XDG_CONFIG_HOME", os.path.join(HOME, ".config"))
CONFIG_DIR = os.path.join(XDG_CONFIG_DIR, "branp")

OS = platform.uname()[0]
