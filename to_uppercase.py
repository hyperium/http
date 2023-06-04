#/usr/bin/env python3
import re
from pathlib import Path
from string import capwords


def cap_header(name):
    if name.lower() == "www":
        return "WWW"
    if name.lower() == "etag":
        return "ETag"

    return capwords(name)

def make_header(name):
    new_header = "-".join([cap_header(n) for n in name.split("-")])

    new_header = new_header + f"\", b\"{new_header.upper()}"

    return new_header


def replace_header_names(filepath: Path):
    with filepath.open("r") as f:
        lines = f.readlines()

    header_matcher = re.compile(r"b\"([a-zA-Z\-]+)\"\);")
    char_matcher = re.compile(r"b'([a-z]{1})'")

    def upper_char(matchobj):
        return f"b'{matchobj.group(1).upper()}'"

    def upper_header(matchobj):
        return matchobj.group(0).replace(matchobj.group(1), matchobj.group(1).lower())

    updated = []
    for line in lines:
        new_line = header_matcher.sub(upper_header, line)

        # if ", //  9x" in line:
        #     new_line = line.replace("b'z'", "b'Z'")
        # elif ", // 10x" not in line and ", // 11x" not in line and ", // 12x" not in line:
        #     new_line = char_matcher.sub(upper_char, new_line)

        if line != new_line:
            print(new_line.strip())

        updated.append(new_line)

    with filepath.open("w") as f:
        f.writelines(updated)


if __name__ == '__main__':
    replace_header_names(Path("src/header/name.rs"))
