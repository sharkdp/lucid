lucid
=====

[![Build Status](https://travis-ci.org/sharkdp/lucid.svg?branch=master)](https://travis-ci.org/sharkdp/lucid)

A simple mock-application that can be used by other programs that work with child processes.

`lucid` is similar to `sleep`, but has a few additional features that can be helpful
when debugging applications that spawn subprocesses.

## Demo

<img src="https://cdn.rawgit.com/sharkdp/lucid/f15d7387/lucid.svg" alt="lucid demo">


## Introduction

Applications or scripts that handle child processes need to deal with a lot of different
scenarios.

There are really simple processes that successfully terminate after a short period of time:
``` bash
lucid 2
```

Others also finish after some time, but fail with a **non-zero exit code**:
``` bash
lucid 3 --exit-code=1
```

Some processes just **run forever** (but can be terminated via `SIGINT` or `SIGTERM`):
``` bash
lucid
```

Others refuse to handle **termination signals** properly and just ignore them:
``` bash
lucid 10 --no-interrupt
```

There are also processes that choose to **daemonize** themselves immediately:
``` bash
lucid 10 --daemon
```

Many processes print a lot on **standard output**:
``` bash
lucid 10 --verbose
```

While some others might generate **error messages**:
``` bash
lucid 10 --stderr --verbose
```

## Usage
```
USAGE:
    lucid [OPTIONS] [duration]

OPTIONS:
    -c, --exit-code <CODE>    Terminate with the given exit code [default: 0]
    -d, --daemon              Daemonize the process after launching
    -I, --no-interrupt        Do not terminate when receiving SIGINT/SIGTERM signals
    -p, --prefix <PREFIX>     Prefix all messages with the given string [default: lucid]
    -v, --verbose             Be noisy
    -q, --quiet               Do not output anything
    -e, --stderr              Print all messages to stderr
    -h, --help                Prints help information
    -V, --version             Prints version information

ARGS:
    <duration>    Sleep time in seconds. If no duration is given, the process will sleep forever.
```

## Installation

```
cargo install lucid
```
