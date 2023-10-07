#![warn(clippy::pedantic)]

use std::io::{BufRead, BufReader};
use std::path::Path;
use std::{env, fs};

use anyhow::{Context, Result};
use nix::sys::stat::Mode;
use nix::unistd::mkfifo;
use rand::prelude::*;

#[derive(Debug)]
struct ToSignatures<R>(R);

impl<R: BufRead> Iterator for ToSignatures<R> {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        let mut buf = String::new();

        while !buf.ends_with("%%\n") {
            // Iterator terminates on error, or if an EOF
            // is reached without reading a full record.
            if self.0.read_line(&mut buf).ok()? == 0 {
                return None;
            }
        }
        buf.truncate(buf.len() - 3);
        Some(buf)
    }
}

fn main() -> Result<()> {
    let home = env::var("HOME")?;
    let sig_file = format!("{home}/.sigfile");
    let sig_pipe = format!("{home}/.signature");
    let fifo_path = Path::new(&sig_pipe);

    if !fifo_path.exists() {
        println!("Creating FIFO {sig_file}");
        let fifo_mode = Mode::from_bits(0o644).context("invalid file mode {FIFO_MODE}")?;
        mkfifo(fifo_path, fifo_mode)?;
    }

    println!("Starting sigrand...");
    loop {
        let mut file = fs::File::open(&sig_file)?;
        let signature = select_signature(&mut file)
            .context("Failed to select a quote; is your sigfile empty?")?;
        fs::write(fifo_path, signature)?;
    }
}

fn select_signature(file: &mut fs::File) -> Option<String> {
    let sig_iter = ToSignatures(BufReader::new(file));
    let mut rng = thread_rng();

    sig_iter
        .enumerate()
        .fold(None, |selected, (index, res)| match res {
            #[allow(clippy::cast_precision_loss)]
            sig if rng.gen::<f64>() * (index as f64 + 1.0) < 1.0 => Some(sig),
            _ => selected,
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIGFILE: &str = include_str!("../sigfile");

    #[test]
    fn test_empty() {
        const EMPTY_FILE: &str = "";
        let reader = BufReader::new(EMPTY_FILE.as_bytes());
        let sigs = ToSignatures(reader).collect::<Vec<_>>();
        assert!(sigs.is_empty());
    }

    #[test]
    fn test_single_signature() {
        const SINGLE_SIG: &str = "One\nTwo\n%%\n";
        let reader = BufReader::new(SINGLE_SIG.as_bytes());
        let sigs = ToSignatures(reader).collect::<Vec<_>>();
        assert_eq!(sigs[0], "One\nTwo\n");
    }

    #[test]
    fn test_three_signatures() {
        const THREE_SIGS: &str = "One\n%%\nTwo\n%%\nThree\n%%\n";
        let reader = BufReader::new(THREE_SIGS.as_bytes());
        let sigs = ToSignatures(reader).collect::<Vec<_>>();
        assert_eq!(sigs, vec!["One\n", "Two\n", "Three\n"]);
    }

    #[test]
    fn test_partial_single() {
        const PARTIAL_SINGLE: &str = "One\nTwo\n";
        let reader = BufReader::new(PARTIAL_SINGLE.as_bytes());
        let sigs = ToSignatures(reader).collect::<Vec<_>>();
        assert!(sigs.is_empty());
    }

    #[test]
    fn test_partial_final() {
        const PARTIAL_FINAL: &str = "One\n%%\nTwo\n";
        let reader = BufReader::new(PARTIAL_FINAL.as_bytes());
        let sigs = ToSignatures(reader).collect::<Vec<_>>();
        assert_eq!(sigs, vec!["One\n"]);
    }

    #[test]
    fn test_sigfile() {
        let reader = BufReader::new(SIGFILE.as_bytes());
        let sigs = ToSignatures(reader).collect::<Vec<_>>();
        assert_eq!(675, sigs.len());
    }

    #[test]
    fn test_select_signature() {
        let mut file = fs::File::open("./sigfile").unwrap();
        let sig = select_signature(&mut file).unwrap();
        assert!(!sig.is_empty());
        assert!(!sig.contains("%%"));
        assert!(SIGFILE.contains(&sig));
    }
}
