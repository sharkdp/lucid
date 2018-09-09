extern crate ctrlc;
#[macro_use]
extern crate clap;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::{thread, time};

use clap::{App, AppSettings, Arg};

#[derive(Debug, PartialEq)]
enum LucidError {
    DurationParseError,
    DurationNegative,
}

impl LucidError {
    fn message(&self) -> &str {
        match self {
            LucidError::DurationParseError => "Could not parse 'duration' argument",
            LucidError::DurationNegative => "Duration can not be negative",
        }
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
        );

    let matches = app.get_matches();

    let sleeping_duration = matches
        .value_of("duration")
        .expect("duration is a required argument")
        .parse::<f64>()
        .map_err(|_| LucidError::DurationParseError)
        .and_then(duration_from_float)?;

    println!("Going to sleep for {}", duration_as_str(&sleeping_duration));

    let start_time = time::Instant::now();

    // Set up signal handler
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    }).expect("Error while setting up signal handler.");

    // Main loop
    let cycle_time = time::Duration::from_millis(100); // TODO
    loop {
        let since_start = start_time.elapsed();

        if !running.load(Ordering::SeqCst) {
            println!("Caught termination signal - interrupting sleep.");
            break;
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

        println!("Still dreaming after {}", duration_as_str(&since_start));
    }

    println!("Waking up after {}", duration_as_str(&start_time.elapsed()));

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
