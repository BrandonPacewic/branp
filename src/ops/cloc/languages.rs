// Generated from `github.com/AlDanial/cloc`'s language tables.

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
    line_markers: &["//"],
    block_markers: &[BlockComment {
        start: "/*",
        end: "*/",
    }],
};

const HASH_LINE: CommentSyntax = CommentSyntax {
    line_markers: &["#"],
    block_markers: &[],
};

const SEMICOLON_LINE: CommentSyntax = CommentSyntax {
    line_markers: &[";"],
    block_markers: &[],
};

const XML_LIKE: CommentSyntax = CommentSyntax {
    line_markers: &[],
    block_markers: &[BlockComment {
        start: "<!--",
        end: "-->",
    }],
};

pub fn comment_syntax_for_language(language: &str) -> Option<&'static CommentSyntax> {
    match language {
        "ActionScript"
        | "ANTLR Grammar"
        | "Apex Class"
        | "Apex Trigger"
        | "Arduino Sketch"
        | "ArkTs"
        | "ASP.NET"
        | "AspectJ"
        | "Asymptote"
        | "C"
        | "C#"
        | "C# Designer"
        | "C# Generated"
        | "C++"
        | "C/C++ Header"
        | "Cadence"
        | "Cake Build Script"
        | "Carbon"
        | "CCS"
        | "Chapel"
        | "Clean"
        | "ColdFusion CFScript"
        | "CSS"
        | "CUDA"
        | "D"
        | "Dart"
        | "Dafny"
        | "DOORS Extension Language"
        | "Drools"
        | "GLSL"
        | "Go"
        | "Godot Shaders"
        | "Groovy"
        | "HCL"
        | "HLSL"
        | "Java"
        | "JavaScript"
        | "JSP"
        | "JSX"
        | "Kotlin"
        | "Less"
        | "Objective-C"
        | "Objective-C++"
        | "PHP"
        | "Processing"
        | "Protobuf"
        | "Rust"
        | "Sass"
        | "SCSS"
        | "Solidity"
        | "Swift"
        | "Svelte"
        | "TypeScript"
        | "Vala"
        | "Vala Header"
        | "Verilog-SystemVerilog"
        | "Vuejs Component"
        | "WGSL"
        | "Zig" => Some(&C_LIKE),
        "awk" | "Bazel" | "Bourne Again Shell" | "Bourne Shell" | "CMake" | "CoffeeScript"
        | "Containerfile" | "Crystal" | "CSON" | "Cucumber" | "Dockerfile" | "Elixir"
        | "Elixir Script" | "Fish Shell" | "Futhark" | "GraphQL" | "INI" | "Julia" | "Nim"
        | "Perl" | "PowerShell" | "Python" | "R" | "Raku" | "Ruby" | "Starlark" | "TOML"
        | "YAML" | "zsh" => Some(&HASH_LINE),
        "Assembly" | "Clojure" | "ClojureC" | "ClojureScript" | "Lisp" | "OpenCL" | "Scheme" => {
            Some(&SEMICOLON_LINE)
        }
        "Ant" | "HTML" | "Markdown" | "Maven" | "SVG" | "XML" => Some(&XML_LIKE),
        _ => None,
    }
}

#[allow(dead_code)]
pub const EXTENSION_LANGUAGES: &[LanguageDescription] = &[
    LanguageDescription {
        pattern: "4th",
        language: "Forth",
    },
    LanguageDescription {
        pattern: "_coffee",
        language: "CoffeeScript",
    },
    LanguageDescription {
        pattern: "_js",
        language: "JavaScript",
    },
    LanguageDescription {
        pattern: "a51",
        language: "Assembly",
    },
    LanguageDescription {
        pattern: "abap",
        language: "ABAP",
    },
    LanguageDescription {
        pattern: "ac",
        language: "m4",
    },
    LanguageDescription {
        pattern: "ack",
        language: "Perl",
    },
    LanguageDescription {
        pattern: "ada",
        language: "Ada",
    },
    LanguageDescription {
        pattern: "adb",
        language: "Ada",
    },
    LanguageDescription {
        pattern: "adml",
        language: "XML",
    },
    LanguageDescription {
        pattern: "admx",
        language: "XML",
    },
    LanguageDescription {
        pattern: "ado",
        language: "Stata",
    },
    LanguageDescription {
        pattern: "adoc",
        language: "AsciiDoc",
    },
    LanguageDescription {
        pattern: "ads",
        language: "Ada",
    },
    LanguageDescription {
        pattern: "adso",
        language: "ADSO/IDSM",
    },
    LanguageDescription {
        pattern: "agda",
        language: "Agda",
    },
    LanguageDescription {
        pattern: "ahk",
        language: "AutoHotkey",
    },
    LanguageDescription {
        pattern: "ahkl",
        language: "AutoHotkey",
    },
    LanguageDescription {
        pattern: "aj",
        language: "AspectJ",
    },
    LanguageDescription {
        pattern: "al",
        language: "Perl",
    },
    LanguageDescription {
        pattern: "am",
        language: "make",
    },
    LanguageDescription {
        pattern: "ample",
        language: "AMPLE",
    },
    LanguageDescription {
        pattern: "ant",
        language: "XML",
    },
    LanguageDescription {
        pattern: "apl",
        language: "APL",
    },
    LanguageDescription {
        pattern: "apla",
        language: "APL",
    },
    LanguageDescription {
        pattern: "aplc",
        language: "APL",
    },
    LanguageDescription {
        pattern: "aplf",
        language: "APL",
    },
    LanguageDescription {
        pattern: "apli",
        language: "APL",
    },
    LanguageDescription {
        pattern: "apln",
        language: "APL",
    },
    LanguageDescription {
        pattern: "aplo",
        language: "APL",
    },
    LanguageDescription {
        pattern: "app.config",
        language: "XML",
    },
    LanguageDescription {
        pattern: "app.src",
        language: "Erlang",
    },
    LanguageDescription {
        pattern: "applescript",
        language: "AppleScript",
    },
    LanguageDescription {
        pattern: "appraisals",
        language: "Ruby",
    },
    LanguageDescription {
        pattern: "arcconfig",
        language: "JSON",
    },
    LanguageDescription {
        pattern: "art",
        language: "Arturo",
    },
    LanguageDescription {
        pattern: "as",
        language: "ActionScript",
    },
    LanguageDescription {
        pattern: "asa",
        language: "ASP",
    },
    LanguageDescription {
        pattern: "asax",
        language: "ASP.NET",
    },
    LanguageDescription {
        pattern: "asciidoc",
        language: "AsciiDoc",
    },
    LanguageDescription {
        pattern: "ascx",
        language: "ASP.NET",
    },
    LanguageDescription {
        pattern: "asd",
        language: "Lisp",
    },
    LanguageDescription {
        pattern: "ashx",
        language: "ASP",
    },
    LanguageDescription {
        pattern: "asm",
        language: "Assembly",
    },
    LanguageDescription {
        pattern: "asmx",
        language: "ASP.NET",
    },
    LanguageDescription {
        pattern: "asp",
        language: "ASP",
    },
    LanguageDescription {
        pattern: "aspx",
        language: "ASP.NET",
    },
    LanguageDescription {
        pattern: "astro",
        language: "Astro",
    },
    LanguageDescription {
        pattern: "asy",
        language: "Asymptote",
    },
    LanguageDescription {
        pattern: "auk",
        language: "awk",
    },
    LanguageDescription {
        pattern: "aux",
        language: "TeX",
    },
    LanguageDescription {
        pattern: "avsc",
        language: "JSON",
    },
    LanguageDescription {
        pattern: "aw",
        language: "PHP",
    },
    LanguageDescription {
        pattern: "awk",
        language: "awk",
    },
    LanguageDescription {
        pattern: "axd",
        language: "ASP",
    },
    LanguageDescription {
        pattern: "axml",
        language: "XML",
    },
    LanguageDescription {
        pattern: "b",
        language: "Brainfuck",
    },
    LanguageDescription {
        pattern: "BAS",
        language: "Visual Basic",
    },
    LanguageDescription {
        pattern: "bas",
        language: "Visual Basic",
    },
    LanguageDescription {
        pattern: "bash",
        language: "Bourne Again Shell",
    },
    LanguageDescription {
        pattern: "BAT",
        language: "DOS Batch",
    },
    LanguageDescription {
        pattern: "bat",
        language: "DOS Batch",
    },
    LanguageDescription {
        pattern: "bazel",
        language: "Starlark",
    },
    LanguageDescription {
        pattern: "bbx",
        language: "TeX",
    },
    LanguageDescription {
        pattern: "bdy",
        language: "Oracle PL/SQL",
    },
    LanguageDescription {
        pattern: "berksfile",
        language: "Ruby",
    },
    LanguageDescription {
        pattern: "bf",
        language: "Brainfuck",
    },
    LanguageDescription {
        pattern: "bib",
        language: "TeX",
    },
    LanguageDescription {
        pattern: "blade",
        language: "Blade",
    },
    LanguageDescription {
        pattern: "blade.php",
        language: "Blade",
    },
    LanguageDescription {
        pattern: "blp",
        language: "Blueprint",
    },
    LanguageDescription {
        pattern: "bod",
        language: "Oracle PL/SQL",
    },
    LanguageDescription {
        pattern: "bones",
        language: "JavaScript",
    },
    LanguageDescription {
        pattern: "boot",
        language: "Clojure",
    },
    LanguageDescription {
        pattern: "brewfile",
        language: "Ruby",
    },
    LanguageDescription {
        pattern: "brs",
        language: "BrightScript",
    },
    LanguageDescription {
        pattern: "bst",
        language: "TeX",
    },
    LanguageDescription {
        pattern: "BTM",
        language: "DOS Batch",
    },
    LanguageDescription {
        pattern: "btm",
        language: "DOS Batch",
    },
    LanguageDescription {
        pattern: "btp",
        language: "BizTalk Pipeline",
    },
    LanguageDescription {
        pattern: "btproj",
        language: "MSBuild script",
    },
    LanguageDescription {
        pattern: "buck",
        language: "Python",
    },
    LanguageDescription {
        pattern: "BUILD",
        language: "Bazel",
    },
    LanguageDescription {
        pattern: "build",
        language: "NAnt script",
    },
    LanguageDescription {
        pattern: "build.bazel",
        language: "Python",
    },
    LanguageDescription {
        pattern: "build.xml",
        language: "Ant",
    },
    LanguageDescription {
        pattern: "builder",
        language: "Ruby",
    },
    LanguageDescription {
        pattern: "buildfile",
        language: "Ruby",
    },
    LanguageDescription {
        pattern: "buildozer.spec",
        language: "INI",
    },
    LanguageDescription {
        pattern: "builds",
        language: "XML",
    },
    LanguageDescription {
        pattern: "bzl",
        language: "Starlark",
    },
    LanguageDescription {
        pattern: "C",
        language: "C++",
    },
    LanguageDescription {
        pattern: "c",
        language: "C",
    },
    LanguageDescription {
        pattern: "c++",
        language: "C++",
    },
    LanguageDescription {
        pattern: "c++m",
        language: "C++",
    },
    LanguageDescription {
        pattern: "c5",
        language: "CoCoA 5",
    },
    LanguageDescription {
        pattern: "cairo",
        language: "Cairo",
    },
    LanguageDescription {
        pattern: "cake",
        language: "Cake Build Script",
    },
    LanguageDescription {
        pattern: "cakefile",
        language: "CoffeeScript",
    },
    LanguageDescription {
        pattern: "capfile",
        language: "Ruby",
    },
    LanguageDescription {
        pattern: "carbon",
        language: "Carbon",
    },
    LanguageDescription {
        pattern: "cats",
        language: "C",
    },
    LanguageDescription {
        pattern: "CBL",
        language: "COBOL",
    },
    LanguageDescription {
        pattern: "cbl",
        language: "COBOL",
    },
    LanguageDescription {
        pattern: "cbx",
        language: "TeX",
    },
    LanguageDescription {
        pattern: "cc",
        language: "C++",
    },
    LanguageDescription {
        pattern: "ccm",
        language: "C++",
    },
    LanguageDescription {
        pattern: "ccp",
        language: "COBOL",
    },
    LanguageDescription {
        pattern: "ccproj",
        language: "XML",
    },
    LanguageDescription {
        pattern: "ccs",
        language: "CCS",
    },
    LanguageDescription {
        pattern: "ccxml",
        language: "XML",
    },
    LanguageDescription {
        pattern: "cdc",
        language: "Cadence",
    },
    LanguageDescription {
        pattern: "cdf",
        language: "Mathematica",
    },
    LanguageDescription {
        pattern: "cfc",
        language: "ColdFusion CFScript",
    },
    LanguageDescription {
        pattern: "cfm",
        language: "ColdFusion",
    },
    LanguageDescription {
        pattern: "cfml",
        language: "ColdFusion",
    },
    LanguageDescription {
        pattern: "cg",
        language: "HLSL",
    },
    LanguageDescription {
        pattern: "cg3",
        language: "Constraint Grammar",
    },
    LanguageDescription {
        pattern: "cginc",
        language: "HLSL",
    },
    LanguageDescription {
        pattern: "ch",
        language: "xBase Header",
    },
    LanguageDescription {
        pattern: "chpl",
        language: "Chapel",
    },
    LanguageDescription {
        pattern: "cii",
        language: "TNSDL",
    },
    LanguageDescription {
        pattern: "cin",
        language: "TNSDL",
    },
    LanguageDescription {
        pattern: "circom",
        language: "Circom",
    },
    LanguageDescription {
        pattern: "civet",
        language: "Civet",
    },
    LanguageDescription {
        pattern: "cj",
        language: "Clojure/Cangjie",
    },
    LanguageDescription {
        pattern: "cjs",
        language: "JavaScript",
    },
    LanguageDescription {
        pattern: "cjsx",
        language: "CoffeeScript",
    },
    LanguageDescription {
        pattern: "cl",
        language: "Lisp/OpenCL",
    },
    LanguageDescription {
        pattern: "cl2",
        language: "Clojure",
    },
    LanguageDescription {
        pattern: "clang-format",
        language: "YAML",
    },
    LanguageDescription {
        pattern: "clang-tidy",
        language: "YAML",
    },
    LanguageDescription {
        pattern: "classpath",
        language: "XML",
    },
    LanguageDescription {
        pattern: "clixml",
        language: "XML",
    },
    LanguageDescription {
        pattern: "clj",
        language: "Clojure",
    },
    LanguageDescription {
        pattern: "cljc",
        language: "ClojureC",
    },
    LanguageDescription {
        pattern: "cljs",
        language: "ClojureScript",
    },
    LanguageDescription {
        pattern: "cljs.hl",
        language: "Clojure",
    },
    LanguageDescription {
        pattern: "cljscm",
        language: "Clojure",
    },
    LanguageDescription {
        pattern: "cljx",
        language: "Clojure",
    },
    LanguageDescription {
        pattern: "cls",
        language: "Visual Basic/TeX/Apex Class",
    },
    LanguageDescription {
        pattern: "cmake",
        language: "CMake",
    },
    LanguageDescription {
        pattern: "cmake.in",
        language: "CMake",
    },
    LanguageDescription {
        pattern: "CMakeLists.txt",
        language: "CMake",
    },
    LanguageDescription {
        pattern: "CMD",
        language: "DOS Batch",
    },
    LanguageDescription {
        pattern: "cmd",
        language: "DOS Batch",
    },
    LanguageDescription {
        pattern: "COB",
        language: "COBOL",
    },
    LanguageDescription {
        pattern: "cob",
        language: "COBOL",
    },
    LanguageDescription {
        pattern: "cobol",
        language: "COBOL",
    },
    LanguageDescription {
        pattern: "cocoa5",
        language: "CoCoA 5",
    },
    LanguageDescription {
        pattern: "cocoa5server",
        language: "CoCoA 5",
    },
    LanguageDescription {
        pattern: "coffee",
        language: "CoffeeScript",
    },
    LanguageDescription {
        pattern: "comp",
        language: "GLSL",
    },
    LanguageDescription {
        pattern: "component",
        language: "Visualforce Component",
    },
    LanguageDescription {
        pattern: "composer.lock",
        language: "JSON",
    },
    LanguageDescription {
        pattern: "Containerfile",
        language: "Containerfile",
    },
    LanguageDescription {
        pattern: "contents.lr",
        language: "Markdown",
    },
    LanguageDescription {
        pattern: "cpanfile",
        language: "Perl",
    },
    LanguageDescription {
        pattern: "cpkg5",
        language: "CoCoA 5",
    },
    LanguageDescription {
        pattern: "CPP",
        language: "C++",
    },
    LanguageDescription {
        pattern: "cpp",
        language: "C++",
    },
    LanguageDescription {
        pattern: "cppm",
        language: "C++",
    },
    LanguageDescription {
        pattern: "cproject",
        language: "XML",
    },
    LanguageDescription {
        pattern: "cpy",
        language: "COBOL",
    },
    LanguageDescription {
        pattern: "cql",
        language: "SQL",
    },
    LanguageDescription {
        pattern: "cr",
        language: "Crystal",
    },
    LanguageDescription {
        pattern: "cscfg",
        language: "XML",
    },
    LanguageDescription {
        pattern: "csdef",
        language: "XML",
    },
    LanguageDescription {
        pattern: "csh",
        language: "C Shell",
    },
    LanguageDescription {
        pattern: "cshtml",
        language: "Razor",
    },
    LanguageDescription {
        pattern: "csl",
        language: "XML",
    },
    LanguageDescription {
        pattern: "cson",
        language: "CSON",
    },
    LanguageDescription {
        pattern: "csproj",
        language: "MSBuild script",
    },
    LanguageDescription {
        pattern: "css",
        language: "CSS",
    },
    LanguageDescription {
        pattern: "csv",
        language: "CSV",
    },
    LanguageDescription {
        pattern: "ct",
        language: "XML",
    },
    LanguageDescription {
        pattern: "ctl",
        language: "Visual Basic",
    },
    LanguageDescription {
        pattern: "ctp",
        language: "PHP",
    },
    LanguageDescription {
        pattern: "cu",
        language: "CUDA",
    },
    LanguageDescription {
        pattern: "cuh",
        language: "CUDA",
    },
    LanguageDescription {
        pattern: "cvt",
        language: "Civet",
    },
    LanguageDescription {
        pattern: "cvtx",
        language: "Civet",
    },
    LanguageDescription {
        pattern: "cxx",
        language: "C++",
    },
    LanguageDescription {
        pattern: "cxxm",
        language: "C++",
    },
    LanguageDescription {
        pattern: "d",
        language: "D/dtrace",
    },
    LanguageDescription {
        pattern: "da",
        language: "DAL",
    },
    LanguageDescription {
        pattern: "dangerfile",
        language: "Ruby",
    },
    LanguageDescription {
        pattern: "dart",
        language: "Dart",
    },
    LanguageDescription {
        pattern: "data.sql",
        language: "SQL Data",
    },
    LanguageDescription {
        pattern: "dcl",
        language: "Clean",
    },
    LanguageDescription {
        pattern: "def",
        language: "Windows Module Definition",
    },
    LanguageDescription {
        pattern: "deliverfile",
        language: "Ruby",
    },
    LanguageDescription {
        pattern: "depproj",
        language: "XML",
    },
    LanguageDescription {
        pattern: "derw",
        language: "Derw",
    },
    LanguageDescription {
        pattern: "dfm",
        language: "Delphi Form",
    },
    LanguageDescription {
        pattern: "dfy",
        language: "Dafny",
    },
    LanguageDescription {
        pattern: "dhall",
        language: "dhall",
    },
    LanguageDescription {
        pattern: "diff",
        language: "diff",
    },
    LanguageDescription {
        pattern: "dita",
        language: "DITA",
    },
    LanguageDescription {
        pattern: "ditamap",
        language: "XML",
    },
    LanguageDescription {
        pattern: "ditaval",
        language: "XML",
    },
    LanguageDescription {
        pattern: "dll.config",
        language: "XML",
    },
    LanguageDescription {
        pattern: "dlm",
        language: "IDL",
    },
    LanguageDescription {
        pattern: "dmap",
        language: "NASTRAN DMAP",
    },
    LanguageDescription {
        pattern: "DO",
        language: "Stata",
    },
    LanguageDescription {
        pattern: "do",
        language: "Stata",
    },
    LanguageDescription {
        pattern: "Dockerfile",
        language: "Dockerfile",
    },
    LanguageDescription {
        pattern: "dockerfile",
        language: "Dockerfile",
    },
    LanguageDescription {
        pattern: "dofile",
        language: "AMPLE",
    },
    LanguageDescription {
        pattern: "doh",
        language: "Stata",
    },
    LanguageDescription {
        pattern: "dotsettings",
        language: "XML",
    },
    LanguageDescription {
        pattern: "dpr",
        language: "Pascal",
    },
    LanguageDescription {
        pattern: "drl",
        language: "Drools",
    },
    LanguageDescription {
        pattern: "dsc",
        language: "DenizenScript",
    },
    LanguageDescription {
        pattern: "dsr",
        language: "Visual Basic",
    },
    LanguageDescription {
        pattern: "dt",
        language: "DIET",
    },
    LanguageDescription {
        pattern: "dtd",
        language: "DTD",
    },
    LanguageDescription {
        pattern: "dtx",
        language: "TeX",
    },
    LanguageDescription {
        pattern: "dxl",
        language: "DOORS Extension Language",
    },
    LanguageDescription {
        pattern: "dyalog",
        language: "APL",
    },
    LanguageDescription {
        pattern: "dyapp",
        language: "APL",
    },
    LanguageDescription {
        pattern: "e",
        language: "Specman e",
    },
    LanguageDescription {
        pattern: "e4",
        language: "Forth",
    },
    LanguageDescription {
        pattern: "ec",
        language: "C",
    },
    LanguageDescription {
        pattern: "ecpp",
        language: "ECPP",
    },
    LanguageDescription {
        pattern: "ecr",
        language: "Embedded Crystal",
    },
    LanguageDescription {
        pattern: "editorconfig",
        language: "INI",
    },
    LanguageDescription {
        pattern: "eex",
        language: "EEx",
    },
    LanguageDescription {
        pattern: "ejs",
        language: "EJS",
    },
    LanguageDescription {
        pattern: "el",
        language: "Lisp",
    },
    LanguageDescription {
        pattern: "eliom",
        language: "OCaml",
    },
    LanguageDescription {
        pattern: "eliomi",
        language: "OCaml",
    },
    LanguageDescription {
        pattern: "elm",
        language: "Elm",
    },
    LanguageDescription {
        pattern: "emakefile",
        language: "Erlang",
    },
    LanguageDescription {
        pattern: "ERB",
        language: "ERB",
    },
    LanguageDescription {
        pattern: "erb",
        language: "ERB",
    },
    LanguageDescription {
        pattern: "erl",
        language: "Erlang",
    },
    LanguageDescription {
        pattern: "es6",
        language: "JavaScript",
    },
    LanguageDescription {
        pattern: "ets",
        language: "ArkTs",
    },
    LanguageDescription {
        pattern: "ex",
        language: "Elixir",
    },
    LanguageDescription {
        pattern: "exp",
        language: "Expect",
    },
    LanguageDescription {
        pattern: "expr-dist",
        language: "R",
    },
    LanguageDescription {
        pattern: "exs",
        language: "Elixir Script",
    },
    LanguageDescription {
        pattern: "eye",
        language: "Ruby",
    },
    LanguageDescription {
        pattern: "F",
        language: "Fortran 77",
    },
    LanguageDescription {
        pattern: "f",
        language: "Fortran 77/Forth",
    },
    LanguageDescription {
        pattern: "F77",
        language: "Fortran 77",
    },
    LanguageDescription {
        pattern: "f77",
        language: "Fortran 77",
    },
    LanguageDescription {
        pattern: "f83",
        language: "Forth",
    },
    LanguageDescription {
        pattern: "F90",
        language: "Fortran 90",
    },
    LanguageDescription {
        pattern: "f90",
        language: "Fortran 90",
    },
    LanguageDescription {
        pattern: "F95",
        language: "Fortran 95",
    },
    LanguageDescription {
        pattern: "f95",
        language: "Fortran 95",
    },
    LanguageDescription {
        pattern: "fastfile",
        language: "Ruby",
    },
    LanguageDescription {
        pattern: "fb",
        language: "Forth",
    },
    LanguageDescription {
        pattern: "fbs",
        language: "Flatbuffers",
    },
    LanguageDescription {
        pattern: "feature",
        language: "Cucumber",
    },
    LanguageDescription {
        pattern: "filters",
        language: "XML",
    },
    LanguageDescription {
        pattern: "fish",
        language: "Fish Shell",
    },
    LanguageDescription {
        pattern: "fmt",
        language: "Oracle Forms",
    },
    LanguageDescription {
        pattern: "fnc",
        language: "Oracle PL/SQL",
    },
    LanguageDescription {
        pattern: "fnl",
        language: "Fennel",
    },
    LanguageDescription {
        pattern: "focexec",
        language: "Focus",
    },
    LanguageDescription {
        pattern: "FOR",
        language: "Fortran 77",
    },
    LanguageDescription {
        pattern: "for",
        language: "Fortran 77/Forth",
    },
    LanguageDescription {
        pattern: "forth",
        language: "Forth",
    },
    LanguageDescription {
        pattern: "fp",
        language: "GLSL",
    },
    LanguageDescription {
        pattern: "fpm",
        language: "Forth",
    },
    LanguageDescription {
        pattern: "fr",
        language: "Forth",
    },
    LanguageDescription {
        pattern: "frag",
        language: "GLSL",
    },
    LanguageDescription {
        pattern: "frg",
        language: "GLSL",
    },
    LanguageDescription {
        pattern: "frm",
        language: "Visual Basic",
    },
    LanguageDescription {
        pattern: "frt",
        language: "Forth",
    },
    LanguageDescription {
        pattern: "FRX",
        language: "Visual Basic",
    },
    LanguageDescription {
        pattern: "frx",
        language: "Visual Basic",
    },
    LanguageDescription {
        pattern: "fsh",
        language: "GLSL",
    },
    LanguageDescription {
        pattern: "fshader",
        language: "GLSL",
    },
    LanguageDescription {
        pattern: "fsl",
        language: "Finite State Language",
    },
    LanguageDescription {
        pattern: "fsproj",
        language: "XML",
    },
    LanguageDescription {
        pattern: "ft",
        language: "Forth",
    },
    LanguageDescription {
        pattern: "fth",
        language: "Forth",
    },
    LanguageDescription {
        pattern: "ftl",
        language: "Freemarker Template",
    },
    LanguageDescription {
        pattern: "FTN",
        language: "Fortran 77",
    },
    LanguageDescription {
        pattern: "ftn",
        language: "Fortran 77",
    },
    LanguageDescription {
        pattern: "fun",
        language: "Standard ML",
    },
    LanguageDescription {
        pattern: "fut",
        language: "Futhark",
    },
    LanguageDescription {
        pattern: "fxh",
        language: "HLSL",
    },
    LanguageDescription {
        pattern: "fxml",
        language: "FXML",
    },
    LanguageDescription {
        pattern: "g",
        language: "ANTLR Grammar",
    },
    LanguageDescription {
        pattern: "g4",
        language: "ANTLR Grammar",
    },
    LanguageDescription {
        pattern: "gant",
        language: "Groovy",
    },
    LanguageDescription {
        pattern: "gawk",
        language: "awk",
    },
    LanguageDescription {
        pattern: "gclient",
        language: "Python",
    },
    LanguageDescription {
        pattern: "gd",
        language: "GDScript",
    },
    LanguageDescription {
        pattern: "gdshader",
        language: "Godot Shaders",
    },
    LanguageDescription {
        pattern: "gemfile",
        language: "Ruby",
    },
    LanguageDescription {
        pattern: "gemfile.lock",
        language: "Ruby",
    },
    LanguageDescription {
        pattern: "gemrc",
        language: "YAML",
    },
    LanguageDescription {
        pattern: "gemspec",
        language: "Ruby",
    },
    LanguageDescription {
        pattern: "geo",
        language: "GLSL",
    },
    LanguageDescription {
        pattern: "geojson",
        language: "JSON",
    },
    LanguageDescription {
        pattern: "geom",
        language: "GLSL",
    },
    LanguageDescription {
        pattern: "gjs",
        language: "Glimmer JavaScript",
    },
    LanguageDescription {
        pattern: "glade",
        language: "Glade",
    },
    LanguageDescription {
        pattern: "gleam",
        language: "Gleam",
    },
    LanguageDescription {
        pattern: "glide.lock",
        language: "YAML",
    },
    LanguageDescription {
        pattern: "glsl",
        language: "GLSL",
    },
    LanguageDescription {
        pattern: "glslv",
        language: "GLSL",
    },
    LanguageDescription {
        pattern: "gltf",
        language: "JSON",
    },
    LanguageDescription {
        pattern: "gmx",
        language: "XML",
    },
    LanguageDescription {
        pattern: "Gnumakefile",
        language: "make",
    },
    LanguageDescription {
        pattern: "gnumakefile",
        language: "make",
    },
    LanguageDescription {
        pattern: "go",
        language: "Go",
    },
    LanguageDescription {
        pattern: "god",
        language: "Ruby",
    },
    LanguageDescription {
        pattern: "gql",
        language: "GraphQL",
    },
    LanguageDescription {
        pattern: "gradle",
        language: "Gradle",
    },
    LanguageDescription {
        pattern: "gradle.kts",
        language: "Gradle",
    },
    LanguageDescription {
        pattern: "graphql",
        language: "GraphQL",
    },
    LanguageDescription {
        pattern: "graphqls",
        language: "GraphQL",
    },
    LanguageDescription {
        pattern: "groovy",
        language: "Groovy",
    },
    LanguageDescription {
        pattern: "grt",
        language: "Groovy",
    },
    LanguageDescription {
        pattern: "grxml",
        language: "XML",
    },
    LanguageDescription {
        pattern: "gshader",
        language: "GLSL",
    },
    LanguageDescription {
        pattern: "gsp",
        language: "Grails",
    },
    LanguageDescription {
        pattern: "gtpl",
        language: "Groovy",
    },
    LanguageDescription {
        pattern: "gts",
        language: "Glimmer TypeScript",
    },
    LanguageDescription {
        pattern: "guardfile",
        language: "Ruby",
    },
    LanguageDescription {
        pattern: "gvy",
        language: "Groovy",
    },
    LanguageDescription {
        pattern: "gyp",
        language: "Python",
    },
    LanguageDescription {
        pattern: "gypi",
        language: "Python",
    },
    LanguageDescription {
        pattern: "H",
        language: "C/C++ Header",
    },
    LanguageDescription {
        pattern: "h",
        language: "C/C++ Header",
    },
    LanguageDescription {
        pattern: "h++",
        language: "C++",
    },
    LanguageDescription {
        pattern: "ha",
        language: "Hare",
    },
    LanguageDescription {
        pattern: "haml",
        language: "Haml",
    },
    LanguageDescription {
        pattern: "haml.deface",
        language: "Haml",
    },
    LanguageDescription {
        pattern: "handlebars",
        language: "Handlebars",
    },
    LanguageDescription {
        pattern: "har",
        language: "JSON",
    },
    LanguageDescription {
        pattern: "hb",
        language: "Harbour",
    },
    LanguageDescription {
        pattern: "hbs",
        language: "Handlebars",
    },
    LanguageDescription {
        pattern: "HC",
        language: "HolyC",
    },
    LanguageDescription {
        pattern: "hcl",
        language: "HCL",
    },
    LanguageDescription {
        pattern: "heex",
        language: "HTML EEx",
    },
    LanguageDescription {
        pattern: "hh",
        language: "C/C++ Header",
    },
    LanguageDescription {
        pattern: "hic",
        language: "Clojure",
    },
    LanguageDescription {
        pattern: "hlean",
        language: "Lean",
    },
    LanguageDescription {
        pattern: "hlsl",
        language: "HLSL",
    },
    LanguageDescription {
        pattern: "hlsli",
        language: "HLSL",
    },
    LanguageDescription {
        pattern: "hoon",
        language: "Hoon",
    },
    LanguageDescription {
        pattern: "hpp",
        language: "C/C++ Header",
    },
    LanguageDescription {
        pattern: "hrl",
        language: "Erlang",
    },
    LanguageDescription {
        pattern: "hs",
        language: "Haskell",
    },
    LanguageDescription {
        pattern: "hsc",
        language: "Haskell",
    },
    LanguageDescription {
        pattern: "htm",
        language: "HTML",
    },
    LanguageDescription {
        pattern: "html",
        language: "HTML",
    },
    LanguageDescription {
        pattern: "html.hl",
        language: "HTML",
    },
    LanguageDescription {
        pattern: "htmlhintrc",
        language: "JSON",
    },
    LanguageDescription {
        pattern: "hx",
        language: "Haxe",
    },
    LanguageDescription {
        pattern: "hxsl",
        language: "Haxe",
    },
    LanguageDescription {
        pattern: "hxx",
        language: "C/C++ Header",
    },
    LanguageDescription {
        pattern: "i",
        language: "SWIG",
    },
    LanguageDescription {
        pattern: "i3",
        language: "Modula3",
    },
    LanguageDescription {
        pattern: "ice",
        language: "Slice",
    },
    LanguageDescription {
        pattern: "iced",
        language: "CoffeeScript",
    },
    LanguageDescription {
        pattern: "icl",
        language: "Clean",
    },
    LanguageDescription {
        pattern: "idc",
        language: "C",
    },
    LanguageDescription {
        pattern: "idl",
        language: "IDL",
    },
    LanguageDescription {
        pattern: "idr",
        language: "Idris",
    },
    LanguageDescription {
        pattern: "ig",
        language: "Modula3",
    },
    LanguageDescription {
        pattern: "ihlp",
        language: "Stata",
    },
    LanguageDescription {
        pattern: "il",
        language: "SKILL/.NET IL",
    },
    LanguageDescription {
        pattern: "ils",
        language: "SKILL++",
    },
    LanguageDescription {
        pattern: "imba",
        language: "Imba",
    },
    LanguageDescription {
        pattern: "iml",
        language: "XML",
    },
    LanguageDescription {
        pattern: "in1",
        language: "TNSDL",
    },
    LanguageDescription {
        pattern: "in2",
        language: "TNSDL",
    },
    LanguageDescription {
        pattern: "in3",
        language: "TNSDL",
    },
    LanguageDescription {
        pattern: "in4",
        language: "TNSDL",
    },
    LanguageDescription {
        pattern: "inc",
        language: "PHP/Pascal/Fortran/Pawn",
    },
    LanguageDescription {
        pattern: "inf",
        language: "TNSDL",
    },
    LanguageDescription {
        pattern: "ini",
        language: "INI",
    },
    LanguageDescription {
        pattern: "inl",
        language: "C++",
    },
    LanguageDescription {
        pattern: "ino",
        language: "Arduino Sketch",
    },
    LanguageDescription {
        pattern: "ins",
        language: "TeX",
    },
    LanguageDescription {
        pattern: "interface",
        language: "TNSDL",
    },
    LanguageDescription {
        pattern: "ipf",
        language: "Igor Pro",
    },
    LanguageDescription {
        pattern: "ipl",
        language: "IPL",
    },
    LanguageDescription {
        pattern: "ipp",
        language: "C++",
    },
    LanguageDescription {
        pattern: "ipynb",
        language: "Jupyter Notebook",
    },
    LanguageDescription {
        pattern: "irbrc",
        language: "Ruby",
    },
    LanguageDescription {
        pattern: "ism",
        language: "InstallShield",
    },
    LanguageDescription {
        pattern: "itk",
        language: "Tcl/Tk",
    },
    LanguageDescription {
        pattern: "iuml",
        language: "PlantUML",
    },
    LanguageDescription {
        pattern: "ivy",
        language: "XML",
    },
    LanguageDescription {
        pattern: "ixx",
        language: "C++",
    },
    LanguageDescription {
        pattern: "j2",
        language: "Jinja Template",
    },
    LanguageDescription {
        pattern: "jade",
        language: "Pug",
    },
    LanguageDescription {
        pattern: "jai",
        language: "Jai",
    },
    LanguageDescription {
        pattern: "jake",
        language: "JavaScript",
    },
    LanguageDescription {
        pattern: "jakefile",
        language: "JavaScript",
    },
    LanguageDescription {
        pattern: "janet",
        language: "Janet",
    },
    LanguageDescription {
        pattern: "jarfile",
        language: "Ruby",
    },
    LanguageDescription {
        pattern: "java",
        language: "Java",
    },
    LanguageDescription {
        pattern: "jbuilder",
        language: "Ruby",
    },
    LanguageDescription {
        pattern: "jcl",
        language: "JCL",
    },
    LanguageDescription {
        pattern: "jelly",
        language: "XML",
    },
    LanguageDescription {
        pattern: "jenkinsfile",
        language: "Groovy",
    },
    LanguageDescription {
        pattern: "jinja",
        language: "Jinja Template",
    },
    LanguageDescription {
        pattern: "jinja2",
        language: "Jinja Template",
    },
    LanguageDescription {
        pattern: "jl",
        language: "Lisp/Julia",
    },
    LanguageDescription {
        pattern: "js",
        language: "JavaScript",
    },
    LanguageDescription {
        pattern: "jsb",
        language: "JavaScript",
    },
    LanguageDescription {
        pattern: "jscad",
        language: "JavaScript",
    },
    LanguageDescription {
        pattern: "jsf",
        language: "JavaServer Faces",
    },
    LanguageDescription {
        pattern: "jsfl",
        language: "JavaScript",
    },
    LanguageDescription {
        pattern: "jsm",
        language: "JavaScript",
    },
    LanguageDescription {
        pattern: "json",
        language: "JSON",
    },
    LanguageDescription {
        pattern: "json-tmlanguage",
        language: "JSON",
    },
    LanguageDescription {
        pattern: "json5",
        language: "JSON5",
    },
    LanguageDescription {
        pattern: "jsonl",
        language: "JSON",
    },
    LanguageDescription {
        pattern: "jsp",
        language: "JSP",
    },
    LanguageDescription {
        pattern: "jspeg",
        language: "tspeg",
    },
    LanguageDescription {
        pattern: "jspf",
        language: "JSP",
    },
    LanguageDescription {
        pattern: "jsproj",
        language: "XML",
    },
    LanguageDescription {
        pattern: "jss",
        language: "JavaScript",
    },
    LanguageDescription {
        pattern: "jssm",
        language: "Finite State Language",
    },
    LanguageDescription {
        pattern: "jsx",
        language: "JSX",
    },
    LanguageDescription {
        pattern: "junos",
        language: "Juniper Junos",
    },
    LanguageDescription {
        pattern: "kml",
        language: "XML",
    },
    LanguageDescription {
        pattern: "kojo",
        language: "Scala",
    },
    LanguageDescription {
        pattern: "ksc",
        language: "Kermit",
    },
    LanguageDescription {
        pattern: "ksh",
        language: "Korn Shell",
    },
    LanguageDescription {
        pattern: "kt",
        language: "Kotlin",
    },
    LanguageDescription {
        pattern: "ktm",
        language: "Kotlin",
    },
    LanguageDescription {
        pattern: "kts",
        language: "Kotlin",
    },
    LanguageDescription {
        pattern: "kv",
        language: "kvlang",
    },
    LanguageDescription {
        pattern: "l",
        language: "lex",
    },
    LanguageDescription {
        pattern: "lagda",
        language: "Agda",
    },
    LanguageDescription {
        pattern: "launch",
        language: "XML",
    },
    LanguageDescription {
        pattern: "lbx",
        language: "TeX",
    },
    LanguageDescription {
        pattern: "ld",
        language: "Linker Script",
    },
    LanguageDescription {
        pattern: "lean",
        language: "Lean",
    },
    LanguageDescription {
        pattern: "lektorproject",
        language: "INI",
    },
    LanguageDescription {
        pattern: "lem",
        language: "Lem",
    },
    LanguageDescription {
        pattern: "less",
        language: "LESS",
    },
    LanguageDescription {
        pattern: "lex",
        language: "lex",
    },
    LanguageDescription {
        pattern: "lfe",
        language: "LFE",
    },
    LanguageDescription {
        pattern: "lgt",
        language: "Logtalk",
    },
    LanguageDescription {
        pattern: "lhs",
        language: "Haskell",
    },
    LanguageDescription {
        pattern: "lidr",
        language: "Literate Idris",
    },
    LanguageDescription {
        pattern: "liquid",
        language: "liquid",
    },
    LanguageDescription {
        pattern: "lisp",
        language: "Lisp",
    },
    LanguageDescription {
        pattern: "lit",
        language: "PL/M",
    },
    LanguageDescription {
        pattern: "ll",
        language: "LLVM IR",
    },
    LanguageDescription {
        pattern: "lmi",
        language: "Python",
    },
    LanguageDescription {
        pattern: "logtalk",
        language: "Logtalk",
    },
    LanguageDescription {
        pattern: "lp",
        language: "AnsProlog",
    },
    LanguageDescription {
        pattern: "lpr",
        language: "Pascal",
    },
    LanguageDescription {
        pattern: "lsp",
        language: "Lisp",
    },
    LanguageDescription {
        pattern: "ltx",
        language: "TeX",
    },
    LanguageDescription {
        pattern: "lua",
        language: "Lua",
    },
    LanguageDescription {
        pattern: "luau",
        language: "Luau",
    },
    LanguageDescription {
        pattern: "m",
        language: "MATLAB/Mathematica/Objective-C/MUMPS/Mercury",
    },
    LanguageDescription {
        pattern: "m3",
        language: "Modula3",
    },
    LanguageDescription {
        pattern: "m4",
        language: "m4",
    },
    LanguageDescription {
        pattern: "ma",
        language: "Mathematica",
    },
    LanguageDescription {
        pattern: "Makefile",
        language: "make",
    },
    LanguageDescription {
        pattern: "makefile",
        language: "make",
    },
    LanguageDescription {
        pattern: "makefile.pl",
        language: "Perl",
    },
    LanguageDescription {
        pattern: "mako",
        language: "Mako",
    },
    LanguageDescription {
        pattern: "mao",
        language: "Mako",
    },
    LanguageDescription {
        pattern: "markdown",
        language: "Markdown",
    },
    LanguageDescription {
        pattern: "master",
        language: "ASP.NET",
    },
    LanguageDescription {
        pattern: "mat",
        language: "Unity-Prefab",
    },
    LanguageDescription {
        pattern: "mata",
        language: "Stata",
    },
    LanguageDescription {
        pattern: "matah",
        language: "Stata",
    },
    LanguageDescription {
        pattern: "mathematica",
        language: "Mathematica",
    },
    LanguageDescription {
        pattern: "mavenfile",
        language: "Ruby",
    },
    LanguageDescription {
        pattern: "mawk",
        language: "awk",
    },
    LanguageDescription {
        pattern: "mbt",
        language: "MoonBit",
    },
    LanguageDescription {
        pattern: "mbti",
        language: "MoonBit",
    },
    LanguageDescription {
        pattern: "mbtx",
        language: "MoonBit",
    },
    LanguageDescription {
        pattern: "mbty",
        language: "MoonBit",
    },
    LanguageDescription {
        pattern: "mc",
        language: "Windows Message File",
    },
    LanguageDescription {
        pattern: "mcmeta",
        language: "JSON",
    },
    LanguageDescription {
        pattern: "mcmod.info",
        language: "JSON",
    },
    LanguageDescription {
        pattern: "md",
        language: "Markdown",
    },
    LanguageDescription {
        pattern: "mdown",
        language: "Markdown",
    },
    LanguageDescription {
        pattern: "mdpolicy",
        language: "XML",
    },
    LanguageDescription {
        pattern: "mdwn",
        language: "Markdown",
    },
    LanguageDescription {
        pattern: "mdx",
        language: "Markdown",
    },
    LanguageDescription {
        pattern: "meson.build",
        language: "Meson",
    },
    LanguageDescription {
        pattern: "met",
        language: "Teamcenter met",
    },
    LanguageDescription {
        pattern: "metal",
        language: "Metal",
    },
    LanguageDescription {
        pattern: "mg",
        language: "Modula3",
    },
    LanguageDescription {
        pattern: "mipage",
        language: "APL",
    },
    LanguageDescription {
        pattern: "mir",
        language: "YAML",
    },
    LanguageDescription {
        pattern: "mjml",
        language: "XML",
    },
    LanguageDescription {
        pattern: "mjs",
        language: "JavaScript",
    },
    LanguageDescription {
        pattern: "mk",
        language: "make",
    },
    LanguageDescription {
        pattern: "mkd",
        language: "Markdown",
    },
    LanguageDescription {
        pattern: "mkdn",
        language: "Markdown",
    },
    LanguageDescription {
        pattern: "mkdown",
        language: "Markdown",
    },
    LanguageDescription {
        pattern: "mkii",
        language: "TeX",
    },
    LanguageDescription {
        pattern: "mkiv",
        language: "TeX",
    },
    LanguageDescription {
        pattern: "mkvi",
        language: "TeX",
    },
    LanguageDescription {
        pattern: "ml",
        language: "OCaml",
    },
    LanguageDescription {
        pattern: "ml4",
        language: "OCaml",
    },
    LanguageDescription {
        pattern: "mli",
        language: "OCaml",
    },
    LanguageDescription {
        pattern: "mll",
        language: "OCaml",
    },
    LanguageDescription {
        pattern: "mly",
        language: "OCaml",
    },
    LanguageDescription {
        pattern: "mm",
        language: "Objective-C++",
    },
    LanguageDescription {
        pattern: "mo",
        language: "Modelica",
    },
    LanguageDescription {
        pattern: "mojo",
        language: "Mojo",
    },
    LanguageDescription {
        pattern: "mojom",
        language: "Mojom",
    },
    LanguageDescription {
        pattern: "mps",
        language: "MUMPS",
    },
    LanguageDescription {
        pattern: "msbuild",
        language: "MSBuild script",
    },
    LanguageDescription {
        pattern: "msg",
        language: "Gencat NLS",
    },
    LanguageDescription {
        pattern: "mspec",
        language: "Ruby",
    },
    LanguageDescription {
        pattern: "mt",
        language: "Mathematica",
    },
    LanguageDescription {
        pattern: "mth",
        language: "Teamcenter mth",
    },
    LanguageDescription {
        pattern: "mts",
        language: "TypeScript",
    },
    LanguageDescription {
        pattern: "mustache",
        language: "Mustache",
    },
    LanguageDescription {
        pattern: "mxml",
        language: "MXML",
    },
    LanguageDescription {
        pattern: "mysql",
        language: "SQL",
    },
    LanguageDescription {
        pattern: "n",
        language: "Nemerle",
    },
    LanguageDescription {
        pattern: "nasm",
        language: "Assembly",
    },
    LanguageDescription {
        pattern: "natvis",
        language: "XML",
    },
    LanguageDescription {
        pattern: "nawk",
        language: "awk",
    },
    LanguageDescription {
        pattern: "nbp",
        language: "Mathematica",
    },
    LanguageDescription {
        pattern: "ncl",
        language: "Nickel",
    },
    LanguageDescription {
        pattern: "ndproj",
        language: "XML",
    },
    LanguageDescription {
        pattern: "nf",
        language: "Nextflow",
    },
    LanguageDescription {
        pattern: "nim",
        language: "Nim",
    },
    LanguageDescription {
        pattern: "nim.cfg",
        language: "Nim",
    },
    LanguageDescription {
        pattern: "nimble",
        language: "Nim",
    },
    LanguageDescription {
        pattern: "nimrod",
        language: "Nim",
    },
    LanguageDescription {
        pattern: "nims",
        language: "Nim",
    },
    LanguageDescription {
        pattern: "nix",
        language: "Nix",
    },
    LanguageDescription {
        pattern: "njk",
        language: "Nunjucks",
    },
    LanguageDescription {
        pattern: "njs",
        language: "JavaScript",
    },
    LanguageDescription {
        pattern: "nlogo",
        language: "NetLogo",
    },
    LanguageDescription {
        pattern: "nls",
        language: "NetLogo",
    },
    LanguageDescription {
        pattern: "nomad",
        language: "HCL",
    },
    LanguageDescription {
        pattern: "nproj",
        language: "XML",
    },
    LanguageDescription {
        pattern: "nse",
        language: "Lua",
    },
    LanguageDescription {
        pattern: "nuget.config",
        language: "XML",
    },
    LanguageDescription {
        pattern: "nuspec",
        language: "XML",
    },
    LanguageDescription {
        pattern: "nut",
        language: "Squirrel",
    },
    LanguageDescription {
        pattern: "odd",
        language: "XML",
    },
    LanguageDescription {
        pattern: "odin",
        language: "Odin",
    },
    LanguageDescription {
        pattern: "odx",
        language: "BizTalk Orchestration",
    },
    LanguageDescription {
        pattern: "oscript",
        language: "LiveLink OScript",
    },
    LanguageDescription {
        pattern: "osm",
        language: "XML",
    },
    LanguageDescription {
        pattern: "P",
        language: "Prolog",
    },
    LanguageDescription {
        pattern: "p",
        language: "Pascal/Pawn",
    },
    LanguageDescription {
        pattern: "p4",
        language: "P4",
    },
    LanguageDescription {
        pattern: "P6",
        language: "Raku/Prolog",
    },
    LanguageDescription {
        pattern: "p6",
        language: "Raku/Prolog",
    },
    LanguageDescription {
        pattern: "p8",
        language: "Lua",
    },
    LanguageDescription {
        pattern: "pac",
        language: "JavaScript",
    },
    LanguageDescription {
        pattern: "packages.config",
        language: "XML",
    },
    LanguageDescription {
        pattern: "pad",
        language: "Ada",
    },
    LanguageDescription {
        pattern: "page",
        language: "Visualforce Page",
    },
    LanguageDescription {
        pattern: "pas",
        language: "Pascal",
    },
    LanguageDescription {
        pattern: "pascal",
        language: "Pascal",
    },
    LanguageDescription {
        pattern: "patch",
        language: "diff",
    },
    LanguageDescription {
        pattern: "pawn",
        language: "Pawn",
    },
    LanguageDescription {
        pattern: "pbt",
        language: "PowerBuilder",
    },
    LanguageDescription {
        pattern: "pcc",
        language: "C++",
    },
    LanguageDescription {
        pattern: "pcl",
        language: "Patran Command Language",
    },
    LanguageDescription {
        pattern: "pd_lua",
        language: "Lua",
    },
    LanguageDescription {
        pattern: "pde",
        language: "Processing",
    },
    LanguageDescription {
        pattern: "peg",
        language: "PEG",
    },
    LanguageDescription {
        pattern: "peggy",
        language: "peggy",
    },
    LanguageDescription {
        pattern: "pegjs",
        language: "peg.js",
    },
    LanguageDescription {
        pattern: "pek",
        language: "Pek",
    },
    LanguageDescription {
        pattern: "perl",
        language: "Perl",
    },
    LanguageDescription {
        pattern: "pest",
        language: "Pest",
    },
    LanguageDescription {
        pattern: "pfo",
        language: "Fortran 77",
    },
    LanguageDescription {
        pattern: "pgc",
        language: "C",
    },
    LanguageDescription {
        pattern: "ph",
        language: "Perl",
    },
    LanguageDescription {
        pattern: "phakefile",
        language: "PHP",
    },
    LanguageDescription {
        pattern: "php",
        language: "PHP",
    },
    LanguageDescription {
        pattern: "php3",
        language: "PHP",
    },
    LanguageDescription {
        pattern: "php4",
        language: "PHP",
    },
    LanguageDescription {
        pattern: "php5",
        language: "PHP",
    },
    LanguageDescription {
        pattern: "php_cs",
        language: "PHP",
    },
    LanguageDescription {
        pattern: "php_cs.dist",
        language: "PHP",
    },
    LanguageDescription {
        pattern: "phps",
        language: "PHP",
    },
    LanguageDescription {
        pattern: "phpt",
        language: "PHP",
    },
    LanguageDescription {
        pattern: "phtml",
        language: "PHP",
    },
    LanguageDescription {
        pattern: "pig",
        language: "Pig Latin",
    },
    LanguageDescription {
        pattern: "pkgproj",
        language: "XML",
    },
    LanguageDescription {
        pattern: "pkl",
        language: "Pkl",
    },
    LanguageDescription {
        pattern: "PL",
        language: "Perl/Prolog",
    },
    LanguageDescription {
        pattern: "pl",
        language: "Perl/Prolog",
    },
    LanguageDescription {
        pattern: "pl1",
        language: "PL/I",
    },
    LanguageDescription {
        pattern: "plantuml",
        language: "PlantUML",
    },
    LanguageDescription {
        pattern: "plh",
        language: "Perl",
    },
    LanguageDescription {
        pattern: "plist",
        language: "XML",
    },
    LanguageDescription {
        pattern: "plm",
        language: "PL/M",
    },
    LanguageDescription {
        pattern: "plx",
        language: "Perl",
    },
    LanguageDescription {
        pattern: "pm",
        language: "Perl",
    },
    LanguageDescription {
        pattern: "pm6",
        language: "Raku",
    },
    LanguageDescription {
        pattern: "po",
        language: "PO File",
    },
    LanguageDescription {
        pattern: "podfile",
        language: "Ruby",
    },
    LanguageDescription {
        pattern: "podspec",
        language: "Ruby",
    },
    LanguageDescription {
        pattern: "pom",
        language: "Maven",
    },
    LanguageDescription {
        pattern: "pom.xml",
        language: "Maven",
    },
    LanguageDescription {
        pattern: "pony",
        language: "Pony",
    },
    LanguageDescription {
        pattern: "pp",
        language: "Pascal/Puppet",
    },
    LanguageDescription {
        pattern: "pprx",
        language: "Rexx",
    },
    LanguageDescription {
        pattern: "prc",
        language: "Oracle PL/SQL",
    },
    LanguageDescription {
        pattern: "prefab",
        language: "Unity-Prefab",
    },
    LanguageDescription {
        pattern: "prefs",
        language: "INI",
    },
    LanguageDescription {
        pattern: "prg",
        language: "xBase",
    },
    LanguageDescription {
        pattern: "prisma",
        language: "Prisma Schema",
    },
    LanguageDescription {
        pattern: "pro",
        language: "IDL/Qt Project/Prolog/ProGuard",
    },
    LanguageDescription {
        pattern: "proj",
        language: "XML",
    },
    LanguageDescription {
        pattern: "project",
        language: "XML",
    },
    LanguageDescription {
        pattern: "prolog",
        language: "Prolog",
    },
    LanguageDescription {
        pattern: "properties",
        language: "Properties",
    },
    LanguageDescription {
        pattern: "props",
        language: "XML",
    },
    LanguageDescription {
        pattern: "proto",
        language: "Protocol Buffers",
    },
    LanguageDescription {
        pattern: "prql",
        language: "PRQL",
    },
    LanguageDescription {
        pattern: "prw",
        language: "xBase",
    },
    LanguageDescription {
        pattern: "pryrc",
        language: "Ruby",
    },
    LanguageDescription {
        pattern: "ps1",
        language: "PowerShell",
    },
    LanguageDescription {
        pattern: "ps1xml",
        language: "XML",
    },
    LanguageDescription {
        pattern: "psc1",
        language: "XML",
    },
    LanguageDescription {
        pattern: "psd1",
        language: "PowerShell",
    },
    LanguageDescription {
        pattern: "psgi",
        language: "Perl",
    },
    LanguageDescription {
        pattern: "psm1",
        language: "PowerShell",
    },
    LanguageDescription {
        pattern: "psql",
        language: "SQL",
    },
    LanguageDescription {
        pattern: "pt",
        language: "XML",
    },
    LanguageDescription {
        pattern: "pu",
        language: "PlantUML",
    },
    LanguageDescription {
        pattern: "pug",
        language: "Pug",
    },
    LanguageDescription {
        pattern: "puml",
        language: "PlantUML",
    },
    LanguageDescription {
        pattern: "puppetfile",
        language: "Ruby",
    },
    LanguageDescription {
        pattern: "purs",
        language: "PureScript",
    },
    LanguageDescription {
        pattern: "pwn",
        language: "Pawn",
    },
    LanguageDescription {
        pattern: "pxd",
        language: "Cython",
    },
    LanguageDescription {
        pattern: "pxi",
        language: "Cython",
    },
    LanguageDescription {
        pattern: "py",
        language: "Python",
    },
    LanguageDescription {
        pattern: "py3",
        language: "Python",
    },
    LanguageDescription {
        pattern: "pyde",
        language: "Python",
    },
    LanguageDescription {
        pattern: "pyi",
        language: "Python",
    },
    LanguageDescription {
        pattern: "pyj",
        language: "RapydScript",
    },
    LanguageDescription {
        pattern: "pyp",
        language: "Python",
    },
    LanguageDescription {
        pattern: "pyt",
        language: "Python",
    },
    LanguageDescription {
        pattern: "pyw",
        language: "Python",
    },
    LanguageDescription {
        pattern: "pyx",
        language: "Cython",
    },
    LanguageDescription {
        pattern: "qbs",
        language: "QML",
    },
    LanguageDescription {
        pattern: "qml",
        language: "QML",
    },
    LanguageDescription {
        pattern: "R",
        language: "R",
    },
    LanguageDescription {
        pattern: "r",
        language: "R",
    },
    LanguageDescription {
        pattern: "rabl",
        language: "Ruby",
    },
    LanguageDescription {
        pattern: "rake",
        language: "Ruby",
    },
    LanguageDescription {
        pattern: "raku",
        language: "Raku",
    },
    LanguageDescription {
        pattern: "rakumod",
        language: "Raku",
    },
    LanguageDescription {
        pattern: "raml",
        language: "RAML",
    },
    LanguageDescription {
        pattern: "razor",
        language: "Razor",
    },
    LanguageDescription {
        pattern: "rb",
        language: "Ruby",
    },
    LanguageDescription {
        pattern: "rbuild",
        language: "Ruby",
    },
    LanguageDescription {
        pattern: "rbw",
        language: "Ruby",
    },
    LanguageDescription {
        pattern: "rbx",
        language: "Ruby",
    },
    LanguageDescription {
        pattern: "rbxs",
        language: "Lua",
    },
    LanguageDescription {
        pattern: "rc",
        language: "Windows Resource File",
    },
    LanguageDescription {
        pattern: "rc2",
        language: "Windows Resource File",
    },
    LanguageDescription {
        pattern: "rd",
        language: "R",
    },
    LanguageDescription {
        pattern: "rdf",
        language: "XML",
    },
    LanguageDescription {
        pattern: "re",
        language: "ReasonML",
    },
    LanguageDescription {
        pattern: "rebar.config",
        language: "Erlang",
    },
    LanguageDescription {
        pattern: "rebar.config.lock",
        language: "Erlang",
    },
    LanguageDescription {
        pattern: "rebar.lock",
        language: "Erlang",
    },
    LanguageDescription {
        pattern: "reek",
        language: "YAML",
    },
    LanguageDescription {
        pattern: "rei",
        language: "ReasonML",
    },
    LanguageDescription {
        pattern: "res",
        language: "ReScript",
    },
    LanguageDescription {
        pattern: "resi",
        language: "ReScript",
    },
    LanguageDescription {
        pattern: "rest",
        language: "reStructuredText",
    },
    LanguageDescription {
        pattern: "rest.txt",
        language: "reStructuredText",
    },
    LanguageDescription {
        pattern: "resx",
        language: "XML",
    },
    LanguageDescription {
        pattern: "rex",
        language: "Oracle Reports",
    },
    LanguageDescription {
        pattern: "rexfile",
        language: "Perl",
    },
    LanguageDescription {
        pattern: "rexx",
        language: "Rexx",
    },
    LanguageDescription {
        pattern: "rform",
        language: "Ring",
    },
    LanguageDescription {
        pattern: "rh",
        language: "Ring",
    },
    LanguageDescription {
        pattern: "rhtml",
        language: "Ruby HTML",
    },
    LanguageDescription {
        pattern: "riemann.config",
        language: "Clojure",
    },
    LanguageDescription {
        pattern: "ring",
        language: "Ring",
    },
    LanguageDescription {
        pattern: "rkt",
        language: "Racket",
    },
    LanguageDescription {
        pattern: "rktd",
        language: "Racket",
    },
    LanguageDescription {
        pattern: "rktl",
        language: "Racket",
    },
    LanguageDescription {
        pattern: "rlx",
        language: "Constraint Grammar",
    },
    LanguageDescription {
        pattern: "Rmd",
        language: "Rmd",
    },
    LanguageDescription {
        pattern: "robot",
        language: "RobotFramework",
    },
    LanguageDescription {
        pattern: "ronn",
        language: "Markdown",
    },
    LanguageDescription {
        pattern: "rou",
        language: "TNSDL",
    },
    LanguageDescription {
        pattern: "rprofile",
        language: "R",
    },
    LanguageDescription {
        pattern: "rs",
        language: "Rust",
    },
    LanguageDescription {
        pattern: "rs.in",
        language: "Rust",
    },
    LanguageDescription {
        pattern: "rss",
        language: "XML",
    },
    LanguageDescription {
        pattern: "rst",
        language: "reStructuredText",
    },
    LanguageDescription {
        pattern: "rst.txt",
        language: "reStructuredText",
    },
    LanguageDescription {
        pattern: "rsx",
        language: "R",
    },
    LanguageDescription {
        pattern: "ru",
        language: "Ruby",
    },
    LanguageDescription {
        pattern: "rules",
        language: "Snakemake",
    },
    LanguageDescription {
        pattern: "rviz",
        language: "YAML",
    },
    LanguageDescription {
        pattern: "rx",
        language: "Forth",
    },
    LanguageDescription {
        pattern: "S",
        language: "Assembly",
    },
    LanguageDescription {
        pattern: "s",
        language: "Assembly",
    },
    LanguageDescription {
        pattern: "sas",
        language: "SAS",
    },
    LanguageDescription {
        pattern: "sass",
        language: "Sass",
    },
    LanguageDescription {
        pattern: "SBL",
        language: "Softbridge Basic",
    },
    LanguageDescription {
        pattern: "sbl",
        language: "Softbridge Basic",
    },
    LanguageDescription {
        pattern: "sbt",
        language: "Scala",
    },
    LanguageDescription {
        pattern: "sc",
        language: "Scheme",
    },
    LanguageDescription {
        pattern: "SCA",
        language: "Visual Fox Pro",
    },
    LanguageDescription {
        pattern: "sca",
        language: "Visual Fox Pro",
    },
    LanguageDescription {
        pattern: "scad",
        language: "OpenSCAD",
    },
    LanguageDescription {
        pattern: "scala",
        language: "Scala",
    },
    LanguageDescription {
        pattern: "sch",
        language: "Scheme",
    },
    LanguageDescription {
        pattern: "scm",
        language: "Scheme",
    },
    LanguageDescription {
        pattern: "sconscript",
        language: "Python",
    },
    LanguageDescription {
        pattern: "sconstruct",
        language: "Python",
    },
    LanguageDescription {
        pattern: "scrbl",
        language: "Racket",
    },
    LanguageDescription {
        pattern: "scss",
        language: "SCSS",
    },
    LanguageDescription {
        pattern: "scxml",
        language: "XML",
    },
    LanguageDescription {
        pattern: "sdl",
        language: "TNSDL",
    },
    LanguageDescription {
        pattern: "sdt",
        language: "TNSDL",
    },
    LanguageDescription {
        pattern: "sed",
        language: "sed",
    },
    LanguageDescription {
        pattern: "ses",
        language: "Patran Command Language",
    },
    LanguageDescription {
        pattern: "settings.stylecop",
        language: "XML",
    },
    LanguageDescription {
        pattern: "sfproj",
        language: "XML",
    },
    LanguageDescription {
        pattern: "sh",
        language: "Bourne Shell",
    },
    LanguageDescription {
        pattern: "shader",
        language: "HLSL",
    },
    LanguageDescription {
        pattern: "shproj",
        language: "XML",
    },
    LanguageDescription {
        pattern: "sig",
        language: "Standard ML",
    },
    LanguageDescription {
        pattern: "sitemap",
        language: "ASP.NET",
    },
    LanguageDescription {
        pattern: "sjs",
        language: "JavaScript",
    },
    LanguageDescription {
        pattern: "sld",
        language: "Scheme",
    },
    LanguageDescription {
        pattern: "slim",
        language: "Slim",
    },
    LanguageDescription {
        pattern: "slint",
        language: "Slint",
    },
    LanguageDescription {
        pattern: "sln",
        language: "Visual Studio Solution",
    },
    LanguageDescription {
        pattern: "sls",
        language: "Scheme/SaltStack",
    },
    LanguageDescription {
        pattern: "smarty",
        language: "Smarty",
    },
    LanguageDescription {
        pattern: "smk",
        language: "Snakemake",
    },
    LanguageDescription {
        pattern: "sml",
        language: "Standard ML",
    },
    LanguageDescription {
        pattern: "snakefile",
        language: "Python",
    },
    LanguageDescription {
        pattern: "snapfile",
        language: "Ruby",
    },
    LanguageDescription {
        pattern: "sol",
        language: "Solidity",
    },
    LanguageDescription {
        pattern: "sp",
        language: "SparForte",
    },
    LanguageDescription {
        pattern: "spc",
        language: "Oracle PL/SQL",
    },
    LanguageDescription {
        pattern: "spc.sql",
        language: "SQL Stored Procedure",
    },
    LanguageDescription {
        pattern: "spd",
        language: "TNSDL",
    },
    LanguageDescription {
        pattern: "spoc.sql",
        language: "SQL Stored Procedure",
    },
    LanguageDescription {
        pattern: "sproc.sql",
        language: "SQL Stored Procedure",
    },
    LanguageDescription {
        pattern: "sps",
        language: "Scheme",
    },
    LanguageDescription {
        pattern: "SQL",
        language: "SQL",
    },
    LanguageDescription {
        pattern: "sql",
        language: "SQL",
    },
    LanguageDescription {
        pattern: "sra",
        language: "PowerBuilder",
    },
    LanguageDescription {
        pattern: "srdf",
        language: "XML",
    },
    LanguageDescription {
        pattern: "srf",
        language: "PowerBuilder",
    },
    LanguageDescription {
        pattern: "srm",
        language: "PowerBuilder",
    },
    LanguageDescription {
        pattern: "srs",
        language: "PowerBuilder",
    },
    LanguageDescription {
        pattern: "sru",
        language: "PowerBuilder",
    },
    LanguageDescription {
        pattern: "srw",
        language: "PowerBuilder",
    },
    LanguageDescription {
        pattern: "ss",
        language: "Scheme",
    },
    LanguageDescription {
        pattern: "ssc",
        language: "TNSDL",
    },
    LanguageDescription {
        pattern: "ssjs",
        language: "JavaScript",
    },
    LanguageDescription {
        pattern: "sss",
        language: "SugarSS",
    },
    LanguageDescription {
        pattern: "sst",
        language: "TNSDL",
    },
    LanguageDescription {
        pattern: "st",
        language: "Smalltalk",
    },
    LanguageDescription {
        pattern: "startup",
        language: "AMPLE",
    },
    LanguageDescription {
        pattern: "sthlp",
        language: "Stata",
    },
    LanguageDescription {
        pattern: "storyboard",
        language: "XML",
    },
    LanguageDescription {
        pattern: "sttheme",
        language: "XML",
    },
    LanguageDescription {
        pattern: "sty",
        language: "TeX",
    },
    LanguageDescription {
        pattern: "styl",
        language: "Stylus",
    },
    LanguageDescription {
        pattern: "sublime-snippet",
        language: "XML",
    },
    LanguageDescription {
        pattern: "sublime-syntax",
        language: "YAML",
    },
    LanguageDescription {
        pattern: "surql",
        language: "SurrealQL",
    },
    LanguageDescription {
        pattern: "sv",
        language: "Verilog-SystemVerilog",
    },
    LanguageDescription {
        pattern: "svelte",
        language: "Svelte",
    },
    LanguageDescription {
        pattern: "SVG",
        language: "SVG",
    },
    LanguageDescription {
        pattern: "svg",
        language: "SVG",
    },
    LanguageDescription {
        pattern: "svh",
        language: "Verilog-SystemVerilog",
    },
    LanguageDescription {
        pattern: "swift",
        language: "Swift",
    },
    LanguageDescription {
        pattern: "syntax",
        language: "YAML",
    },
    LanguageDescription {
        pattern: "tab",
        language: "SQL",
    },
    LanguageDescription {
        pattern: "tac",
        language: "Python",
    },
    LanguageDescription {
        pattern: "targets",
        language: "XML",
    },
    LanguageDescription {
        pattern: "tcc",
        language: "C++",
    },
    LanguageDescription {
        pattern: "tcl",
        language: "Tcl/Tk",
    },
    LanguageDescription {
        pattern: "tcsh",
        language: "C Shell",
    },
    LanguageDescription {
        pattern: "td",
        language: "TableGen",
    },
    LanguageDescription {
        pattern: "teal",
        language: "TEAL",
    },
    LanguageDescription {
        pattern: "templ",
        language: "Templ",
    },
    LanguageDescription {
        pattern: "tern-config",
        language: "JSON",
    },
    LanguageDescription {
        pattern: "tern-project",
        language: "JSON",
    },
    LanguageDescription {
        pattern: "tesc",
        language: "GLSL",
    },
    LanguageDescription {
        pattern: "tese",
        language: "GLSL",
    },
    LanguageDescription {
        pattern: "tex",
        language: "TeX",
    },
    LanguageDescription {
        pattern: "text",
        language: "Text",
    },
    LanguageDescription {
        pattern: "tf",
        language: "HCL",
    },
    LanguageDescription {
        pattern: "tfstate",
        language: "JSON",
    },
    LanguageDescription {
        pattern: "tfstate.backup",
        language: "JSON",
    },
    LanguageDescription {
        pattern: "tfvars",
        language: "HCL",
    },
    LanguageDescription {
        pattern: "thor",
        language: "Ruby",
    },
    LanguageDescription {
        pattern: "thorfile",
        language: "Ruby",
    },
    LanguageDescription {
        pattern: "thrift",
        language: "Thrift",
    },
    LanguageDescription {
        pattern: "tk",
        language: "Tcl/Tk",
    },
    LanguageDescription {
        pattern: "tla",
        language: "TLA+",
    },
    LanguageDescription {
        pattern: "tmcommand",
        language: "XML",
    },
    LanguageDescription {
        pattern: "tml",
        language: "XML",
    },
    LanguageDescription {
        pattern: "tmlanguage",
        language: "XML",
    },
    LanguageDescription {
        pattern: "tmpreferences",
        language: "XML",
    },
    LanguageDescription {
        pattern: "tmsnippet",
        language: "XML",
    },
    LanguageDescription {
        pattern: "tmtheme",
        language: "XML",
    },
    LanguageDescription {
        pattern: "toml",
        language: "TOML",
    },
    LanguageDescription {
        pattern: "topojson",
        language: "JSON",
    },
    LanguageDescription {
        pattern: "tpd",
        language: "TITAN Project File Information",
    },
    LanguageDescription {
        pattern: "tpl",
        language: "Smarty",
    },
    LanguageDescription {
        pattern: "tpp",
        language: "C++",
    },
    LanguageDescription {
        pattern: "tres",
        language: "Godot Resource",
    },
    LanguageDescription {
        pattern: "trg",
        language: "Oracle PL/SQL",
    },
    LanguageDescription {
        pattern: "trigger",
        language: "Apex Trigger",
    },
    LanguageDescription {
        pattern: "ts",
        language: "TypeScript/Qt Linguist",
    },
    LanguageDescription {
        pattern: "tscn",
        language: "Godot Scene",
    },
    LanguageDescription {
        pattern: "tspeg",
        language: "tspeg",
    },
    LanguageDescription {
        pattern: "tss",
        language: "Titanium Style Sheet",
    },
    LanguageDescription {
        pattern: "tsx",
        language: "TypeScript",
    },
    LanguageDescription {
        pattern: "ttcn",
        language: "TTCN",
    },
    LanguageDescription {
        pattern: "ttcn2",
        language: "TTCN",
    },
    LanguageDescription {
        pattern: "ttcn3",
        language: "TTCN",
    },
    LanguageDescription {
        pattern: "ttcnpp",
        language: "TTCN",
    },
    LanguageDescription {
        pattern: "twig",
        language: "Twig",
    },
    LanguageDescription {
        pattern: "txt",
        language: "Text",
    },
    LanguageDescription {
        pattern: "typ",
        language: "Typst",
    },
    LanguageDescription {
        pattern: "udf",
        language: "SQL",
    },
    LanguageDescription {
        pattern: "udf.sql",
        language: "SQL Stored Procedure",
    },
    LanguageDescription {
        pattern: "ui",
        language: "XML-Qt-GTK/Glade",
    },
    LanguageDescription {
        pattern: "um",
        language: "Umka",
    },
    LanguageDescription {
        pattern: "urdf",
        language: "XML",
    },
    LanguageDescription {
        pattern: "ux",
        language: "XML",
    },
    LanguageDescription {
        pattern: "v",
        language: "Verilog-SystemVerilog/Coq",
    },
    LanguageDescription {
        pattern: "vagrantfile",
        language: "Ruby",
    },
    LanguageDescription {
        pattern: "vala",
        language: "Vala",
    },
    LanguageDescription {
        pattern: "vapi",
        language: "Vala Header",
    },
    LanguageDescription {
        pattern: "VB",
        language: "Visual Basic .NET",
    },
    LanguageDescription {
        pattern: "vb",
        language: "Visual Basic .NET",
    },
    LanguageDescription {
        pattern: "VBA",
        language: "VB for Applications",
    },
    LanguageDescription {
        pattern: "vba",
        language: "VB for Applications",
    },
    LanguageDescription {
        pattern: "VBHTML",
        language: "Visual Basic",
    },
    LanguageDescription {
        pattern: "vbhtml",
        language: "Visual Basic",
    },
    LanguageDescription {
        pattern: "vbp",
        language: "Visual Basic",
    },
    LanguageDescription {
        pattern: "vbproj",
        language: "Visual Basic .NET",
    },
    LanguageDescription {
        pattern: "VBS",
        language: "Visual Basic Script",
    },
    LanguageDescription {
        pattern: "vbs",
        language: "Visual Basic Script",
    },
    LanguageDescription {
        pattern: "vbw",
        language: "Visual Basic",
    },
    LanguageDescription {
        pattern: "vcproj",
        language: "MSBuild script",
    },
    LanguageDescription {
        pattern: "vcxproj",
        language: "XML",
    },
    LanguageDescription {
        pattern: "vert",
        language: "GLSL",
    },
    LanguageDescription {
        pattern: "VHD",
        language: "VHDL",
    },
    LanguageDescription {
        pattern: "vhd",
        language: "VHDL",
    },
    LanguageDescription {
        pattern: "VHDL",
        language: "VHDL",
    },
    LanguageDescription {
        pattern: "vhdl",
        language: "VHDL",
    },
    LanguageDescription {
        pattern: "vhf",
        language: "VHDL",
    },
    LanguageDescription {
        pattern: "vhi",
        language: "VHDL",
    },
    LanguageDescription {
        pattern: "vho",
        language: "VHDL",
    },
    LanguageDescription {
        pattern: "vhs",
        language: "VHDL",
    },
    LanguageDescription {
        pattern: "vht",
        language: "VHDL",
    },
    LanguageDescription {
        pattern: "vhw",
        language: "VHDL",
    },
    LanguageDescription {
        pattern: "vim",
        language: "vim script",
    },
    LanguageDescription {
        pattern: "viw",
        language: "SQL",
    },
    LanguageDescription {
        pattern: "vm",
        language: "Velocity Template Language",
    },
    LanguageDescription {
        pattern: "vrx",
        language: "GLSL",
    },
    LanguageDescription {
        pattern: "vsh",
        language: "GLSL",
    },
    LanguageDescription {
        pattern: "vshader",
        language: "GLSL",
    },
    LanguageDescription {
        pattern: "vsixmanifest",
        language: "XML",
    },
    LanguageDescription {
        pattern: "vssettings",
        language: "XML",
    },
    LanguageDescription {
        pattern: "vstemplate",
        language: "XML",
    },
    LanguageDescription {
        pattern: "vue",
        language: "Vuejs Component",
    },
    LanguageDescription {
        pattern: "vxml",
        language: "XML",
    },
    LanguageDescription {
        pattern: "vy",
        language: "Vyper",
    },
    LanguageDescription {
        pattern: "wast",
        language: "WebAssembly",
    },
    LanguageDescription {
        pattern: "wat",
        language: "WebAssembly",
    },
    LanguageDescription {
        pattern: "watchmanconfig",
        language: "JSON",
    },
    LanguageDescription {
        pattern: "watchr",
        language: "Ruby",
    },
    LanguageDescription {
        pattern: "wdproj",
        language: "MSBuild script",
    },
    LanguageDescription {
        pattern: "web.config",
        language: "XML",
    },
    LanguageDescription {
        pattern: "web.debug.config",
        language: "XML",
    },
    LanguageDescription {
        pattern: "web.release.config",
        language: "XML",
    },
    LanguageDescription {
        pattern: "webapp",
        language: "JSON",
    },
    LanguageDescription {
        pattern: "webinfo",
        language: "ASP.NET",
    },
    LanguageDescription {
        pattern: "webmanifest",
        language: "JSON",
    },
    LanguageDescription {
        pattern: "wgsl",
        language: "WGSL",
    },
    LanguageDescription {
        pattern: "wixproj",
        language: "MSBuild script",
    },
    LanguageDescription {
        pattern: "wl",
        language: "Mathematica",
    },
    LanguageDescription {
        pattern: "wlt",
        language: "Mathematica",
    },
    LanguageDescription {
        pattern: "wlua",
        language: "Lua",
    },
    LanguageDescription {
        pattern: "workbook",
        language: "Markdown",
    },
    LanguageDescription {
        pattern: "workspace",
        language: "Python",
    },
    LanguageDescription {
        pattern: "wscript",
        language: "Python",
    },
    LanguageDescription {
        pattern: "wsd",
        language: "PlantUML",
    },
    LanguageDescription {
        pattern: "wsdl",
        language: "Web Services Description",
    },
    LanguageDescription {
        pattern: "wsf",
        language: "XML",
    },
    LanguageDescription {
        pattern: "wsgi",
        language: "Python",
    },
    LanguageDescription {
        pattern: "wxi",
        language: "WiX include",
    },
    LanguageDescription {
        pattern: "wxl",
        language: "WiX string localization",
    },
    LanguageDescription {
        pattern: "wxml",
        language: "WXML",
    },
    LanguageDescription {
        pattern: "wxs",
        language: "WiX source",
    },
    LanguageDescription {
        pattern: "wxss",
        language: "WXSS",
    },
    LanguageDescription {
        pattern: "x",
        language: "Logos",
    },
    LanguageDescription {
        pattern: "x3d",
        language: "XML",
    },
    LanguageDescription {
        pattern: "xacro",
        language: "XML",
    },
    LanguageDescription {
        pattern: "xaml",
        language: "XAML",
    },
    LanguageDescription {
        pattern: "xht",
        language: "HTML",
    },
    LanguageDescription {
        pattern: "xhtml",
        language: "XHTML",
    },
    LanguageDescription {
        pattern: "xib",
        language: "XML",
    },
    LanguageDescription {
        pattern: "xlf",
        language: "XML",
    },
    LanguageDescription {
        pattern: "xliff",
        language: "XML",
    },
    LanguageDescription {
        pattern: "xm",
        language: "Logos",
    },
    LanguageDescription {
        pattern: "XMI",
        language: "XMI",
    },
    LanguageDescription {
        pattern: "xmi",
        language: "XMI",
    },
    LanguageDescription {
        pattern: "XML",
        language: "XML",
    },
    LanguageDescription {
        pattern: "xml",
        language: "XML",
    },
    LanguageDescription {
        pattern: "xml.builder",
        language: "builder",
    },
    LanguageDescription {
        pattern: "xml.dist",
        language: "XML",
    },
    LanguageDescription {
        pattern: "xpo",
        language: "X++",
    },
    LanguageDescription {
        pattern: "xproj",
        language: "XML",
    },
    LanguageDescription {
        pattern: "xpy",
        language: "Python",
    },
    LanguageDescription {
        pattern: "xq",
        language: "XQuery",
    },
    LanguageDescription {
        pattern: "xql",
        language: "XQuery",
    },
    LanguageDescription {
        pattern: "xqm",
        language: "XQuery",
    },
    LanguageDescription {
        pattern: "xquery",
        language: "XQuery",
    },
    LanguageDescription {
        pattern: "xqy",
        language: "XQuery",
    },
    LanguageDescription {
        pattern: "xrl",
        language: "Erlang",
    },
    LanguageDescription {
        pattern: "XSD",
        language: "XSD",
    },
    LanguageDescription {
        pattern: "xsd",
        language: "XSD",
    },
    LanguageDescription {
        pattern: "xsjs",
        language: "JavaScript",
    },
    LanguageDescription {
        pattern: "xsjslib",
        language: "JavaScript",
    },
    LanguageDescription {
        pattern: "XSL",
        language: "XSLT",
    },
    LanguageDescription {
        pattern: "xsl",
        language: "XSLT",
    },
    LanguageDescription {
        pattern: "XSLT",
        language: "XSLT",
    },
    LanguageDescription {
        pattern: "xslt",
        language: "XSLT",
    },
    LanguageDescription {
        pattern: "xspec",
        language: "XML",
    },
    LanguageDescription {
        pattern: "xtend",
        language: "Xtend",
    },
    LanguageDescription {
        pattern: "xul",
        language: "XML",
    },
    LanguageDescription {
        pattern: "y",
        language: "yacc",
    },
    LanguageDescription {
        pattern: "yacc",
        language: "yacc",
    },
    LanguageDescription {
        pattern: "yaml",
        language: "YAML",
    },
    LanguageDescription {
        pattern: "yaml-tmlanguage",
        language: "YAML",
    },
    LanguageDescription {
        pattern: "yang",
        language: "Yang",
    },
    LanguageDescription {
        pattern: "yap",
        language: "Prolog",
    },
    LanguageDescription {
        pattern: "yml",
        language: "YAML",
    },
    LanguageDescription {
        pattern: "yml.mysql",
        language: "YAML",
    },
    LanguageDescription {
        pattern: "yrl",
        language: "Erlang",
    },
    LanguageDescription {
        pattern: "yyp",
        language: "JSON",
    },
    LanguageDescription {
        pattern: "zcml",
        language: "XML",
    },
    LanguageDescription {
        pattern: "zig",
        language: "Zig",
    },
    LanguageDescription {
        pattern: "zsh",
        language: "zsh",
    },
    LanguageDescription {
        pattern: "ʕ◔ϖ◔ʔ",
        language: "Go",
    },
    LanguageDescription {
        pattern: "🔥",
        language: "Mojo",
    },
];

#[allow(dead_code)]
pub const FILENAME_LANGUAGES: &[LanguageDescription] = &[
    LanguageDescription {
        pattern: "BUILD",
        language: "Bazel",
    },
    LanguageDescription {
        pattern: "build.xml",
        language: "Ant/XML",
    },
    LanguageDescription {
        pattern: "CMakeLists.txt",
        language: "CMake",
    },
    LanguageDescription {
        pattern: "cmakelists.txt",
        language: "CMake",
    },
    LanguageDescription {
        pattern: "Containerfile",
        language: "Containerfile",
    },
    LanguageDescription {
        pattern: "Dockerfile",
        language: "Dockerfile",
    },
    LanguageDescription {
        pattern: "dockerfile",
        language: "Dockerfile",
    },
    LanguageDescription {
        pattern: "Dockerfile.cmake",
        language: "Dockerfile",
    },
    LanguageDescription {
        pattern: "dockerfile.cmake",
        language: "Dockerfile",
    },
    LanguageDescription {
        pattern: "Dockerfile.m4",
        language: "Dockerfile",
    },
    LanguageDescription {
        pattern: "dockerfile.m4",
        language: "Dockerfile",
    },
    LanguageDescription {
        pattern: "Gnumakefile",
        language: "make",
    },
    LanguageDescription {
        pattern: "gnumakefile",
        language: "make",
    },
    LanguageDescription {
        pattern: "Jamfile",
        language: "Jam",
    },
    LanguageDescription {
        pattern: "jamfile",
        language: "Jam",
    },
    LanguageDescription {
        pattern: "Jamrules",
        language: "Jam",
    },
    LanguageDescription {
        pattern: "Makefile",
        language: "make",
    },
    LanguageDescription {
        pattern: "makefile",
        language: "make",
    },
    LanguageDescription {
        pattern: "meson.build",
        language: "Meson",
    },
    LanguageDescription {
        pattern: "pom.xml",
        language: "Maven/XML",
    },
    LanguageDescription {
        pattern: "Rakefile",
        language: "Ruby",
    },
    LanguageDescription {
        pattern: "rakefile",
        language: "Ruby",
    },
    LanguageDescription {
        pattern: "Snakefile",
        language: "Snakemake",
    },
    LanguageDescription {
        pattern: "WORKSPACE",
        language: "Bazel",
    },
];

#[allow(dead_code)]
pub const PREFIX_LANGUAGES: &[LanguageDescription] = &[
    LanguageDescription {
        pattern: "Containerfile",
        language: "Containerfile",
    },
    LanguageDescription {
        pattern: "Dockerfile",
        language: "Dockerfile",
    },
];

pub fn language_for_extension(pattern: &str) -> Option<&'static str> {
    match pattern {
        "4th" => Some("Forth"),
        "_coffee" => Some("CoffeeScript"),
        "_js" => Some("JavaScript"),
        "a51" => Some("Assembly"),
        "abap" => Some("ABAP"),
        "ac" => Some("m4"),
        "ack" => Some("Perl"),
        "ada" => Some("Ada"),
        "adb" => Some("Ada"),
        "adml" => Some("XML"),
        "admx" => Some("XML"),
        "ado" => Some("Stata"),
        "adoc" => Some("AsciiDoc"),
        "ads" => Some("Ada"),
        "adso" => Some("ADSO/IDSM"),
        "agda" => Some("Agda"),
        "ahk" => Some("AutoHotkey"),
        "ahkl" => Some("AutoHotkey"),
        "aj" => Some("AspectJ"),
        "al" => Some("Perl"),
        "am" => Some("make"),
        "ample" => Some("AMPLE"),
        "ant" => Some("XML"),
        "apl" => Some("APL"),
        "apla" => Some("APL"),
        "aplc" => Some("APL"),
        "aplf" => Some("APL"),
        "apli" => Some("APL"),
        "apln" => Some("APL"),
        "aplo" => Some("APL"),
        "app.config" => Some("XML"),
        "app.src" => Some("Erlang"),
        "applescript" => Some("AppleScript"),
        "appraisals" => Some("Ruby"),
        "arcconfig" => Some("JSON"),
        "art" => Some("Arturo"),
        "as" => Some("ActionScript"),
        "asa" => Some("ASP"),
        "asax" => Some("ASP.NET"),
        "asciidoc" => Some("AsciiDoc"),
        "ascx" => Some("ASP.NET"),
        "asd" => Some("Lisp"),
        "ashx" => Some("ASP"),
        "asm" => Some("Assembly"),
        "asmx" => Some("ASP.NET"),
        "asp" => Some("ASP"),
        "aspx" => Some("ASP.NET"),
        "astro" => Some("Astro"),
        "asy" => Some("Asymptote"),
        "auk" => Some("awk"),
        "aux" => Some("TeX"),
        "avsc" => Some("JSON"),
        "aw" => Some("PHP"),
        "awk" => Some("awk"),
        "axd" => Some("ASP"),
        "axml" => Some("XML"),
        "b" => Some("Brainfuck"),
        "BAS" => Some("Visual Basic"),
        "bas" => Some("Visual Basic"),
        "bash" => Some("Bourne Again Shell"),
        "BAT" => Some("DOS Batch"),
        "bat" => Some("DOS Batch"),
        "bazel" => Some("Starlark"),
        "bbx" => Some("TeX"),
        "bdy" => Some("Oracle PL/SQL"),
        "berksfile" => Some("Ruby"),
        "bf" => Some("Brainfuck"),
        "bib" => Some("TeX"),
        "blade" => Some("Blade"),
        "blade.php" => Some("Blade"),
        "blp" => Some("Blueprint"),
        "bod" => Some("Oracle PL/SQL"),
        "bones" => Some("JavaScript"),
        "boot" => Some("Clojure"),
        "brewfile" => Some("Ruby"),
        "brs" => Some("BrightScript"),
        "bst" => Some("TeX"),
        "BTM" => Some("DOS Batch"),
        "btm" => Some("DOS Batch"),
        "btp" => Some("BizTalk Pipeline"),
        "btproj" => Some("MSBuild script"),
        "buck" => Some("Python"),
        "BUILD" => Some("Bazel"),
        "build" => Some("NAnt script"),
        "build.bazel" => Some("Python"),
        "build.xml" => Some("Ant"),
        "builder" => Some("Ruby"),
        "buildfile" => Some("Ruby"),
        "buildozer.spec" => Some("INI"),
        "builds" => Some("XML"),
        "bzl" => Some("Starlark"),
        "C" => Some("C++"),
        "c" => Some("C"),
        "c++" => Some("C++"),
        "c++m" => Some("C++"),
        "c5" => Some("CoCoA 5"),
        "cairo" => Some("Cairo"),
        "cake" => Some("Cake Build Script"),
        "cakefile" => Some("CoffeeScript"),
        "capfile" => Some("Ruby"),
        "carbon" => Some("Carbon"),
        "cats" => Some("C"),
        "CBL" => Some("COBOL"),
        "cbl" => Some("COBOL"),
        "cbx" => Some("TeX"),
        "cc" => Some("C++"),
        "ccm" => Some("C++"),
        "ccp" => Some("COBOL"),
        "ccproj" => Some("XML"),
        "ccs" => Some("CCS"),
        "ccxml" => Some("XML"),
        "cdc" => Some("Cadence"),
        "cdf" => Some("Mathematica"),
        "cfc" => Some("ColdFusion CFScript"),
        "cfm" => Some("ColdFusion"),
        "cfml" => Some("ColdFusion"),
        "cg" => Some("HLSL"),
        "cg3" => Some("Constraint Grammar"),
        "cginc" => Some("HLSL"),
        "ch" => Some("xBase Header"),
        "chpl" => Some("Chapel"),
        "cii" => Some("TNSDL"),
        "cin" => Some("TNSDL"),
        "circom" => Some("Circom"),
        "civet" => Some("Civet"),
        "cj" => Some("Clojure/Cangjie"),
        "cjs" => Some("JavaScript"),
        "cjsx" => Some("CoffeeScript"),
        "cl" => Some("Lisp/OpenCL"),
        "cl2" => Some("Clojure"),
        "clang-format" => Some("YAML"),
        "clang-tidy" => Some("YAML"),
        "classpath" => Some("XML"),
        "clixml" => Some("XML"),
        "clj" => Some("Clojure"),
        "cljc" => Some("ClojureC"),
        "cljs" => Some("ClojureScript"),
        "cljs.hl" => Some("Clojure"),
        "cljscm" => Some("Clojure"),
        "cljx" => Some("Clojure"),
        "cls" => Some("Visual Basic/TeX/Apex Class"),
        "cmake" => Some("CMake"),
        "cmake.in" => Some("CMake"),
        "CMakeLists.txt" => Some("CMake"),
        "CMD" => Some("DOS Batch"),
        "cmd" => Some("DOS Batch"),
        "COB" => Some("COBOL"),
        "cob" => Some("COBOL"),
        "cobol" => Some("COBOL"),
        "cocoa5" => Some("CoCoA 5"),
        "cocoa5server" => Some("CoCoA 5"),
        "coffee" => Some("CoffeeScript"),
        "comp" => Some("GLSL"),
        "component" => Some("Visualforce Component"),
        "composer.lock" => Some("JSON"),
        "Containerfile" => Some("Containerfile"),
        "contents.lr" => Some("Markdown"),
        "cpanfile" => Some("Perl"),
        "cpkg5" => Some("CoCoA 5"),
        "CPP" => Some("C++"),
        "cpp" => Some("C++"),
        "cppm" => Some("C++"),
        "cproject" => Some("XML"),
        "cpy" => Some("COBOL"),
        "cql" => Some("SQL"),
        "cr" => Some("Crystal"),
        "cscfg" => Some("XML"),
        "csdef" => Some("XML"),
        "csh" => Some("C Shell"),
        "cshtml" => Some("Razor"),
        "csl" => Some("XML"),
        "cson" => Some("CSON"),
        "csproj" => Some("MSBuild script"),
        "css" => Some("CSS"),
        "csv" => Some("CSV"),
        "ct" => Some("XML"),
        "ctl" => Some("Visual Basic"),
        "ctp" => Some("PHP"),
        "cu" => Some("CUDA"),
        "cuh" => Some("CUDA"),
        "cvt" => Some("Civet"),
        "cvtx" => Some("Civet"),
        "cxx" => Some("C++"),
        "cxxm" => Some("C++"),
        "d" => Some("D/dtrace"),
        "da" => Some("DAL"),
        "dangerfile" => Some("Ruby"),
        "dart" => Some("Dart"),
        "data.sql" => Some("SQL Data"),
        "dcl" => Some("Clean"),
        "def" => Some("Windows Module Definition"),
        "deliverfile" => Some("Ruby"),
        "depproj" => Some("XML"),
        "derw" => Some("Derw"),
        "dfm" => Some("Delphi Form"),
        "dfy" => Some("Dafny"),
        "dhall" => Some("dhall"),
        "diff" => Some("diff"),
        "dita" => Some("DITA"),
        "ditamap" => Some("XML"),
        "ditaval" => Some("XML"),
        "dll.config" => Some("XML"),
        "dlm" => Some("IDL"),
        "dmap" => Some("NASTRAN DMAP"),
        "DO" => Some("Stata"),
        "do" => Some("Stata"),
        "Dockerfile" => Some("Dockerfile"),
        "dockerfile" => Some("Dockerfile"),
        "dofile" => Some("AMPLE"),
        "doh" => Some("Stata"),
        "dotsettings" => Some("XML"),
        "dpr" => Some("Pascal"),
        "drl" => Some("Drools"),
        "dsc" => Some("DenizenScript"),
        "dsr" => Some("Visual Basic"),
        "dt" => Some("DIET"),
        "dtd" => Some("DTD"),
        "dtx" => Some("TeX"),
        "dxl" => Some("DOORS Extension Language"),
        "dyalog" => Some("APL"),
        "dyapp" => Some("APL"),
        "e" => Some("Specman e"),
        "e4" => Some("Forth"),
        "ec" => Some("C"),
        "ecpp" => Some("ECPP"),
        "ecr" => Some("Embedded Crystal"),
        "editorconfig" => Some("INI"),
        "eex" => Some("EEx"),
        "ejs" => Some("EJS"),
        "el" => Some("Lisp"),
        "eliom" => Some("OCaml"),
        "eliomi" => Some("OCaml"),
        "elm" => Some("Elm"),
        "emakefile" => Some("Erlang"),
        "ERB" => Some("ERB"),
        "erb" => Some("ERB"),
        "erl" => Some("Erlang"),
        "es6" => Some("JavaScript"),
        "ets" => Some("ArkTs"),
        "ex" => Some("Elixir"),
        "exp" => Some("Expect"),
        "expr-dist" => Some("R"),
        "exs" => Some("Elixir Script"),
        "eye" => Some("Ruby"),
        "F" => Some("Fortran 77"),
        "f" => Some("Fortran 77/Forth"),
        "F77" => Some("Fortran 77"),
        "f77" => Some("Fortran 77"),
        "f83" => Some("Forth"),
        "F90" => Some("Fortran 90"),
        "f90" => Some("Fortran 90"),
        "F95" => Some("Fortran 95"),
        "f95" => Some("Fortran 95"),
        "fastfile" => Some("Ruby"),
        "fb" => Some("Forth"),
        "fbs" => Some("Flatbuffers"),
        "feature" => Some("Cucumber"),
        "filters" => Some("XML"),
        "fish" => Some("Fish Shell"),
        "fmt" => Some("Oracle Forms"),
        "fnc" => Some("Oracle PL/SQL"),
        "fnl" => Some("Fennel"),
        "focexec" => Some("Focus"),
        "FOR" => Some("Fortran 77"),
        "for" => Some("Fortran 77/Forth"),
        "forth" => Some("Forth"),
        "fp" => Some("GLSL"),
        "fpm" => Some("Forth"),
        "fr" => Some("Forth"),
        "frag" => Some("GLSL"),
        "frg" => Some("GLSL"),
        "frm" => Some("Visual Basic"),
        "frt" => Some("Forth"),
        "FRX" => Some("Visual Basic"),
        "frx" => Some("Visual Basic"),
        "fsh" => Some("GLSL"),
        "fshader" => Some("GLSL"),
        "fsl" => Some("Finite State Language"),
        "fsproj" => Some("XML"),
        "ft" => Some("Forth"),
        "fth" => Some("Forth"),
        "ftl" => Some("Freemarker Template"),
        "FTN" => Some("Fortran 77"),
        "ftn" => Some("Fortran 77"),
        "fun" => Some("Standard ML"),
        "fut" => Some("Futhark"),
        "fxh" => Some("HLSL"),
        "fxml" => Some("FXML"),
        "g" => Some("ANTLR Grammar"),
        "g4" => Some("ANTLR Grammar"),
        "gant" => Some("Groovy"),
        "gawk" => Some("awk"),
        "gclient" => Some("Python"),
        "gd" => Some("GDScript"),
        "gdshader" => Some("Godot Shaders"),
        "gemfile" => Some("Ruby"),
        "gemfile.lock" => Some("Ruby"),
        "gemrc" => Some("YAML"),
        "gemspec" => Some("Ruby"),
        "geo" => Some("GLSL"),
        "geojson" => Some("JSON"),
        "geom" => Some("GLSL"),
        "gjs" => Some("Glimmer JavaScript"),
        "glade" => Some("Glade"),
        "gleam" => Some("Gleam"),
        "glide.lock" => Some("YAML"),
        "glsl" => Some("GLSL"),
        "glslv" => Some("GLSL"),
        "gltf" => Some("JSON"),
        "gmx" => Some("XML"),
        "Gnumakefile" => Some("make"),
        "gnumakefile" => Some("make"),
        "go" => Some("Go"),
        "god" => Some("Ruby"),
        "gql" => Some("GraphQL"),
        "gradle" => Some("Gradle"),
        "gradle.kts" => Some("Gradle"),
        "graphql" => Some("GraphQL"),
        "graphqls" => Some("GraphQL"),
        "groovy" => Some("Groovy"),
        "grt" => Some("Groovy"),
        "grxml" => Some("XML"),
        "gshader" => Some("GLSL"),
        "gsp" => Some("Grails"),
        "gtpl" => Some("Groovy"),
        "gts" => Some("Glimmer TypeScript"),
        "guardfile" => Some("Ruby"),
        "gvy" => Some("Groovy"),
        "gyp" => Some("Python"),
        "gypi" => Some("Python"),
        "H" => Some("C/C++ Header"),
        "h" => Some("C/C++ Header"),
        "h++" => Some("C++"),
        "ha" => Some("Hare"),
        "haml" => Some("Haml"),
        "haml.deface" => Some("Haml"),
        "handlebars" => Some("Handlebars"),
        "har" => Some("JSON"),
        "hb" => Some("Harbour"),
        "hbs" => Some("Handlebars"),
        "HC" => Some("HolyC"),
        "hcl" => Some("HCL"),
        "heex" => Some("HTML EEx"),
        "hh" => Some("C/C++ Header"),
        "hic" => Some("Clojure"),
        "hlean" => Some("Lean"),
        "hlsl" => Some("HLSL"),
        "hlsli" => Some("HLSL"),
        "hoon" => Some("Hoon"),
        "hpp" => Some("C/C++ Header"),
        "hrl" => Some("Erlang"),
        "hs" => Some("Haskell"),
        "hsc" => Some("Haskell"),
        "htm" => Some("HTML"),
        "html" => Some("HTML"),
        "html.hl" => Some("HTML"),
        "htmlhintrc" => Some("JSON"),
        "hx" => Some("Haxe"),
        "hxsl" => Some("Haxe"),
        "hxx" => Some("C/C++ Header"),
        "i" => Some("SWIG"),
        "i3" => Some("Modula3"),
        "ice" => Some("Slice"),
        "iced" => Some("CoffeeScript"),
        "icl" => Some("Clean"),
        "idc" => Some("C"),
        "idl" => Some("IDL"),
        "idr" => Some("Idris"),
        "ig" => Some("Modula3"),
        "ihlp" => Some("Stata"),
        "il" => Some("SKILL/.NET IL"),
        "ils" => Some("SKILL++"),
        "imba" => Some("Imba"),
        "iml" => Some("XML"),
        "in1" => Some("TNSDL"),
        "in2" => Some("TNSDL"),
        "in3" => Some("TNSDL"),
        "in4" => Some("TNSDL"),
        "inc" => Some("PHP/Pascal/Fortran/Pawn"),
        "inf" => Some("TNSDL"),
        "ini" => Some("INI"),
        "inl" => Some("C++"),
        "ino" => Some("Arduino Sketch"),
        "ins" => Some("TeX"),
        "interface" => Some("TNSDL"),
        "ipf" => Some("Igor Pro"),
        "ipl" => Some("IPL"),
        "ipp" => Some("C++"),
        "ipynb" => Some("Jupyter Notebook"),
        "irbrc" => Some("Ruby"),
        "ism" => Some("InstallShield"),
        "itk" => Some("Tcl/Tk"),
        "iuml" => Some("PlantUML"),
        "ivy" => Some("XML"),
        "ixx" => Some("C++"),
        "j2" => Some("Jinja Template"),
        "jade" => Some("Pug"),
        "jai" => Some("Jai"),
        "jake" => Some("JavaScript"),
        "jakefile" => Some("JavaScript"),
        "janet" => Some("Janet"),
        "jarfile" => Some("Ruby"),
        "java" => Some("Java"),
        "jbuilder" => Some("Ruby"),
        "jcl" => Some("JCL"),
        "jelly" => Some("XML"),
        "jenkinsfile" => Some("Groovy"),
        "jinja" => Some("Jinja Template"),
        "jinja2" => Some("Jinja Template"),
        "jl" => Some("Lisp/Julia"),
        "js" => Some("JavaScript"),
        "jsb" => Some("JavaScript"),
        "jscad" => Some("JavaScript"),
        "jsf" => Some("JavaServer Faces"),
        "jsfl" => Some("JavaScript"),
        "jsm" => Some("JavaScript"),
        "json" => Some("JSON"),
        "json-tmlanguage" => Some("JSON"),
        "json5" => Some("JSON5"),
        "jsonl" => Some("JSON"),
        "jsp" => Some("JSP"),
        "jspeg" => Some("tspeg"),
        "jspf" => Some("JSP"),
        "jsproj" => Some("XML"),
        "jss" => Some("JavaScript"),
        "jssm" => Some("Finite State Language"),
        "jsx" => Some("JSX"),
        "junos" => Some("Juniper Junos"),
        "kml" => Some("XML"),
        "kojo" => Some("Scala"),
        "ksc" => Some("Kermit"),
        "ksh" => Some("Korn Shell"),
        "kt" => Some("Kotlin"),
        "ktm" => Some("Kotlin"),
        "kts" => Some("Kotlin"),
        "kv" => Some("kvlang"),
        "l" => Some("lex"),
        "lagda" => Some("Agda"),
        "launch" => Some("XML"),
        "lbx" => Some("TeX"),
        "ld" => Some("Linker Script"),
        "lean" => Some("Lean"),
        "lektorproject" => Some("INI"),
        "lem" => Some("Lem"),
        "less" => Some("LESS"),
        "lex" => Some("lex"),
        "lfe" => Some("LFE"),
        "lgt" => Some("Logtalk"),
        "lhs" => Some("Haskell"),
        "lidr" => Some("Literate Idris"),
        "liquid" => Some("liquid"),
        "lisp" => Some("Lisp"),
        "lit" => Some("PL/M"),
        "ll" => Some("LLVM IR"),
        "lmi" => Some("Python"),
        "logtalk" => Some("Logtalk"),
        "lp" => Some("AnsProlog"),
        "lpr" => Some("Pascal"),
        "lsp" => Some("Lisp"),
        "ltx" => Some("TeX"),
        "lua" => Some("Lua"),
        "luau" => Some("Luau"),
        "m" => Some("MATLAB/Mathematica/Objective-C/MUMPS/Mercury"),
        "m3" => Some("Modula3"),
        "m4" => Some("m4"),
        "ma" => Some("Mathematica"),
        "Makefile" => Some("make"),
        "makefile" => Some("make"),
        "makefile.pl" => Some("Perl"),
        "mako" => Some("Mako"),
        "mao" => Some("Mako"),
        "markdown" => Some("Markdown"),
        "master" => Some("ASP.NET"),
        "mat" => Some("Unity-Prefab"),
        "mata" => Some("Stata"),
        "matah" => Some("Stata"),
        "mathematica" => Some("Mathematica"),
        "mavenfile" => Some("Ruby"),
        "mawk" => Some("awk"),
        "mbt" => Some("MoonBit"),
        "mbti" => Some("MoonBit"),
        "mbtx" => Some("MoonBit"),
        "mbty" => Some("MoonBit"),
        "mc" => Some("Windows Message File"),
        "mcmeta" => Some("JSON"),
        "mcmod.info" => Some("JSON"),
        "md" => Some("Markdown"),
        "mdown" => Some("Markdown"),
        "mdpolicy" => Some("XML"),
        "mdwn" => Some("Markdown"),
        "mdx" => Some("Markdown"),
        "meson.build" => Some("Meson"),
        "met" => Some("Teamcenter met"),
        "metal" => Some("Metal"),
        "mg" => Some("Modula3"),
        "mipage" => Some("APL"),
        "mir" => Some("YAML"),
        "mjml" => Some("XML"),
        "mjs" => Some("JavaScript"),
        "mk" => Some("make"),
        "mkd" => Some("Markdown"),
        "mkdn" => Some("Markdown"),
        "mkdown" => Some("Markdown"),
        "mkii" => Some("TeX"),
        "mkiv" => Some("TeX"),
        "mkvi" => Some("TeX"),
        "ml" => Some("OCaml"),
        "ml4" => Some("OCaml"),
        "mli" => Some("OCaml"),
        "mll" => Some("OCaml"),
        "mly" => Some("OCaml"),
        "mm" => Some("Objective-C++"),
        "mo" => Some("Modelica"),
        "mojo" => Some("Mojo"),
        "mojom" => Some("Mojom"),
        "mps" => Some("MUMPS"),
        "msbuild" => Some("MSBuild script"),
        "msg" => Some("Gencat NLS"),
        "mspec" => Some("Ruby"),
        "mt" => Some("Mathematica"),
        "mth" => Some("Teamcenter mth"),
        "mts" => Some("TypeScript"),
        "mustache" => Some("Mustache"),
        "mxml" => Some("MXML"),
        "mysql" => Some("SQL"),
        "n" => Some("Nemerle"),
        "nasm" => Some("Assembly"),
        "natvis" => Some("XML"),
        "nawk" => Some("awk"),
        "nbp" => Some("Mathematica"),
        "ncl" => Some("Nickel"),
        "ndproj" => Some("XML"),
        "nf" => Some("Nextflow"),
        "nim" => Some("Nim"),
        "nim.cfg" => Some("Nim"),
        "nimble" => Some("Nim"),
        "nimrod" => Some("Nim"),
        "nims" => Some("Nim"),
        "nix" => Some("Nix"),
        "njk" => Some("Nunjucks"),
        "njs" => Some("JavaScript"),
        "nlogo" => Some("NetLogo"),
        "nls" => Some("NetLogo"),
        "nomad" => Some("HCL"),
        "nproj" => Some("XML"),
        "nse" => Some("Lua"),
        "nuget.config" => Some("XML"),
        "nuspec" => Some("XML"),
        "nut" => Some("Squirrel"),
        "odd" => Some("XML"),
        "odin" => Some("Odin"),
        "odx" => Some("BizTalk Orchestration"),
        "oscript" => Some("LiveLink OScript"),
        "osm" => Some("XML"),
        "P" => Some("Prolog"),
        "p" => Some("Pascal/Pawn"),
        "p4" => Some("P4"),
        "P6" => Some("Raku/Prolog"),
        "p6" => Some("Raku/Prolog"),
        "p8" => Some("Lua"),
        "pac" => Some("JavaScript"),
        "packages.config" => Some("XML"),
        "pad" => Some("Ada"),
        "page" => Some("Visualforce Page"),
        "pas" => Some("Pascal"),
        "pascal" => Some("Pascal"),
        "patch" => Some("diff"),
        "pawn" => Some("Pawn"),
        "pbt" => Some("PowerBuilder"),
        "pcc" => Some("C++"),
        "pcl" => Some("Patran Command Language"),
        "pd_lua" => Some("Lua"),
        "pde" => Some("Processing"),
        "peg" => Some("PEG"),
        "peggy" => Some("peggy"),
        "pegjs" => Some("peg.js"),
        "pek" => Some("Pek"),
        "perl" => Some("Perl"),
        "pest" => Some("Pest"),
        "pfo" => Some("Fortran 77"),
        "pgc" => Some("C"),
        "ph" => Some("Perl"),
        "phakefile" => Some("PHP"),
        "php" => Some("PHP"),
        "php3" => Some("PHP"),
        "php4" => Some("PHP"),
        "php5" => Some("PHP"),
        "php_cs" => Some("PHP"),
        "php_cs.dist" => Some("PHP"),
        "phps" => Some("PHP"),
        "phpt" => Some("PHP"),
        "phtml" => Some("PHP"),
        "pig" => Some("Pig Latin"),
        "pkgproj" => Some("XML"),
        "pkl" => Some("Pkl"),
        "PL" => Some("Perl/Prolog"),
        "pl" => Some("Perl/Prolog"),
        "pl1" => Some("PL/I"),
        "plantuml" => Some("PlantUML"),
        "plh" => Some("Perl"),
        "plist" => Some("XML"),
        "plm" => Some("PL/M"),
        "plx" => Some("Perl"),
        "pm" => Some("Perl"),
        "pm6" => Some("Raku"),
        "po" => Some("PO File"),
        "podfile" => Some("Ruby"),
        "podspec" => Some("Ruby"),
        "pom" => Some("Maven"),
        "pom.xml" => Some("Maven"),
        "pony" => Some("Pony"),
        "pp" => Some("Pascal/Puppet"),
        "pprx" => Some("Rexx"),
        "prc" => Some("Oracle PL/SQL"),
        "prefab" => Some("Unity-Prefab"),
        "prefs" => Some("INI"),
        "prg" => Some("xBase"),
        "prisma" => Some("Prisma Schema"),
        "pro" => Some("IDL/Qt Project/Prolog/ProGuard"),
        "proj" => Some("XML"),
        "project" => Some("XML"),
        "prolog" => Some("Prolog"),
        "properties" => Some("Properties"),
        "props" => Some("XML"),
        "proto" => Some("Protocol Buffers"),
        "prql" => Some("PRQL"),
        "prw" => Some("xBase"),
        "pryrc" => Some("Ruby"),
        "ps1" => Some("PowerShell"),
        "ps1xml" => Some("XML"),
        "psc1" => Some("XML"),
        "psd1" => Some("PowerShell"),
        "psgi" => Some("Perl"),
        "psm1" => Some("PowerShell"),
        "psql" => Some("SQL"),
        "pt" => Some("XML"),
        "pu" => Some("PlantUML"),
        "pug" => Some("Pug"),
        "puml" => Some("PlantUML"),
        "puppetfile" => Some("Ruby"),
        "purs" => Some("PureScript"),
        "pwn" => Some("Pawn"),
        "pxd" => Some("Cython"),
        "pxi" => Some("Cython"),
        "py" => Some("Python"),
        "py3" => Some("Python"),
        "pyde" => Some("Python"),
        "pyi" => Some("Python"),
        "pyj" => Some("RapydScript"),
        "pyp" => Some("Python"),
        "pyt" => Some("Python"),
        "pyw" => Some("Python"),
        "pyx" => Some("Cython"),
        "qbs" => Some("QML"),
        "qml" => Some("QML"),
        "R" => Some("R"),
        "r" => Some("R"),
        "rabl" => Some("Ruby"),
        "rake" => Some("Ruby"),
        "raku" => Some("Raku"),
        "rakumod" => Some("Raku"),
        "raml" => Some("RAML"),
        "razor" => Some("Razor"),
        "rb" => Some("Ruby"),
        "rbuild" => Some("Ruby"),
        "rbw" => Some("Ruby"),
        "rbx" => Some("Ruby"),
        "rbxs" => Some("Lua"),
        "rc" => Some("Windows Resource File"),
        "rc2" => Some("Windows Resource File"),
        "rd" => Some("R"),
        "rdf" => Some("XML"),
        "re" => Some("ReasonML"),
        "rebar.config" => Some("Erlang"),
        "rebar.config.lock" => Some("Erlang"),
        "rebar.lock" => Some("Erlang"),
        "reek" => Some("YAML"),
        "rei" => Some("ReasonML"),
        "res" => Some("ReScript"),
        "resi" => Some("ReScript"),
        "rest" => Some("reStructuredText"),
        "rest.txt" => Some("reStructuredText"),
        "resx" => Some("XML"),
        "rex" => Some("Oracle Reports"),
        "rexfile" => Some("Perl"),
        "rexx" => Some("Rexx"),
        "rform" => Some("Ring"),
        "rh" => Some("Ring"),
        "rhtml" => Some("Ruby HTML"),
        "riemann.config" => Some("Clojure"),
        "ring" => Some("Ring"),
        "rkt" => Some("Racket"),
        "rktd" => Some("Racket"),
        "rktl" => Some("Racket"),
        "rlx" => Some("Constraint Grammar"),
        "Rmd" => Some("Rmd"),
        "robot" => Some("RobotFramework"),
        "ronn" => Some("Markdown"),
        "rou" => Some("TNSDL"),
        "rprofile" => Some("R"),
        "rs" => Some("Rust"),
        "rs.in" => Some("Rust"),
        "rss" => Some("XML"),
        "rst" => Some("reStructuredText"),
        "rst.txt" => Some("reStructuredText"),
        "rsx" => Some("R"),
        "ru" => Some("Ruby"),
        "rules" => Some("Snakemake"),
        "rviz" => Some("YAML"),
        "rx" => Some("Forth"),
        "S" => Some("Assembly"),
        "s" => Some("Assembly"),
        "sas" => Some("SAS"),
        "sass" => Some("Sass"),
        "SBL" => Some("Softbridge Basic"),
        "sbl" => Some("Softbridge Basic"),
        "sbt" => Some("Scala"),
        "sc" => Some("Scheme"),
        "SCA" => Some("Visual Fox Pro"),
        "sca" => Some("Visual Fox Pro"),
        "scad" => Some("OpenSCAD"),
        "scala" => Some("Scala"),
        "sch" => Some("Scheme"),
        "scm" => Some("Scheme"),
        "sconscript" => Some("Python"),
        "sconstruct" => Some("Python"),
        "scrbl" => Some("Racket"),
        "scss" => Some("SCSS"),
        "scxml" => Some("XML"),
        "sdl" => Some("TNSDL"),
        "sdt" => Some("TNSDL"),
        "sed" => Some("sed"),
        "ses" => Some("Patran Command Language"),
        "settings.stylecop" => Some("XML"),
        "sfproj" => Some("XML"),
        "sh" => Some("Bourne Shell"),
        "shader" => Some("HLSL"),
        "shproj" => Some("XML"),
        "sig" => Some("Standard ML"),
        "sitemap" => Some("ASP.NET"),
        "sjs" => Some("JavaScript"),
        "sld" => Some("Scheme"),
        "slim" => Some("Slim"),
        "slint" => Some("Slint"),
        "sln" => Some("Visual Studio Solution"),
        "sls" => Some("Scheme/SaltStack"),
        "smarty" => Some("Smarty"),
        "smk" => Some("Snakemake"),
        "sml" => Some("Standard ML"),
        "snakefile" => Some("Python"),
        "snapfile" => Some("Ruby"),
        "sol" => Some("Solidity"),
        "sp" => Some("SparForte"),
        "spc" => Some("Oracle PL/SQL"),
        "spc.sql" => Some("SQL Stored Procedure"),
        "spd" => Some("TNSDL"),
        "spoc.sql" => Some("SQL Stored Procedure"),
        "sproc.sql" => Some("SQL Stored Procedure"),
        "sps" => Some("Scheme"),
        "SQL" => Some("SQL"),
        "sql" => Some("SQL"),
        "sra" => Some("PowerBuilder"),
        "srdf" => Some("XML"),
        "srf" => Some("PowerBuilder"),
        "srm" => Some("PowerBuilder"),
        "srs" => Some("PowerBuilder"),
        "sru" => Some("PowerBuilder"),
        "srw" => Some("PowerBuilder"),
        "ss" => Some("Scheme"),
        "ssc" => Some("TNSDL"),
        "ssjs" => Some("JavaScript"),
        "sss" => Some("SugarSS"),
        "sst" => Some("TNSDL"),
        "st" => Some("Smalltalk"),
        "startup" => Some("AMPLE"),
        "sthlp" => Some("Stata"),
        "storyboard" => Some("XML"),
        "sttheme" => Some("XML"),
        "sty" => Some("TeX"),
        "styl" => Some("Stylus"),
        "sublime-snippet" => Some("XML"),
        "sublime-syntax" => Some("YAML"),
        "surql" => Some("SurrealQL"),
        "sv" => Some("Verilog-SystemVerilog"),
        "svelte" => Some("Svelte"),
        "SVG" => Some("SVG"),
        "svg" => Some("SVG"),
        "svh" => Some("Verilog-SystemVerilog"),
        "swift" => Some("Swift"),
        "syntax" => Some("YAML"),
        "tab" => Some("SQL"),
        "tac" => Some("Python"),
        "targets" => Some("XML"),
        "tcc" => Some("C++"),
        "tcl" => Some("Tcl/Tk"),
        "tcsh" => Some("C Shell"),
        "td" => Some("TableGen"),
        "teal" => Some("TEAL"),
        "templ" => Some("Templ"),
        "tern-config" => Some("JSON"),
        "tern-project" => Some("JSON"),
        "tesc" => Some("GLSL"),
        "tese" => Some("GLSL"),
        "tex" => Some("TeX"),
        "text" => Some("Text"),
        "tf" => Some("HCL"),
        "tfstate" => Some("JSON"),
        "tfstate.backup" => Some("JSON"),
        "tfvars" => Some("HCL"),
        "thor" => Some("Ruby"),
        "thorfile" => Some("Ruby"),
        "thrift" => Some("Thrift"),
        "tk" => Some("Tcl/Tk"),
        "tla" => Some("TLA+"),
        "tmcommand" => Some("XML"),
        "tml" => Some("XML"),
        "tmlanguage" => Some("XML"),
        "tmpreferences" => Some("XML"),
        "tmsnippet" => Some("XML"),
        "tmtheme" => Some("XML"),
        "toml" => Some("TOML"),
        "topojson" => Some("JSON"),
        "tpd" => Some("TITAN Project File Information"),
        "tpl" => Some("Smarty"),
        "tpp" => Some("C++"),
        "tres" => Some("Godot Resource"),
        "trg" => Some("Oracle PL/SQL"),
        "trigger" => Some("Apex Trigger"),
        "ts" => Some("TypeScript/Qt Linguist"),
        "tscn" => Some("Godot Scene"),
        "tspeg" => Some("tspeg"),
        "tss" => Some("Titanium Style Sheet"),
        "tsx" => Some("TypeScript"),
        "ttcn" => Some("TTCN"),
        "ttcn2" => Some("TTCN"),
        "ttcn3" => Some("TTCN"),
        "ttcnpp" => Some("TTCN"),
        "twig" => Some("Twig"),
        "txt" => Some("Text"),
        "typ" => Some("Typst"),
        "udf" => Some("SQL"),
        "udf.sql" => Some("SQL Stored Procedure"),
        "ui" => Some("XML-Qt-GTK/Glade"),
        "um" => Some("Umka"),
        "urdf" => Some("XML"),
        "ux" => Some("XML"),
        "v" => Some("Verilog-SystemVerilog/Coq"),
        "vagrantfile" => Some("Ruby"),
        "vala" => Some("Vala"),
        "vapi" => Some("Vala Header"),
        "VB" => Some("Visual Basic .NET"),
        "vb" => Some("Visual Basic .NET"),
        "VBA" => Some("VB for Applications"),
        "vba" => Some("VB for Applications"),
        "VBHTML" => Some("Visual Basic"),
        "vbhtml" => Some("Visual Basic"),
        "vbp" => Some("Visual Basic"),
        "vbproj" => Some("Visual Basic .NET"),
        "VBS" => Some("Visual Basic Script"),
        "vbs" => Some("Visual Basic Script"),
        "vbw" => Some("Visual Basic"),
        "vcproj" => Some("MSBuild script"),
        "vcxproj" => Some("XML"),
        "vert" => Some("GLSL"),
        "VHD" => Some("VHDL"),
        "vhd" => Some("VHDL"),
        "VHDL" => Some("VHDL"),
        "vhdl" => Some("VHDL"),
        "vhf" => Some("VHDL"),
        "vhi" => Some("VHDL"),
        "vho" => Some("VHDL"),
        "vhs" => Some("VHDL"),
        "vht" => Some("VHDL"),
        "vhw" => Some("VHDL"),
        "vim" => Some("vim script"),
        "viw" => Some("SQL"),
        "vm" => Some("Velocity Template Language"),
        "vrx" => Some("GLSL"),
        "vsh" => Some("GLSL"),
        "vshader" => Some("GLSL"),
        "vsixmanifest" => Some("XML"),
        "vssettings" => Some("XML"),
        "vstemplate" => Some("XML"),
        "vue" => Some("Vuejs Component"),
        "vxml" => Some("XML"),
        "vy" => Some("Vyper"),
        "wast" => Some("WebAssembly"),
        "wat" => Some("WebAssembly"),
        "watchmanconfig" => Some("JSON"),
        "watchr" => Some("Ruby"),
        "wdproj" => Some("MSBuild script"),
        "web.config" => Some("XML"),
        "web.debug.config" => Some("XML"),
        "web.release.config" => Some("XML"),
        "webapp" => Some("JSON"),
        "webinfo" => Some("ASP.NET"),
        "webmanifest" => Some("JSON"),
        "wgsl" => Some("WGSL"),
        "wixproj" => Some("MSBuild script"),
        "wl" => Some("Mathematica"),
        "wlt" => Some("Mathematica"),
        "wlua" => Some("Lua"),
        "workbook" => Some("Markdown"),
        "workspace" => Some("Python"),
        "wscript" => Some("Python"),
        "wsd" => Some("PlantUML"),
        "wsdl" => Some("Web Services Description"),
        "wsf" => Some("XML"),
        "wsgi" => Some("Python"),
        "wxi" => Some("WiX include"),
        "wxl" => Some("WiX string localization"),
        "wxml" => Some("WXML"),
        "wxs" => Some("WiX source"),
        "wxss" => Some("WXSS"),
        "x" => Some("Logos"),
        "x3d" => Some("XML"),
        "xacro" => Some("XML"),
        "xaml" => Some("XAML"),
        "xht" => Some("HTML"),
        "xhtml" => Some("XHTML"),
        "xib" => Some("XML"),
        "xlf" => Some("XML"),
        "xliff" => Some("XML"),
        "xm" => Some("Logos"),
        "XMI" => Some("XMI"),
        "xmi" => Some("XMI"),
        "XML" => Some("XML"),
        "xml" => Some("XML"),
        "xml.builder" => Some("builder"),
        "xml.dist" => Some("XML"),
        "xpo" => Some("X++"),
        "xproj" => Some("XML"),
        "xpy" => Some("Python"),
        "xq" => Some("XQuery"),
        "xql" => Some("XQuery"),
        "xqm" => Some("XQuery"),
        "xquery" => Some("XQuery"),
        "xqy" => Some("XQuery"),
        "xrl" => Some("Erlang"),
        "XSD" => Some("XSD"),
        "xsd" => Some("XSD"),
        "xsjs" => Some("JavaScript"),
        "xsjslib" => Some("JavaScript"),
        "XSL" => Some("XSLT"),
        "xsl" => Some("XSLT"),
        "XSLT" => Some("XSLT"),
        "xslt" => Some("XSLT"),
        "xspec" => Some("XML"),
        "xtend" => Some("Xtend"),
        "xul" => Some("XML"),
        "y" => Some("yacc"),
        "yacc" => Some("yacc"),
        "yaml" => Some("YAML"),
        "yaml-tmlanguage" => Some("YAML"),
        "yang" => Some("Yang"),
        "yap" => Some("Prolog"),
        "yml" => Some("YAML"),
        "yml.mysql" => Some("YAML"),
        "yrl" => Some("Erlang"),
        "yyp" => Some("JSON"),
        "zcml" => Some("XML"),
        "zig" => Some("Zig"),
        "zsh" => Some("zsh"),
        "ʕ◔ϖ◔ʔ" => Some("Go"),
        "🔥" => Some("Mojo"),
        _ => None,
    }
}

pub fn language_for_filename(pattern: &str) -> Option<&'static str> {
    match pattern {
        "BUILD" => Some("Bazel"),
        "build.xml" => Some("Ant/XML"),
        "CMakeLists.txt" => Some("CMake"),
        "cmakelists.txt" => Some("CMake"),
        "Containerfile" => Some("Containerfile"),
        "Dockerfile" => Some("Dockerfile"),
        "dockerfile" => Some("Dockerfile"),
        "Dockerfile.cmake" => Some("Dockerfile"),
        "dockerfile.cmake" => Some("Dockerfile"),
        "Dockerfile.m4" => Some("Dockerfile"),
        "dockerfile.m4" => Some("Dockerfile"),
        "Gnumakefile" => Some("make"),
        "gnumakefile" => Some("make"),
        "Jamfile" => Some("Jam"),
        "jamfile" => Some("Jam"),
        "Jamrules" => Some("Jam"),
        "Makefile" => Some("make"),
        "makefile" => Some("make"),
        "meson.build" => Some("Meson"),
        "pom.xml" => Some("Maven/XML"),
        "Rakefile" => Some("Ruby"),
        "rakefile" => Some("Ruby"),
        "Snakefile" => Some("Snakemake"),
        "WORKSPACE" => Some("Bazel"),
        _ => None,
    }
}

pub fn language_for_prefix(pattern: &str) -> Option<&'static str> {
    match pattern {
        "Containerfile" => Some("Containerfile"),
        "Dockerfile" => Some("Dockerfile"),
        _ => None,
    }
}
