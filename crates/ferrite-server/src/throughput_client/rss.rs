use std::{error::Error, future::Future, process::Command, time::Duration};

use tokio::time::sleep;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RssSummary {
    before_bytes: u64,
    after_bytes: u64,
    idle_bytes: u64,
}

impl RssSummary {
    pub fn new(before_bytes: u64, after_bytes: u64, idle_bytes: u64) -> Self {
        Self {
            before_bytes,
            after_bytes,
            idle_bytes,
        }
    }

    pub async fn sample_around<F, T>(
        pid: u32,
        idle_delay: Duration,
        operation: F,
    ) -> Result<(T, Self), Box<dyn Error>>
    where
        F: Future<Output = Result<T, Box<dyn Error>>>,
    {
        let before_bytes = sample_process_rss_bytes(pid)?;
        let result = operation.await?;
        let after_bytes = sample_process_rss_bytes(pid)?;
        sleep(idle_delay).await;
        let idle_bytes = sample_process_rss_bytes(pid)?;
        Ok((result, Self::new(before_bytes, after_bytes, idle_bytes)))
    }

    pub fn parse_ps_rss_bytes(output: &str) -> Result<u64, Box<dyn Error>> {
        let kib: u64 = output
            .split_whitespace()
            .next()
            .ok_or("missing ps rss output")?
            .parse()?;
        kib.checked_mul(1024)
            .ok_or_else(|| "ps rss byte conversion overflowed".into())
    }

    pub fn before_bytes(&self) -> u64 {
        self.before_bytes
    }

    pub fn after_bytes(&self) -> u64 {
        self.after_bytes
    }

    pub fn idle_bytes(&self) -> u64 {
        self.idle_bytes
    }
}

fn sample_process_rss_bytes(pid: u32) -> Result<u64, Box<dyn Error>> {
    let output = Command::new("ps")
        .args(["-o", "rss=", "-p", &pid.to_string()])
        .output()?;
    if !output.status.success() {
        return Err(format!("ps rss sample failed for pid {pid}: {}", output.status).into());
    }
    let stdout = String::from_utf8(output.stdout)?;
    RssSummary::parse_ps_rss_bytes(&stdout)
}
