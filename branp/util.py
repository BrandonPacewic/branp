# Copyright (c) Brandon Pacewic
# SPDX-License-Identifier: MIT

from typing import List

GREEN = "\033[92m"
RED = "\033[91m"
BOLD_RED = "\033[1;91m"
YELLOW = "\033[93m"
ENDC = "\033[0m"


def get_similar(key: str, match_against: List[str]) -> str | None:
    from difflib import get_close_matches
    key = key.lower()
    close_matches = get_close_matches(key, match_against)
    return close_matches[0] if close_matches else None
