# branp

<p align="center">
    <a href="#about">About</a> •
    <a href="#install">Install</a> •
    <a href="#usage">Usage</a> •
    <a href="#Milestones">Milestones</a>
</p>

## About

`branp` is a a cross platform, general purpose, command line tool developed for hyper-specific
use cases in my own workflow. It is not intended to be used by anyone other than myself, much less
used in production at any large scale. That being said, if you find use for it, let me know how it goes.

## Install

Currently, `branp` is not available on any package manager or through a weget-like command. Only
supports a manual install. This is something that will change in the future, part of this tool is
fast configuration on new machines and having many streamlined methods of quickly installing it
is a necessity. 

To get started, clone the repository and install the project using `cargo`:

```bash
git clone https://github.com/BrandonPacewic/branp branp-cli && cargo install --path branp-cli
```

You also have the option of simply building the binary once you have a clone of this repository, if
you would prefer to just do that instead of directly installing it in your `cargo` path. Since `branp`
compiles down into a single binary, you can manually move it wherever you want.

## Usage

Basic usage can be provided by simply running `branp` in your terminal with no arguments. This will
give you a list of all available subcommands. A more comprehensive list of all available subcommands
and their use cases is a work in progress.

## Milestones

`branp` is very much a work in progress. The following is a list of goals I have for the project, in
no particular order:

|  #  | Goal                                                      | Status |
| :-: | --------------------------------------------------------- | :----: |
|  1  | Project re-write in *pure* Rust                           |   ⚠️   |
|  2  | Performance with supporting benchmarks                    |   ❌   |
|  3  | Zero external dependencies                                |   ⚠️   |
|  4  | Nightly like mode for *in-progress* features              |   ❌   |
|  5  | Ability to run in heavily restricted environments         |   ❌   |
|  6  | Cross platform support and configuration                  |   ❌   |
|  N  | More fancy features (to be expanded upon later)           |   ❌   |

These goals do not reflect specific features of `branp`, more of a compliance standard for each feature.
A more detailed description of each goal can be found below:

#### Project Re-write in *Pure* Rust

Bit of history, this project started as a collection of python scripts that I used for competitive programming.
That then evolved into a python cli tool that contained all of the original scripts and slowly morphed into
a do-it-all tool that I used for everything. The original code was a mess and not up to the standards
that I now hold for myself. Thus the project was re-written in Rust, and only rust.

A self imposed restriction of this project is that everything must be done in rust. No other languages, even
shell or bash scripts. *Pure* rust.

#### Performance with Supporting Benchmarks

Performance is a big deal, this is reflected with some of the features of `branp` having functionality
that mirrors of already existing tools. The goal is to be able to run `branp` in a way that is faster
than the equivalent tool. This is a lofty, but within reach, goal.

Currently I do not have a requirement for what commands constitute having a benchmark. This will likely
be more defined at a later date. As a general rule, if the subcommand is based on another tool, it should
be benchmarked.

> [!NOTE]
> `branp` is already _very fast_ but there is always room for improvement.

#### Zero External Dependencies

The goal of `branp` is to be a totally self contained tool. This means that it should not rely on or require
any library outside of the standard library. The status of this goal is likely to fluxuate as new features
get added. The end goal is to have no external dependencies, but if I need to add something temporarily to
get a feature working, I will. You can check the status of this goal via the
<a href="#milestones">Milestones</a> table or by checking `Cargo.toml` for any dependencies.

> [!NOTE]
> Another reason for this goal is that `branp` should be able to run in a heavily restricted environment.
> This means that any dependencies that needs to be installed or configured needs to be avoided.

#### Nightly like Mode for *In-Progress* Features

Much like `rustup` or `cargo`, I want to be able to add a semi-working feature to `branp` with an explicit
way of identifying it as such.

The category of these features will consist of subcommands that have performance issues or are simply
not fully implemented and are missing quality-of-life output, help information, etc. Basically, if
in order to use the command you need to have precise knowledge of its exact implementation, I'll mark
it as nightly.


#### Ability to Run in Heavily Restricted Environments

I originally started developing `branp` while in college, where a lot of the work I would do was on servers
with heavy environment restrictions on what can be installed and what configuration was possible. This is
what I would refer to as *heavily restricted*.

Another use case for `branp` is on work computers that simply may not have access to administrator.

For each subcommand that requires file access, network access, or any other permission that could be denied
or restricted, there should be safeguards in place for detecting this restrictions and handing them
accordingly (if possible). This of course means that some subcommands will simple not work, depending
on the restriction, and this has to be expected and accepted.


#### Cross Platform Support and Configuration

`branp` is intended to be a cross platform tool, across Windows, MacOS, and Linux. This goal is derived from
the fact that `branp` provides functionality to machines that have lacking configuration (i.e. not my personal
computer). This comes with no guarantee that I can assume the operating system before I need a specific
use case of this tool. As such, `branp` needs to be cross platform.

## License

Copyright (c) Brandon Pacewic

SPDX-License-Identifier: MIT
