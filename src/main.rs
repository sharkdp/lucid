extern crate ctrlc;
#[macro_use]
extern crate clap;
extern crate nix;

use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::{thread, time};

use clap::{App, AppSettings, Arg};

use nix::unistd;

#[derive(Debug, PartialEq)]
enum LucidError {
    DurationParseError,
    DurationNegative,
    FailedToDaemonize,
}

impl LucidError {
    fn message(&self) -> &str {
        match self {
            LucidError::DurationParseError => "Could not parse 'duration' argument",
            LucidError::DurationNegative => "Duration can not be negative",
            LucidError::FailedToDaemonize => "Failed to daemonize itself",
        }
    }
}

/// Determines how much information should be printed.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
enum VerbosityLevel {
    Quiet,
    Normal,
    Verbose,
}

type ExitCode = i32;

struct OutputHandler<'a> {
    stdout: io::StdoutLock<'a>,
    stderr: io::StderrLock<'a>,
    prefix: &'a str,
    verbosity_level: VerbosityLevel,
    print_to_stderr: bool,
}

impl<'a> OutputHandler<'a> {
    fn new(
        stdout: io::StdoutLock<'a>,
        stderr: io::StderrLock<'a>,
        prefix: &'a str,
        verbosity_level: VerbosityLevel,
        print_to_stderr: bool,
    ) -> Self {
        OutputHandler {
            stdout,
            stderr,
            prefix,
            verbosity_level,
            print_to_stderr,
        }
    }

    fn print(&mut self, msg: &str) {
        match self.verbosity_level {
            VerbosityLevel::Verbose | VerbosityLevel::Normal => self.print_with_prefix(msg),
            _ => {}
        }
    }

    fn print_verbose(&mut self, msg: &str) {
        if self.verbosity_level == VerbosityLevel::Verbose {
            self.print_with_prefix(msg)
        }
    }

    fn print_with_prefix(&mut self, msg: &str) {
        let mut handle: Box<dyn Write> = if self.print_to_stderr {
            Box::new(&mut self.stderr)
        } else {
            Box::new(&mut self.stdout)
        };
        writeln!(handle, "[{}]: {}", self.prefix, msg).ok();
    }
}

type Result<T> = std::result::Result<T, LucidError>;

fn duration_as_str(duration: &time::Duration) -> String {
    format!("{}.{:03}s", duration.as_secs(), duration.subsec_millis())
}

fn duration_from_float(duration_sec: f64) -> Result<time::Duration> {
    if duration_sec < 0.0 {
        return Err(LucidError::DurationNegative);
    }

    let secs = duration_sec.floor() as u64;
    let millisecs = ((duration_sec - secs as f64) * 1e3).round() as u64;

    Ok(time::Duration::from_millis(secs * 1000 + millisecs))
}

fn run() -> Result<ExitCode> {
    let app = App::new(crate_name!())
        .setting(AppSettings::ColorAuto)
        .setting(AppSettings::ColoredHelp)
        .setting(AppSettings::DeriveDisplayOrder)
        .setting(AppSettings::UnifiedHelpMessage)
        .version(crate_version!())
        .arg(Arg::with_name("duration").help(
            "Sleep time in seconds. If no duration is given, \
             the process will sleep forever.",
        ))
        .arg(
            Arg::with_name("ignored")
                .help("Additional arguments are ignored")
                .hidden(true)
                .multiple(true),
        )
        .arg(
            Arg::with_name("exit-code")
                .long("exit-code")
                .short("c")
                .takes_value(true)
                .value_name("CODE")
                .allow_hyphen_values(true)
                .default_value("0")
                .help("Terminate with the given exit code"),
        )
        .arg(
            Arg::with_name("daemon")
                .long("daemon")
                .short("d")
                .help("Daemonize the process after launching"),
        )
        .arg(
            Arg::with_name("no-interrupt")
                .long("no-interrupt")
                .short("I")
                .help("Do not terminate when receiving SIGINT/SIGTERM signals"),
        )
        .arg(
            Arg::with_name("prefix")
                .long("prefix")
                .short("p")
                .takes_value(true)
                .value_name("PREFIX")
                .default_value("lucid")
                .help("Prefix all messages with the given string"),
        )
        .arg(
            Arg::with_name("verbose")
                .long("verbose")
                .short("v")
                .help("Be noisy"),
        )
        .arg(
            Arg::with_name("quiet")
                .long("quiet")
                .short("q")
                .conflicts_with("verbose")
                .help("Do not output anything"),
        )
        .arg(
            Arg::with_name("stderr")
                .long("stderr")
                .short("e")
                .help("Print all messages to stderr"),
        );

    let matches = app.get_matches();

    let sleeping_duration = match matches.value_of("duration") {
        None => None,
        Some(duration) => Some(
            duration
                .parse::<f64>()
                .map_err(|_| LucidError::DurationParseError)
                .and_then(duration_from_float)?,
        ),
    };

    let verbosity_level = if matches.is_present("verbose") {
        VerbosityLevel::Verbose
    } else if matches.is_present("quiet") {
        VerbosityLevel::Quiet
    } else {
        VerbosityLevel::Normal
    };

    let no_interrupt = matches.is_present("no-interrupt");

    let prefix = matches.value_of("prefix").unwrap_or("lucid");

    let exit_code = matches
        .value_of("exit-code")
        .and_then(|c| c.parse::<i32>().ok())
        .unwrap_or(0i32);

    let stdout = io::stdout();
    let stderr = io::stderr();
    let mut output = OutputHandler::new(
        stdout.lock(),
        stderr.lock(),
        prefix,
        verbosity_level,
        matches.is_present("stderr"),
    );

    if matches.is_present("daemon") {
        output.print_verbose("Daemonizing..");
        unistd::daemon(true, true).map_err(|_| LucidError::FailedToDaemonize)?;
    }

    // Print status information
    output.print_verbose(&format!(
        "getcwd() = {}",
        unistd::getcwd()
            .map(|p| p.to_string_lossy().into_owned())
            .map(|s| format!("\"{}\"", s))
            .unwrap_or_else(|_| "<error: could not read current working directory>".into())
    ));
    output.print_verbose(&format!("getpid() = {}", unistd::getpid()));

    match sleeping_duration {
        None => {
            output.print(&("Going to sleep forever").to_string());
        }
        Some(sleeping_duration) => {
            output.print(&format!(
                "Going to sleep for {}",
                duration_as_str(&sleeping_duration)
            ));
        }
    }

    let start_time = time::Instant::now();

    // Set up signal handler
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .expect("Error while setting up signal handler.");

    // Main loop
    let cycle_time = time::Duration::from_millis(100);
    loop {
        let since_start = start_time.elapsed();

        if !running.load(Ordering::SeqCst) {
            if no_interrupt {
                output.print("Ignoring termination signal.");
                running.store(true, Ordering::SeqCst);
            } else {
                output.print("Caught termination signal - interrupting sleep.");
                break;
            }
        }

        if let Some(sleeping_duration) = sleeping_duration {
            if since_start >= sleeping_duration {
                break;
            }

            if since_start + cycle_time > sleeping_duration {
                if sleeping_duration > since_start {
                    thread::sleep(sleeping_duration - since_start);
                } else {
                    break;
                }
            } else {
                thread::sleep(cycle_time);
            }
        } else {
            thread::sleep(cycle_time);
        }

        output.print_verbose(&format!(
            "Still dreaming after {}",
            duration_as_str(&since_start)
        ));
    }

    output.print(&format!(
        "Woke up after {}",
        duration_as_str(&start_time.elapsed())
    ));

    Ok(exit_code)
}

fn main() {
    let result = run();
    match result {
        Err(err) => {
            eprintln!("Error: {}", err.message());
            std::process::exit(1);
        }
        Ok(exit_code) => {
            std::process::exit(exit_code);
        }
    }
}

#[test]
fn test_duration_from_float() {
    assert_eq!(Ok(time::Duration::from_secs(14)), duration_from_float(14.0));
    assert_eq!(
        Ok(time::Duration::from_secs(14)),
        duration_from_float(14.0001)
    );

    assert_eq!(Ok(time::Duration::from_secs(0)), duration_from_float(0.0));

    assert_eq!(
        Ok(time::Duration::from_millis(12345)),
        duration_from_float(12.345)
    );
    assert_eq!(
        Ok(time::Duration::from_millis(12345)),
        duration_from_float(12.3454)
    );
    assert_eq!(
        Ok(time::Duration::from_millis(12346)),
        duration_from_float(12.3456)
    );

    assert_eq!(
        Ok(time::Duration::from_millis(1)),
        duration_from_float(0.001)
    );
    assert_eq!(
        Ok(time::Duration::from_millis(1100)),
        duration_from_float(1.1)
    );

    assert_eq!(Err(LucidError::DurationNegative), duration_from_float(-1.2));
}

#[test]
fn test_verbosity_level() {
    assert!(VerbosityLevel::Normal > VerbosityLevel::Quiet);
    assert!(VerbosityLevel::Verbose > VerbosityLevel::Normal);
    assert!(VerbosityLevel::Verbose > VerbosityLevel::Quiet);
}
