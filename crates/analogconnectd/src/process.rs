use std::{
    ffi::OsStr,
    io::Read,
    path::Path,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use thiserror::Error;

/// Runs a read-only helper with bounded wall time. Captured stdout may contain
/// private backend metadata and must never be logged or persisted by callers.
pub(crate) fn run_bounded<I, S>(
    executable: &Path,
    arguments: I,
    timeout: Duration,
    maximum_output_bytes: usize,
) -> Result<Vec<u8>, BoundedProcessError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    if timeout.is_zero() || maximum_output_bytes == 0 {
        return Err(BoundedProcessError::InvalidTimeout);
    }
    let deadline = Instant::now()
        .checked_add(timeout)
        .ok_or(BoundedProcessError::InvalidTimeout)?;
    let mut child = Command::new(executable)
        .args(arguments)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| BoundedProcessError::Unavailable)?;
    let mut stdout = match child.stdout.take() {
        Some(stdout) => stdout,
        None => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(BoundedProcessError::Unavailable);
        }
    };
    let output_reader = thread::spawn(move || {
        let mut output = Vec::new();
        stdout
            .by_ref()
            .take(maximum_output_bytes.saturating_add(1) as u64)
            .read_to_end(&mut output)
            .map(|_| output)
    });
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(10)),
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = output_reader.join();
                return Err(BoundedProcessError::TimedOut);
            }
            Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = output_reader.join();
                return Err(BoundedProcessError::Unavailable);
            }
        }
    };
    let output = output_reader
        .join()
        .map_err(|_| BoundedProcessError::Unavailable)?
        .map_err(|_| BoundedProcessError::Unavailable)?;
    if output.len() > maximum_output_bytes {
        return Err(BoundedProcessError::OutputTooLarge);
    }
    if status.success() {
        Ok(output)
    } else {
        Err(BoundedProcessError::Failed)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub(crate) enum BoundedProcessError {
    #[error("helper process timeout is invalid")]
    InvalidTimeout,
    #[error("helper process is unavailable")]
    Unavailable,
    #[error("helper process timed out")]
    TimedOut,
    #[error("helper process failed")]
    Failed,
    #[error("helper process output exceeded its private memory bound")]
    OutputTooLarge,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_runner_returns_output_without_diagnostic_disclosure() {
        let marker = "PRIVATE-PROCESS-OUTPUT-MARKER";
        let output =
            run_bounded(Path::new("printf"), [marker], Duration::from_secs(1), 1024).unwrap();
        assert_eq!(output, marker.as_bytes());
        let error = run_bounded(
            Path::new("false"),
            std::iter::empty::<&str>(),
            Duration::from_secs(1),
            1024,
        )
        .unwrap_err();
        assert!(!format!("{error:?}").contains(marker));
        assert_eq!(
            run_bounded(Path::new("printf"), [marker], Duration::from_secs(1), 4),
            Err(BoundedProcessError::OutputTooLarge)
        );
    }

    #[test]
    fn bounded_runner_kills_timeout_and_rejects_zero_duration() {
        let started = Instant::now();
        assert_eq!(
            run_bounded(Path::new("sleep"), ["2"], Duration::from_millis(25), 1024),
            Err(BoundedProcessError::TimedOut)
        );
        assert!(started.elapsed() < Duration::from_secs(1));
        assert_eq!(
            run_bounded(
                Path::new("true"),
                std::iter::empty::<&str>(),
                Duration::ZERO,
                1024
            ),
            Err(BoundedProcessError::InvalidTimeout)
        );
    }
}
