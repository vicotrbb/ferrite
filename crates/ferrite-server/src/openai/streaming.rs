use super::error::OpenAiHttpError;
use axum::response::{
    IntoResponse, Response,
    sse::{Event, Sse},
};
use serde::Serialize;
use std::convert::Infallible;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

const STREAM_CHANNEL_CAPACITY: usize = 16;
type StreamEvent = Result<Event, Infallible>;

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

#[derive(Clone, Debug)]
pub struct StreamSender {
    sender: mpsc::Sender<StreamEvent>,
}

impl StreamSender {
    pub fn send_json_blocking<T>(&self, item: &T) -> Result<(), OpenAiHttpError>
    where
        T: Serialize,
    {
        let data = serde_json::to_string(item).map_err(|error| {
            OpenAiHttpError::internal(format!("failed to serialize stream: {error}"))
        })?;
        self.send_event_blocking(Event::default().data(data))
    }

    pub fn send_done_blocking(&self) -> Result<(), OpenAiHttpError> {
        self.send_event_blocking(Event::default().data("[DONE]"))
    }

    pub fn is_closed(&self) -> bool {
        self.sender.is_closed()
    }

    fn send_event_blocking(&self, event: Event) -> Result<(), OpenAiHttpError> {
        self.sender
            .blocking_send(Ok(event))
            .map_err(|_error| OpenAiHttpError::internal("stream receiver closed"))
    }
}

pub fn channel_response() -> (StreamSender, Response) {
    let (sender, receiver) = mpsc::channel(STREAM_CHANNEL_CAPACITY);
    (
        StreamSender { sender },
        Sse::new(ReceiverStream::new(receiver)).into_response(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::{Body, to_bytes};

    #[tokio::test]
    async fn channel_response_streams_serialized_events_and_done()
    -> Result<(), Box<dyn std::error::Error>> {
        let (sender, response) = channel_response();
        tokio::task::spawn_blocking(move || -> Result<(), OpenAiHttpError> {
            sender.send_json_blocking(&serde_json::json!({"text":"winner"}))?;
            sender.send_done_blocking()?;
            Ok(())
        })
        .await??;

        let body = to_text(response.into_body()).await?;

        assert!(body.contains("data: {\"text\":\"winner\"}"));
        assert!(body.contains("data: [DONE]"));
        Ok(())
    }

    #[test]
    fn stream_sender_reports_when_receiver_is_closed() {
        let (sender, response) = channel_response();

        assert!(!sender.is_closed());
        drop(response);
        assert!(sender.is_closed());
    }

    async fn to_text(body: Body) -> Result<String, Box<dyn std::error::Error>> {
        let bytes = to_bytes(body, usize::MAX).await?;
        Ok(String::from_utf8(bytes.to_vec())?)
    }
}
