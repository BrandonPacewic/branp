# Copyright (c) Brandon Pacewic
# SPDX-License-Identifier: MIT

from branp.core import OS

assert OS == "Darwin", "All current development has been on OSX. Other platform behavior is undefined."

__version__ = "v0.0.b0"
__all__ = ["__version__"]
