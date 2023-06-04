#/usr/bin/env python3
import re
from pathlib import Path


def replace_header_names(filepath: Path):
    with filepath.open("r") as f:
        lines = f.readlines()

    header_matcher = re.compile(r"b\"([a-z\-]+)\"")
    char_matcher = re.compile(r"b'([a-z]{1})'")

    def upper_char(matchobj):
        return f"b'{matchobj.group(1).upper()}'"

    def upper_header(matchobj):
        return matchobj.group(0).replace(matchobj.group(1), matchobj.group(1).upper())

    updated = []
    for line in lines:
        new_line = header_matcher.sub(upper_header, line)

        new_line = char_matcher.sub(upper_char, new_line)

        if line != new_line:
            print(new_line.strip())

        updated.append(new_line)

    with filepath.open("w") as f:
        f.writelines(updated)


if __name__ == '__main__':
    replace_header_names(Path("src/header/name.rs"))
