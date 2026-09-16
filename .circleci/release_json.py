#!/usr/bin/env python3
"""Reject ambiguous/nonstandard JSON before schema checks with jq."""

import json
import os
import stat
import sys


def unique_object(pairs: list[tuple[str, object]]) -> dict[str, object]:
    value: dict[str, object] = {}
    for key, item in pairs:
        if key in value:
            raise ValueError(f"duplicate JSON member: {key}")
        value[key] = item
    return value


def reject_constant(value: str) -> None:
    raise ValueError(f"nonstandard JSON constant: {value}")


def load(path: str) -> object:
    with open(os.open(path, os.O_RDONLY | os.O_NOFOLLOW), "rb") as handle:
        metadata = os.fstat(handle.fileno())
        if not stat.S_ISREG(metadata.st_mode) or not 0 < metadata.st_size <= 20 * 1024 * 1024:
            raise ValueError("JSON input must be a bounded nonempty regular file")
        return json.loads(handle.read().decode("utf-8"), object_pairs_hook=unique_object,
                          parse_constant=reject_constant)


if __name__ == "__main__":
    try:
        for filename in sys.argv[1:]:
            load(filename)
    except (OSError, ValueError, RecursionError) as error:
        sys.exit(f"Invalid release JSON: {error}")
