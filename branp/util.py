# Copyright (c) Brandon Pacewic
# SPDX-License-Identifier: MIT

from time import perf_counter
from typing import List

GREEN = "\033[92m"
RED = "\033[91m"
BOLD_RED = "\033[1;91m"
YELLOW = "\033[93m"
ENDC = "\033[0m"


class Timer:
    def __init__(self) -> None:
        self.tics = [perf_counter()]

    def add_tic(self) -> None:
        self.tics.append(perf_counter())

    def get_elapsed(self) -> str:
        try:
            return f"{self.tics[-1] - self.tics[-2]:.3f}s"
        except IndexError:
            return "-0.000s"


def get_similar(key: str, match_against: List[str]) -> str | None:
    from difflib import get_close_matches
    key = key.lower()
    close_matches = get_close_matches(key, match_against)
    return close_matches[0] if close_matches else None
