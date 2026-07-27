#!/usr/bin/env python3
"""Generate bp cloc language tables from cloc's language definitions.

Default source:
  https://raw.githubusercontent.com/AlDanial/cloc/master/cloc

Run `cargo fmt` after regenerating.
"""

from __future__ import annotations

import argparse
import re
from pathlib import Path
from urllib.request import Request, urlopen


DEFAULT_CLOC_URL = "https://raw.githubusercontent.com/AlDanial/cloc/master/cloc"

HEADER = """// Generated from cloc's language tables.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LanguageDescription {
    pub pattern: &'static str,
    pub language: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CommentSyntax {
    pub line_markers: &'static [&'static str],
    pub block_markers: &'static [BlockComment],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BlockComment {
    pub start: &'static str,
    pub end: &'static str,
}

const C_LIKE: CommentSyntax = CommentSyntax {
    line_markers: &[\"//\"],
    block_markers: &[BlockComment {
        start: \"/*\",
        end: \"*/\",
    }],
};

const HASH_LINE: CommentSyntax = CommentSyntax {
    line_markers: &[\"#\"],
    block_markers: &[],
};

const SEMICOLON_LINE: CommentSyntax = CommentSyntax {
    line_markers: &[\";\"],
    block_markers: &[],
};

const XML_LIKE: CommentSyntax = CommentSyntax {
    line_markers: &[],
    block_markers: &[BlockComment {
        start: \"<!--\",
        end: \"-->\",
    }],
};

pub fn comment_syntax_for_language(language: &str) -> Option<&'static CommentSyntax> {
    match language {
        \"ActionScript\"
        | \"ANTLR Grammar\"
        | \"Apex Class\"
        | \"Apex Trigger\"
        | \"Arduino Sketch\"
        | \"ArkTs\"
        | \"ASP.NET\"
        | \"AspectJ\"
        | \"Asymptote\"
        | \"C\"
        | \"C#\"
        | \"C# Designer\"
        | \"C# Generated\"
        | \"C++\"
        | \"C/C++ Header\"
        | \"Cadence\"
        | \"Cake Build Script\"
        | \"Carbon\"
        | \"CCS\"
        | \"Chapel\"
        | \"Clean\"
        | \"ColdFusion CFScript\"
        | \"CSS\"
        | \"CUDA\"
        | \"D\"
        | \"Dart\"
        | \"Dafny\"
        | \"DOORS Extension Language\"
        | \"Drools\"
        | \"GLSL\"
        | \"Go\"
        | \"Godot Shaders\"
        | \"Groovy\"
        | \"HCL\"
        | \"HLSL\"
        | \"Java\"
        | \"JavaScript\"
        | \"JSP\"
        | \"JSX\"
        | \"Kotlin\"
        | \"Less\"
        | \"Objective-C\"
        | \"Objective-C++\"
        | \"PHP\"
        | \"Processing\"
        | \"Protobuf\"
        | \"Rust\"
        | \"Sass\"
        | \"SCSS\"
        | \"Solidity\"
        | \"Swift\"
        | \"Svelte\"
        | \"TypeScript\"
        | \"Vala\"
        | \"Vala Header\"
        | \"Verilog-SystemVerilog\"
        | \"Vuejs Component\"
        | \"WGSL\"
        | \"Zig\" => Some(&C_LIKE),
        \"awk\" | \"Bazel\" | \"Bourne Again Shell\" | \"Bourne Shell\" | \"CMake\" | \"CoffeeScript\"
        | \"Containerfile\" | \"Crystal\" | \"CSON\" | \"Cucumber\" | \"Dockerfile\" | \"Elixir\"
        | \"Elixir Script\" | \"Fish Shell\" | \"Futhark\" | \"GraphQL\" | \"INI\" | \"Julia\" | \"Nim\"
        | \"Perl\" | \"PowerShell\" | \"Python\" | \"R\" | \"Raku\" | \"Ruby\" | \"Starlark\" | \"TOML\"
        | \"YAML\" | \"zsh\" => Some(&HASH_LINE),
        \"Assembly\" | \"Clojure\" | \"ClojureC\" | \"ClojureScript\" | \"Lisp\" | \"OpenCL\" | \"Scheme\" => {
            Some(&SEMICOLON_LINE)
        }
        \"Ant\" | \"HTML\" | \"Markdown\" | \"Maven\" | \"SVG\" | \"XML\" => Some(&XML_LIKE),
        _ => None,
    }
}

"""


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--cloc",
        type=Path,
        help="Path to a local cloc Perl script; defaults to fetching GitHub raw content",
    )
    parser.add_argument(
        "--url",
        default=DEFAULT_CLOC_URL,
        help="GitHub raw cloc URL to fetch when --cloc is not provided",
    )
    parser.add_argument(
        "--out",
        type=Path,
        default=Path("src/ops/cloc/languages.rs"),
        help="Output Rust file",
    )
    args = parser.parse_args()

    source = read_source(args.cloc, args.url)
    extension_languages = entries(source, "Language_by_Extension")
    filename_languages = entries(source, "Language_by_File_Type")
    prefix_languages = entries(source, "Language_by_Prefix")

    output = [HEADER]
    write_const(output, "EXTENSION_LANGUAGES", extension_languages)
    write_const(output, "FILENAME_LANGUAGES", filename_languages)
    write_const(output, "PREFIX_LANGUAGES", prefix_languages)
    write_match(output, "language_for_extension", extension_languages)
    write_match(output, "language_for_filename", filename_languages)
    write_match(output, "language_for_prefix", prefix_languages)

    args.out.write_text("".join(output))
    print(
        f"wrote {args.out}: "
        f"{len(extension_languages)} extensions, "
        f"{len(filename_languages)} filenames, "
        f"{len(prefix_languages)} prefixes"
    )


def read_source(path: Path | None, url: str) -> str:
    if path is not None:
        return path.read_text()

    request = Request(url, headers={"User-Agent": "bp-cloc-language-generator"})
    with urlopen(request, timeout=30) as response:
        return response.read().decode("utf-8")


def entries(source: str, name: str) -> list[tuple[str, str]]:
    body = table_body(source, name)
    pairs: dict[str, str] = {}

    single_quoted = re.compile(r"'((?:\\'|[^'])*)'\s*=>\s*'((?:\\'|[^'])*)'")
    double_quoted = re.compile(r"'((?:\\'|[^'])*)'\s*=>\s*\"((?:\\\"|[^\"])*)\"")

    for line in body.splitlines():
        line = re.sub(r"#.*", "", line).strip()
        match = single_quoted.match(line) or double_quoted.match(line)
        if match:
            key = unescape(match.group(1))
            value = unescape(match.group(2))
            pairs[key] = value

    return sorted(pairs.items(), key=lambda item: (item[0].lower(), item[0]))


def table_body(source: str, name: str) -> str:
    pattern = rf"%\{{\$rh_{name}\}}\s*=\s*\((.*?)\n\s*\);"
    match = re.search(pattern, source, re.S)
    if not match:
        raise SystemExit(f"could not find cloc table {name}")
    return match.group(1)


def unescape(value: str) -> str:
    return value.replace("\\'", "'").replace('\\"', '"')


def rust_string(value: str) -> str:
    return '"' + value.replace("\\", "\\\\").replace('"', '\\"') + '"'


def write_const(output: list[str], name: str, values: list[tuple[str, str]]) -> None:
    output.append("#[allow(dead_code)]\n")
    output.append(f"pub const {name}: &[LanguageDescription] = &[\n")
    for pattern, language in values:
        output.append("    LanguageDescription {\n")
        output.append(f"        pattern: {rust_string(pattern)},\n")
        output.append(f"        language: {rust_string(language)},\n")
        output.append("    },\n")
    output.append("];\n\n")


def write_match(output: list[str], name: str, values: list[tuple[str, str]]) -> None:
    output.append(f"pub fn {name}(pattern: &str) -> Option<&'static str> {{\n")
    output.append("    match pattern {\n")
    for pattern, language in values:
        output.append(f"        {rust_string(pattern)} => Some({rust_string(language)}),\n")
    output.append("        _ => None,\n")
    output.append("    }\n")
    output.append("}\n\n")


if __name__ == "__main__":
    main()
