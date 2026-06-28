use super::error::OpenAiHttpError;
use axum::response::{
    sse::{Event, Sse},
    IntoResponse, Response,
};
use serde::Serialize;
use std::convert::Infallible;

pub fn response<T>(items: impl IntoIterator<Item = T>) -> Result<Response, OpenAiHttpError>
where
    T: Serialize,
{
    let mut events = Vec::new();
    for item in items {
        let data = serde_json::to_string(&item).map_err(|error| {
            OpenAiHttpError::internal(format!("failed to serialize stream: {error}"))
        })?;
        events.push(Ok::<Event, Infallible>(Event::default().data(data)));
    }
    events.push(Ok::<Event, Infallible>(Event::default().data("[DONE]")));

    Ok(Sse::new(tokio_stream::iter(events)).into_response())
}
