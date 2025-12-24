//! k6 runner implementation
//!
//! This module provides functionality to run k6 scripts and parse their output.

use super::{RunMetrics, RunResult, Runner};
use crate::error::{HornetError, Result};
use regex::Regex;
use std::io::Write;
use std::path::Path;
use std::process::Command;
use tempfile::NamedTempFile;

/// Runner for k6 test scripts
#[derive(Debug, Clone, Default)]
pub struct K6Runner {
    /// Path to k6 binary (defaults to "k6" in PATH)
    k6_path: String,
    /// Additional k6 arguments
    extra_args: Vec<String>,
}

impl K6Runner {
    /// Create a new K6Runner with default settings
    pub fn new() -> Self {
        Self {
            k6_path: "k6".to_string(),
            extra_args: Vec::new(),
        }
    }

    /// Create a K6Runner with a custom k6 binary path
    pub fn with_path<S: Into<String>>(path: S) -> Self {
        Self {
            k6_path: path.into(),
            extra_args: Vec::new(),
        }
    }

    /// Add extra arguments to pass to k6
    pub fn with_args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.extra_args = args.into_iter().map(|s| s.into()).collect();
        self
    }

    /// Parse k6 output to extract metrics
    fn parse_output(&self, stdout: &str) -> Option<RunMetrics> {
        let mut metrics = RunMetrics::default();

        // Parse http_reqs
        let http_reqs_re = Regex::new(r"http_reqs[.\s]+:\s+(\d+)").ok()?;
        if let Some(caps) = http_reqs_re.captures(stdout) {
            metrics.http_reqs = caps[1].parse().unwrap_or(0);
        }

        // Parse iterations
        let iterations_re = Regex::new(r"iterations[.\s]+:\s+(\d+)").ok()?;
        if let Some(caps) = iterations_re.captures(stdout) {
            metrics.iterations = caps[1].parse().unwrap_or(0);
        }

        // Parse http_req_duration avg
        let duration_re = Regex::new(r"http_req_duration[.\s]+:.*avg=(\d+\.?\d*)(\w+)").ok()?;
        if let Some(caps) = duration_re.captures(stdout) {
            let value: f64 = caps[1].parse().unwrap_or(0.0);
            let unit = &caps[2];
            metrics.avg_response_time_ms = match unit {
                "s" => value * 1000.0,
                "ms" => value,
                "µs" | "us" => value / 1000.0,
                _ => value,
            };
        }

        // Parse checks
        let checks_re = Regex::new(r"checks[.\s]+:\s+(\d+\.?\d*)%\s+✓\s+(\d+)\s+✗\s+(\d+)").ok()?;
        if let Some(caps) = checks_re.captures(stdout) {
            metrics.checks_passed = caps[2].parse().unwrap_or(0);
            metrics.checks_failed = caps[3].parse().unwrap_or(0);
        }

        // Parse vus
        let vus_re = Regex::new(r"vus[.\s]+:\s+(\d+)").ok()?;
        if let Some(caps) = vus_re.captures(stdout) {
            metrics.vus = caps[1].parse().unwrap_or(0);
        }

        Some(metrics)
    }
}

impl Runner for K6Runner {
    fn is_available(&self) -> bool {
        Command::new(&self.k6_path)
            .arg("version")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    fn version(&self) -> Result<String> {
        let output = Command::new(&self.k6_path)
            .arg("version")
            .output()
            .map_err(HornetError::IoError)?;

        if output.status.success() {
            let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
            Ok(version)
        } else {
            Err(HornetError::ValidationError(
                "Failed to get k6 version".to_string(),
            ))
        }
    }

    fn run_script<P: AsRef<Path>>(&self, script_path: P) -> Result<RunResult> {
        let script_path = script_path.as_ref();

        if !script_path.exists() {
            return Err(HornetError::IoError(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Script not found: {}", script_path.display()),
            )));
        }

        let mut cmd = Command::new(&self.k6_path);
        cmd.arg("run");

        // Add extra arguments
        for arg in &self.extra_args {
            cmd.arg(arg);
        }

        cmd.arg(script_path);

        let output = cmd.output().map_err(HornetError::IoError)?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let exit_code = output.status.code().unwrap_or(-1);
        let success = output.status.success();

        let metrics = self.parse_output(&stdout);

        Ok(RunResult {
            success,
            exit_code,
            stdout,
            stderr,
            metrics,
        })
    }

    fn run_script_content(&self, content: &str) -> Result<RunResult> {
        // Write content to a temporary file
        let mut temp_file = NamedTempFile::with_suffix(".js").map_err(HornetError::IoError)?;

        temp_file
            .write_all(content.as_bytes())
            .map_err(HornetError::IoError)?;

        temp_file.flush().map_err(HornetError::IoError)?;

        // Run the temporary script
        self.run_script(temp_file.path())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_k6_runner_creation() {
        let runner = K6Runner::new();
        assert_eq!(runner.k6_path, "k6");

        let runner = K6Runner::with_path("/usr/local/bin/k6");
        assert_eq!(runner.k6_path, "/usr/local/bin/k6");
    }

    #[test]
    fn test_k6_runner_with_args() {
        let runner = K6Runner::new().with_args(vec!["--no-color", "--quiet"]);
        assert_eq!(runner.extra_args, vec!["--no-color", "--quiet"]);
    }

    #[test]
    fn test_parse_output() {
        let runner = K6Runner::new();
        let sample_output = r#"
          scenarios: (100.00%) 1 scenario, 1 max VUs, 10m30s max duration (incl. graceful stop):
                   * default: 1 iterations for each of 1 VUs (maxDuration: 10m0s, gracefulStop: 30s)

     data_received..................: 0 B   0 B/s
     data_sent......................: 0 B   0 B/s
     http_req_blocked...............: avg=0s       min=0s      med=0s      max=0s      p(90)=0s      p(95)=0s
     http_req_connecting............: avg=0s       min=0s      med=0s      max=0s      p(90)=0s      p(95)=0s
     http_req_duration..............: avg=123.45ms min=100ms   med=120ms   max=150ms   p(90)=140ms   p(95)=145ms
     http_req_failed................: 0.00% ✓ 0       ✗ 4
     http_req_receiving.............: avg=0s       min=0s      med=0s      max=0s      p(90)=0s      p(95)=0s
     http_req_sending...............: avg=0s       min=0s      med=0s      max=0s      p(90)=0s      p(95)=0s
     http_req_tls_handshaking.......: avg=0s       min=0s      med=0s      max=0s      p(90)=0s      p(95)=0s
     http_req_waiting...............: avg=0s       min=0s      med=0s      max=0s      p(90)=0s      p(95)=0s
     http_reqs......................: 4     0.123456/s
     iteration_duration.............: avg=1s       min=1s      med=1s      max=1s      p(90)=1s      p(95)=1s
     iterations.....................: 1     0.030864/s
     vus............................: 1     min=1       max=1
     vus_max........................: 1     min=1       max=1
     checks.........................: 100.00% ✓ 8       ✗ 0
        "#;

        let metrics = runner.parse_output(sample_output);
        assert!(metrics.is_some());

        let metrics = metrics.unwrap();
        assert_eq!(metrics.http_reqs, 4);
        assert_eq!(metrics.iterations, 1);
        assert!((metrics.avg_response_time_ms - 123.45).abs() < 0.01);
        assert_eq!(metrics.checks_passed, 8);
        assert_eq!(metrics.checks_failed, 0);
        assert_eq!(metrics.vus, 1);
    }

    #[test]
    #[ignore] // Requires k6 to be installed
    fn test_k6_is_available() {
        let runner = K6Runner::new();
        // This test will pass if k6 is installed
        let _ = runner.is_available();
    }

    #[test]
    #[ignore] // Requires k6 to be installed
    fn test_k6_version() {
        let runner = K6Runner::new();
        let version = runner.version();
        if runner.is_available() {
            assert!(version.is_ok());
            assert!(version.unwrap().contains("k6"));
        }
    }
}
