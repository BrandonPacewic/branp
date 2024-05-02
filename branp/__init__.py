# Copyright (c) Brandon Pacewic
# SPDX-License-Identifier: MIT

from branp.core import OS
from branp.logging import init_logging

assert OS == "Darwin", "All current development has been on OSX. Other platform behavior is undefined."

__version__ = "v0.0.b0"
__all__ = ["__version__"]

init_logging()
