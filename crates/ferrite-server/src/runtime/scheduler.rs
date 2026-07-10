use super::{
    GeneratedText, GenerationControl, GenerationFinishReason, GenerationFinishSource,
    InferenceEngine, RuntimeError, TokenTextBuffer,
};
use crate::openai::stop_filter::StopSequenceFilter;
use ferrite_inference::scalar::{accept_token_ids_batch, ScalarLlamaSession};
use std::collections::VecDeque;
use std::sync::{
    mpsc::{self, Receiver, RecvTimeoutError, SyncSender, TryRecvError, TrySendError},
    Arc,
};
use std::thread;
use std::time::Duration;
use tokio::sync::mpsc as tokio_mpsc;

const EVENT_CHANNEL_CAPACITY: usize = 8;
const IDLE_POLL_INTERVAL: Duration = Duration::from_millis(1);

#[derive(Clone, Debug)]
pub struct BatchScheduler {
    sender: SyncSender<ScheduledJob>,
    max_batch_streams: usize,
}

impl BatchScheduler {
    pub fn start(
        engine: Arc<InferenceEngine>,
        max_batch_streams: usize,
    ) -> Result<Self, RuntimeError> {
        let max_batch_streams = max_batch_streams.max(1);
        let (sender, receiver) = mpsc::sync_channel(max_batch_streams * 2);
        thread::Builder::new()
            .name("ferrite-batch-scheduler".to_owned())
            .spawn(move || scheduler_loop(engine, receiver, max_batch_streams))
            .map_err(|error| {
                RuntimeError::new(format!("failed to start batch scheduler: {error}"))
            })?;
        Ok(Self {
            sender,
            max_batch_streams,
        })
    }

    pub fn max_batch_streams(&self) -> usize {
        self.max_batch_streams
    }

    pub fn submit(
        &self,
        prompt: String,
        max_tokens: usize,
        stop_sequences: Vec<String>,
    ) -> Result<tokio_mpsc::Receiver<BatchedGenerationEvent>, RuntimeError> {
        let (events, receiver) = tokio_mpsc::channel(EVENT_CHANNEL_CAPACITY);
        self.sender
            .try_send(ScheduledJob {
                prompt,
                max_tokens,
                stop_sequences,
                events,
            })
            .map_err(|error| match error {
                TrySendError::Full(_) => RuntimeError::new("batch scheduler queue is full"),
                TrySendError::Disconnected(_) => RuntimeError::new("batch scheduler stopped"),
            })?;
        Ok(receiver)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BatchedGenerationEvent {
    Token { text: String, token_ids: Vec<usize> },
    Finished(GeneratedText),
    Failed(RuntimeError),
}

#[derive(Debug)]
struct ScheduledJob {
    prompt: String,
    max_tokens: usize,
    stop_sequences: Vec<String>,
    events: tokio_mpsc::Sender<BatchedGenerationEvent>,
}

#[derive(Debug)]
struct ActiveJob<'model> {
    session: Option<ScalarLlamaSession<'model>>,
    events: tokio_mpsc::Sender<BatchedGenerationEvent>,
    pending_events: VecDeque<BatchedGenerationEvent>,
    stop_filter: StopSequenceFilter,
    prompt_tokens: usize,
    remaining_tokens: usize,
    decode_input_token_id: Option<usize>,
    generated_token_ids: Vec<usize>,
    token_texts: Vec<String>,
    token_id_chunks: Vec<Vec<usize>>,
    token_text_buffer: TokenTextBuffer,
    finished: bool,
}

impl<'model> ActiveJob<'model> {
    fn prepare(engine: &'model InferenceEngine, job: ScheduledJob) -> Result<Self, RuntimeError> {
        let prompt_token_ids = engine
            .tokenizer
            .encode(&job.prompt)
            .map_err(|error| RuntimeError::new(format!("failed to tokenize prompt: {error}")))?;
        if prompt_token_ids.is_empty() {
            return Err(RuntimeError::new("prompt must contain at least one token"));
        }
        if job.max_tokens == 0 {
            return Err(RuntimeError::new("max tokens must be greater than zero"));
        }

        let mut session = engine.start_session()?;
        let first = session
            .accept_prompt(&prompt_token_ids)
            .map_err(|error| RuntimeError::new(format!("failed to evaluate prompt: {error}")))?;
        let mut active = Self {
            session: Some(session),
            events: job.events,
            pending_events: VecDeque::new(),
            stop_filter: StopSequenceFilter::new(job.stop_sequences),
            prompt_tokens: prompt_token_ids.len(),
            remaining_tokens: job.max_tokens,
            decode_input_token_id: None,
            generated_token_ids: Vec::with_capacity(job.max_tokens),
            token_texts: Vec::with_capacity(job.max_tokens),
            token_id_chunks: Vec::with_capacity(job.max_tokens),
            token_text_buffer: TokenTextBuffer::new(),
            finished: false,
        };
        active.process_token(engine, first.token_id)?;
        Ok(active)
    }

    fn process_token(
        &mut self,
        engine: &InferenceEngine,
        token_id: usize,
    ) -> Result<(), RuntimeError> {
        self.generated_token_ids.push(token_id);
        self.remaining_tokens = self.remaining_tokens.saturating_sub(1);
        if Some(token_id) == engine.tokenizer.eos_token_id() {
            return self.finish(
                engine,
                GenerationFinishReason::Stop,
                GenerationFinishSource::Eos,
                true,
            );
        }

        let mut ready_piece = None;
        self.token_text_buffer.emit_ready_text(
            &self.generated_token_ids,
            |ids| engine.decode_token_text(ids),
            |text, ids| {
                ready_piece = Some((text.to_owned(), ids.to_vec()));
                Ok(GenerationControl::Continue)
            },
        )?;
        if let Some((text, token_ids)) = ready_piece {
            self.token_texts.push(text.clone());
            self.token_id_chunks.push(token_ids.clone());
            for visible in self.stop_filter.push(&text) {
                self.pending_events
                    .push_back(BatchedGenerationEvent::Token {
                        text: visible,
                        token_ids: token_ids.clone(),
                    });
            }
        }

        if self.stop_filter.stopped() {
            self.finish(
                engine,
                GenerationFinishReason::Stop,
                GenerationFinishSource::StopSequence,
                false,
            )
        } else if self.remaining_tokens == 0 {
            self.finish(
                engine,
                GenerationFinishReason::Length,
                GenerationFinishSource::Length,
                false,
            )
        } else {
            self.decode_input_token_id = Some(token_id);
            Ok(())
        }
    }

    fn finish(
        &mut self,
        engine: &InferenceEngine,
        finish_reason: GenerationFinishReason,
        finish_source: GenerationFinishSource,
        stopped_on_eos: bool,
    ) -> Result<(), RuntimeError> {
        if !self.stop_filter.stopped() {
            let filter =
                std::mem::replace(&mut self.stop_filter, StopSequenceFilter::new(Vec::new()));
            for visible in filter.finish() {
                self.pending_events
                    .push_back(BatchedGenerationEvent::Token {
                        text: visible,
                        token_ids: Vec::new(),
                    });
            }
        }

        let visible_token_ids = if stopped_on_eos {
            &self.generated_token_ids[..self.generated_token_ids.len().saturating_sub(1)]
        } else {
            &self.generated_token_ids
        };
        let text = if visible_token_ids.is_empty() {
            String::new()
        } else {
            engine
                .tokenizer
                .decode(visible_token_ids)
                .map_err(|error| {
                    RuntimeError::new(format!("failed to decode completion: {error}"))
                })?
        };
        let generated = GeneratedText::with_finish_reason(
            text,
            self.prompt_tokens,
            self.generated_token_ids.len(),
            std::mem::take(&mut self.token_texts),
            finish_reason,
        )
        .with_finish_source(finish_source)
        .with_token_id_chunks(std::mem::take(&mut self.token_id_chunks))?;
        self.pending_events
            .push_back(BatchedGenerationEvent::Finished(generated));
        self.decode_input_token_id = None;
        self.finished = true;
        Ok(())
    }

    fn fail(&mut self, error: RuntimeError) {
        self.pending_events
            .push_back(BatchedGenerationEvent::Failed(error));
        self.decode_input_token_id = None;
        self.finished = true;
    }

    fn flush_pending_events(&mut self) {
        while !self.pending_events.is_empty() {
            match self.events.try_reserve() {
                Ok(permit) => {
                    if let Some(event) = self.pending_events.pop_front() {
                        permit.send(event);
                    }
                }
                Err(tokio_mpsc::error::TrySendError::Full(_)) => break,
                Err(tokio_mpsc::error::TrySendError::Closed(_)) => {
                    self.pending_events.clear();
                    self.finished = true;
                    self.decode_input_token_id = None;
                    break;
                }
            }
        }
    }

    fn ready_to_decode(&self) -> bool {
        !self.finished
            && self.pending_events.is_empty()
            && !self.events.is_closed()
            && self.decode_input_token_id.is_some()
    }

    fn retired(&self) -> bool {
        self.events.is_closed() || (self.finished && self.pending_events.is_empty())
    }
}

fn scheduler_loop(
    engine: Arc<InferenceEngine>,
    receiver: Receiver<ScheduledJob>,
    max_batch_streams: usize,
) {
    let mut active = Vec::<ActiveJob<'_>>::with_capacity(max_batch_streams);
    let mut input_closed = false;

    loop {
        if active.is_empty() && !input_closed {
            match receiver.recv() {
                Ok(job) => admit_job(&engine, &mut active, job),
                Err(_) => input_closed = true,
            }
        }
        while active.len() < max_batch_streams && !input_closed {
            match receiver.try_recv() {
                Ok(job) => admit_job(&engine, &mut active, job),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => input_closed = true,
            }
        }

        for job in &mut active {
            job.flush_pending_events();
        }
        active.retain(|job| !job.retired());
        if active.is_empty() && input_closed {
            break;
        }

        let ready = active
            .iter()
            .enumerate()
            .filter_map(|(index, job)| job.ready_to_decode().then_some(index))
            .collect::<Vec<_>>();
        if ready.is_empty() {
            if active.len() < max_batch_streams && !input_closed {
                match receiver.recv_timeout(IDLE_POLL_INTERVAL) {
                    Ok(job) => admit_job(&engine, &mut active, job),
                    Err(RecvTimeoutError::Timeout) => {}
                    Err(RecvTimeoutError::Disconnected) => input_closed = true,
                }
            } else {
                thread::yield_now();
            }
            continue;
        }

        let mut decode_indices = Vec::with_capacity(ready.len());
        let mut sessions = Vec::with_capacity(ready.len());
        let mut token_ids = Vec::with_capacity(ready.len());
        for index in ready {
            let Some(session) = active[index].session.take() else {
                active[index].fail(RuntimeError::new(
                    "batch scheduler invariant failed: ready job has no session",
                ));
                continue;
            };
            let Some(token_id) = active[index].decode_input_token_id.take() else {
                active[index].session = Some(session);
                active[index].fail(RuntimeError::new(
                    "batch scheduler invariant failed: ready job has no decode token",
                ));
                continue;
            };
            decode_indices.push(index);
            sessions.push(session);
            token_ids.push(token_id);
        }
        if sessions.is_empty() {
            continue;
        }

        if sessions.len() == 1 {
            let index = decode_indices[0];
            let mut session = sessions.remove(0);
            let token_id = token_ids[0];
            match session.accept_token_id(token_id) {
                Ok(next_token_id) => {
                    active[index].session = Some(session);
                    if let Err(error) = active[index].process_token(&engine, next_token_id) {
                        active[index].fail(error);
                    }
                }
                Err(error) => {
                    active[index].session = Some(session);
                    active[index].fail(RuntimeError::new(format!(
                        "failed to decode token: {error}"
                    )));
                }
            }
            continue;
        }

        match accept_token_ids_batch(&mut sessions, &token_ids) {
            Ok(next_token_ids) => {
                for ((index, session), next_token_id) in
                    decode_indices.into_iter().zip(sessions).zip(next_token_ids)
                {
                    active[index].session = Some(session);
                    if let Err(error) = active[index].process_token(&engine, next_token_id) {
                        active[index].fail(error);
                    }
                }
            }
            Err(error) => {
                let error = RuntimeError::new(format!("failed to batch decode token: {error}"));
                for (index, session) in decode_indices.into_iter().zip(sessions) {
                    active[index].session = Some(session);
                    active[index].fail(error.clone());
                }
            }
        }
    }
}

fn admit_job<'model>(
    engine: &'model InferenceEngine,
    active: &mut Vec<ActiveJob<'model>>,
    job: ScheduledJob,
) {
    let error_sender = job.events.clone();
    match ActiveJob::prepare(engine, job) {
        Ok(job) => active.push(job),
        Err(error) => {
            let _ = error_sender.blocking_send(BatchedGenerationEvent::Failed(error));
        }
    }
}
