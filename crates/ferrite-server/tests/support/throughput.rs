use super::{
    http::{response_json, send_http_request},
    MODEL_ID,
};
use std::{
    net::SocketAddr,
    time::{Duration, Instant},
};

#[derive(Debug)]
pub struct CompletionThroughputMeasurement {
    pub completed_requests: usize,
    pub elapsed: Duration,
}

impl CompletionThroughputMeasurement {
    pub fn requests_per_second(&self) -> f64 {
        self.completed_requests as f64 / self.elapsed.as_secs_f64()
    }
}

pub async fn measure_sequential_completion_requests(
    addr: SocketAddr,
    request_body: &[u8],
    request_count: usize,
) -> Result<CompletionThroughputMeasurement, Box<dyn std::error::Error>> {
    let started = Instant::now();
    for _ in 0..request_count {
        let response = send_http_request(addr, "POST", "/v1/completions", request_body).await?;
        validate_completion_response(&response)?;
    }

    Ok(CompletionThroughputMeasurement {
        completed_requests: request_count,
        elapsed: started.elapsed(),
    })
}

pub async fn measure_concurrent_completion_requests(
    addr: SocketAddr,
    request_body: &[u8],
    request_count: usize,
) -> Result<CompletionThroughputMeasurement, Box<dyn std::error::Error>> {
    let started = Instant::now();
    let mut tasks = Vec::with_capacity(request_count);

    for _ in 0..request_count {
        let request_body = request_body.to_vec();
        tasks.push(tokio::spawn(async move {
            send_http_request(addr, "POST", "/v1/completions", &request_body)
                .await
                .map_err(|error| error.to_string())
        }));
    }

    for task in tasks {
        let response = task
            .await
            .map_err(|error| std::io::Error::other(format!("request task failed: {error}")))?
            .map_err(std::io::Error::other)?;
        validate_completion_response(&response)?;
    }

    Ok(CompletionThroughputMeasurement {
        completed_requests: request_count,
        elapsed: started.elapsed(),
    })
}

fn validate_completion_response(response: &str) -> Result<(), Box<dyn std::error::Error>> {
    assert!(
        response.starts_with("HTTP/1.1 200 OK"),
        "unexpected response: {response}"
    );
    let body = response_json(response)?;
    assert_eq!(body["object"], "text_completion");
    assert_eq!(body["model"], MODEL_ID);
    assert_eq!(body["choices"][0]["text"], "winner");
    Ok(())
}
