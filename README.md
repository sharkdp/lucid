lucid
=====

A simple mock-application that can be used by other programs that spawn subprocesses.

`lucid` is similar to `sleep`, but has a few additional features that can be helpful
when debugging applications that spawn subprocesses.

## Introduction

Applications or scripts that handle child processes need to handle a lot of different
scenarios.

There are very simple processes that successfully terminate after a short period of time:
```
lucid 2
```

Others also finish after some time but fail with a non-zero exit code:
```
lucid 3 --exit-code=1
```

Some processes run forever (but can be terminated via `SIGINT` or `SIGTERM`)
```
lucid
```

Others refuse to handle termination signals properly and just ignore them:
```
lucid 10 --no-interrupt
```

Many processes output a lot on standard output:
```
lucid 10 --verbose
```

Others generate a lot of error messages:
```
lucid 10 --stderr --verbose
```

## Usage
```
USAGE:
    lucid [OPTIONS] [duration]

OPTIONS:
    -v, --verbose             Be noisy.
    -q, --quiet               Do not output anything.
    -p, --prefix <PREFIX>     Prefix all messages with the given string. [default: lucid]
    -c, --exit-code <CODE>    Terminate with the given exit code. [default: 0]
    -e, --stderr              Print all messages to stderr.
    -I, --no-interrupt        Do not terminate when receiving SIGINT/SIGTERM signals.
    -h, --help                Print help information.
    -V, --version             Print version information.

ARGS:
    <duration>    Sleep time in seconds. If no duration is given, the process will sleep forever.
```
