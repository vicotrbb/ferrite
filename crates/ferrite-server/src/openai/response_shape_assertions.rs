use serde_json::Value;

pub(super) fn json_sse_events(body: &str) -> Result<Vec<Value>, Box<dyn std::error::Error>> {
    body.lines()
        .filter_map(|line| line.strip_prefix("data: "))
        .filter(|data| *data != "[DONE]")
        .map(|data| Ok(serde_json::from_str(data)?))
        .collect()
}

pub(super) fn assert_choice_has_null_logprobs(
    event: &Value,
) -> Result<(), Box<dyn std::error::Error>> {
    let choice = event["choices"][0]
        .as_object()
        .ok_or("expected streamed choice object")?;
    assert!(choice.contains_key("logprobs"), "{event}");
    assert!(choice["logprobs"].is_null(), "{event}");
    Ok(())
}

pub(super) fn assert_null_system_fingerprint(
    body: &Value,
) -> Result<(), Box<dyn std::error::Error>> {
    let object = body.as_object().ok_or("expected response object")?;
    assert!(object.contains_key("system_fingerprint"), "{body}");
    assert!(body["system_fingerprint"].is_null(), "{body}");
    Ok(())
}

pub(super) fn assert_usage_has_detail_counters(
    usage: &Value,
) -> Result<(), Box<dyn std::error::Error>> {
    let prompt_details = usage["prompt_tokens_details"]
        .as_object()
        .ok_or("expected prompt token details")?;
    assert_eq!(prompt_details["cached_tokens"], 0, "{usage}");
    assert_eq!(prompt_details["audio_tokens"], 0, "{usage}");

    let completion_details = usage["completion_tokens_details"]
        .as_object()
        .ok_or("expected completion token details")?;
    assert_eq!(completion_details["reasoning_tokens"], 0, "{usage}");
    assert_eq!(completion_details["audio_tokens"], 0, "{usage}");
    assert_eq!(
        completion_details["accepted_prediction_tokens"], 0,
        "{usage}"
    );
    assert_eq!(
        completion_details["rejected_prediction_tokens"], 0,
        "{usage}"
    );
    Ok(())
}
