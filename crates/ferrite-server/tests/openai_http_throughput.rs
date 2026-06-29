mod support;

use std::{
    net::SocketAddr,
    time::{Duration, Instant},
};

use support::http::{response_json, send_http_request};

#[tokio::test]
async fn live_http_server_measures_sequential_completion_request_rate(
) -> Result<(), Box<dyn std::error::Error>> {
    let server = support::LiveServer::start().await?;
    let request_body = format!(
        r#"{{"model":"{}","prompt":"hello","max_tokens":1}}"#,
        support::MODEL_ID
    );

    let measurement =
        measure_sequential_completion_requests(server.addr(), request_body.as_bytes(), 5).await?;

    assert_eq!(measurement.completed_requests, 5);
    assert!(measurement.elapsed.as_nanos() > 0);
    assert!(measurement.requests_per_second().is_finite());
    assert!(measurement.requests_per_second() > 0.0);
    Ok(())
}

struct SequentialHttpMeasurement {
    completed_requests: usize,
    elapsed: Duration,
}

impl SequentialHttpMeasurement {
    fn requests_per_second(&self) -> f64 {
        self.completed_requests as f64 / self.elapsed.as_secs_f64()
    }
}

async fn measure_sequential_completion_requests(
    addr: SocketAddr,
    request_body: &[u8],
    request_count: usize,
) -> Result<SequentialHttpMeasurement, Box<dyn std::error::Error>> {
    let started = Instant::now();
    for _ in 0..request_count {
        let response = send_http_request(addr, "POST", "/v1/completions", request_body).await?;
        assert!(
            response.starts_with("HTTP/1.1 200 OK"),
            "unexpected response: {response}"
        );
        let body = response_json(&response)?;
        assert_eq!(body["object"], "text_completion");
        assert_eq!(body["model"], support::MODEL_ID);
        assert_eq!(body["choices"][0]["text"], "winner");
    }

    Ok(SequentialHttpMeasurement {
        completed_requests: request_count,
        elapsed: started.elapsed(),
    })
}
