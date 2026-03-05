<h1>
<p align="center">
  <br>branp
</h1>
</p>

## About

`branp` is a a cross platform, general purpose, command line tool developed for hyper-specific
use cases in my own workflow. It is not intended to be used by anyone other than myself, much less
used in production at any large scale. That being said, if you find use for it, let me know how it goes.

## Install

Currently, `branp` is not available on any package manager or through a wget-like command. Only
supports a manual install.

To get started, clone the repository and install the project using `cargo`:

```bash
git clone https://github.com/BrandonPacewic/branp && cargo install --path branp
```

You also have the option of simply building the binary once you have a clone of this repository, if
you would prefer to just do that instead of directly installing it in your `cargo` path. Since `branp`
compiles down into a single binary, you can manually move it wherever you want.

## Usage

Basic usage can be provided by simply running `bp` in your terminal with no arguments. This will
give you a list of all available subcommands. 

## License

Copyright (c) Brandon Pacewic

SPDX-License-Identifier: MIT
