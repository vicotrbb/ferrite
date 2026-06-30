use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StreamingTimingSummary {
    token_events: usize,
    time_to_first_token: Duration,
    total_elapsed: Duration,
    min_token_latency: Duration,
    p50_token_latency: Duration,
    p95_token_latency: Duration,
    max_token_latency: Duration,
}

impl StreamingTimingSummary {
    pub fn from_event_offsets(offsets: &[Duration]) -> Option<Self> {
        let (&time_to_first_token, &total_elapsed) = offsets.first().zip(offsets.last())?;
        let mut token_latencies = token_latencies(offsets);
        token_latencies.sort_unstable();

        Some(Self {
            token_events: offsets.len(),
            time_to_first_token,
            total_elapsed,
            min_token_latency: token_latencies[0],
            p50_token_latency: percentile_nearest_rank(&token_latencies, 0.50),
            p95_token_latency: percentile_nearest_rank(&token_latencies, 0.95),
            max_token_latency: token_latencies[token_latencies.len() - 1],
        })
    }

    pub fn token_events(&self) -> usize {
        self.token_events
    }

    pub fn time_to_first_token(&self) -> Duration {
        self.time_to_first_token
    }

    pub fn total_elapsed(&self) -> Duration {
        self.total_elapsed
    }

    pub fn min_token_latency(&self) -> Duration {
        self.min_token_latency
    }

    pub fn p50_token_latency(&self) -> Duration {
        self.p50_token_latency
    }

    pub fn p95_token_latency(&self) -> Duration {
        self.p95_token_latency
    }

    pub fn max_token_latency(&self) -> Duration {
        self.max_token_latency
    }

    pub fn tokens_per_second(&self) -> f64 {
        self.token_events as f64 / self.total_elapsed.as_secs_f64()
    }
}

fn token_latencies(offsets: &[Duration]) -> Vec<Duration> {
    let mut previous = Duration::ZERO;
    offsets
        .iter()
        .map(|&offset| {
            let latency = offset.saturating_sub(previous);
            previous = offset;
            latency
        })
        .collect()
}

fn percentile_nearest_rank(sorted_values: &[Duration], percentile: f64) -> Duration {
    let rank = (percentile * sorted_values.len() as f64).ceil() as usize;
    sorted_values[rank.saturating_sub(1).min(sorted_values.len() - 1)]
}
