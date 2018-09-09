extern crate ctrlc;
#[macro_use]
extern crate clap;

use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::{thread, time};

use clap::{App, AppSettings, Arg};

#[derive(Debug, PartialEq)]
enum LucidError {
    DurationParseError,
    DurationNegative,
}

/// Determines how much information should be printed.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
enum VerbosityLevel {
    Quiet,
    Normal,
    Verbose,
}

impl LucidError {
    fn message(&self) -> &str {
        match self {
            LucidError::DurationParseError => "Could not parse 'duration' argument",
            LucidError::DurationNegative => "Duration can not be negative",
        }
    }
}

struct OutputHandler<'a> {
    handle: io::StdoutLock<'a>,
    prefix: &'a str,
    verbosity_level: VerbosityLevel,
}

impl<'a> OutputHandler<'a> {
    fn new(handle: io::StdoutLock<'a>, prefix: &'a str, verbosity_level: VerbosityLevel) -> Self {
        OutputHandler {
            handle,
            prefix,
            verbosity_level,
        }
    }

    fn print(&mut self, msg: &str) {
        match self.verbosity_level {
            VerbosityLevel::Verbose | VerbosityLevel::Normal => self.print_with_prefix(msg),
            _ => {}
        }
    }

    fn print_verbose(&mut self, msg: &str) {
        match self.verbosity_level {
            VerbosityLevel::Verbose => self.print_with_prefix(msg),
            _ => {}
        }
    }

    fn print_with_prefix(&mut self, msg: &str) {
        writeln!(self.handle, "[{}]: {}", self.prefix, msg).ok();
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

fn run() -> Result<()> {
    let app = App::new(crate_name!())
        .setting(AppSettings::ColorAuto)
        .setting(AppSettings::ColoredHelp)
        .setting(AppSettings::DeriveDisplayOrder)
        .version(crate_version!())
        .arg(
            Arg::with_name("duration")
                .help("sleep time in seconds")
                .required(true),
        ).arg(
            Arg::with_name("verbose")
                .long("verbose")
                .short("v")
                .help("Be verbose"),
        ).arg(
            Arg::with_name("quiet")
                .long("quiet")
                .short("q")
                .conflicts_with("verbose")
                .help("Do not output anything"),
        ).arg(
            Arg::with_name("prefix")
                .long("prefix")
                .short("p")
                .takes_value(true)
                .default_value("lucid")
                .help("Prefix all messages with the given string"),
        ).arg(
            Arg::with_name("no-interrupt")
                .long("no-interrupt")
                .short("I")
                .help("Do not terminate when receiving SIGINT/SIGTERM signals"),
        );

    let matches = app.get_matches();

    let sleeping_duration = matches
        .value_of("duration")
        .expect("duration is a required argument")
        .parse::<f64>()
        .map_err(|_| LucidError::DurationParseError)
        .and_then(duration_from_float)?;

    let verbosity_level = if matches.is_present("verbose") {
        VerbosityLevel::Verbose
    } else if matches.is_present("quiet") {
        VerbosityLevel::Quiet
    } else {
        VerbosityLevel::Normal
    };

    let no_interrupt = matches.is_present("no-interrupt");

    let prefix = matches.value_of("prefix").unwrap_or("lucid");

    let stdout = io::stdout();
    let mut output = OutputHandler::new(stdout.lock(), prefix, verbosity_level);

    output.print(&format!(
        "Going to sleep for {}",
        duration_as_str(&sleeping_duration)
    ));

    let start_time = time::Instant::now();

    // Set up signal handler
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    }).expect("Error while setting up signal handler.");

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

        output.print_verbose(&format!(
            "Still dreaming after {}",
            duration_as_str(&since_start)
        ));
    }

    output.print(&format!(
        "Woke up after {}",
        duration_as_str(&start_time.elapsed())
    ));

    Ok(())
}

fn main() {
    let result = run();
    if let Err(err) = result {
        eprintln!("Error: {}", err.message());
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
