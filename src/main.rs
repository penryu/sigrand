#![warn(clippy::pedantic)]

//! Crate docs go here

use std::env;
use std::error;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::sync::atomic;
use std::sync::Arc;

use getopts::Options;
use log::{debug, error, info, warn};
use nix::sys::signal::kill;
use nix::sys::stat::Mode;
use nix::unistd::{fork, mkfifo, ForkResult, Pid};
use rand::prelude::*;
use serde::{Deserialize, Serialize};

type SigrandResult<T> = Result<T, Box<dyn error::Error>>;

#[derive(Debug, Deserialize, Serialize)]
struct SigrandConfig {
    fifo: String,
    pid: Option<i32>,
    signature_file: String,
}

impl Default for SigrandConfig {
    fn default() -> Self {
        SigrandConfig {
            fifo: "~/.signature".into(),
            pid: None,
            signature_file: "~/.sigfile".into(),
        }
    }
}

const APP_NAME: &str = "sigrand";

fn main() -> SigrandResult<()> {
    pretty_env_logger::init();

    let args: Vec<String> = env::args().collect();
    let my_name = args[0].clone();

    let mut opts = Options::new();
    opts.optflag("d", "", "daemon mode; forks after launch");
    opts.optflag("h", "help", "display this help");
    opts.optflag("k", "kill", "kills a running instance");

    let matches = opts.parse(&args[1..])?;
    if matches.opt_present("h") {
        let header = format!("Usage: {} [options]", my_name);
        print!("{}", opts.usage(&header));
        return Ok(());
    }

    let mut config: SigrandConfig = confy::load(APP_NAME)?;

    let sig_pipe = shellexpand::tilde(&config.fifo);
    let sig_file = shellexpand::tilde(&config.signature_file);

    if let Some(pid) = config.pid {
        if pid <= 1 {
            warn!("Found invalid pid {}; ignoring", pid);
        } else if kill(Pid::from_raw(pid), None).is_ok() {
            error!("Already running at pid {}", pid);
            return Err(
                format!("{} already running at pid {}", APP_NAME, pid).into()
            );
        } else {
            warn!("Ignoring stale pid {}", pid);
        }
    }

    config.pid = None;
    confy::store(APP_NAME, &config)?;

    let fifo_path = Path::new(&*sig_pipe);
    if !fifo_path.exists() {
        mkfifo(fifo_path, Mode::from_bits(0o644).unwrap())?;
    }

    info!("Starting sigrand...");

    if let ForkResult::Parent { child } = unsafe { fork() }? {
        info!("Forked child process {}; exiting...", child);
        config.pid = Some(child.into());
        confy::store(APP_NAME, config)?;
        return Ok(());
    }

    debug!("Starting loop...");

    let quit = Arc::new(atomic::AtomicBool::new(false));
    let _ = signal_hook::flag::register(libc::SIGINT, Arc::clone(&quit))?;
    let _ = signal_hook::flag::register(libc::SIGTERM, Arc::clone(&quit))?;
    while !quit.load(atomic::Ordering::Relaxed) {
        let signature = select_signature(&sig_file)?;
        fs::write(fifo_path, signature)?;
    }

    debug!("Shutting down...");

    // reset pid in cfg
    config.pid = None;
    confy::store(APP_NAME, &config)?;

    Ok(())
}

fn select_signature(filename: &str) -> SigrandResult<String> {
    let mut rng = thread_rng();
    let sigfile = fs::File::open(filename)?;
    let sigbuf = BufReader::new(sigfile);

    let mut quote = String::new();
    let mut candidate = String::new();
    let mut quote_count = 0;
    for line in sigbuf.lines() {
        match line?.as_str() {
            "%%" => {
                quote_count += 1;
                if rng.gen::<f64>() * f64::from(quote_count) < 1.0 {
                    quote = candidate;
                }
                candidate = String::new();
            }
            line => {
                candidate.push('\n');
                candidate.push_str(line);
            }
        }
    }

    Ok(quote)
}
