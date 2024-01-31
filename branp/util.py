# Copyright (c) Brandon Pacewic
# SPDX-License-Identifier: MIT

from typing import List


def get_similar(key: str, match_against: List[str]) -> str | None:
    from difflib import get_close_matches
    key = key.lower()
    close_matches = get_close_matches(key, match_against)
    return close_matches[0] if close_matches else None
