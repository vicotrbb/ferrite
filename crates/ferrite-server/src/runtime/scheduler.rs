use super::{
    GeneratedText, GenerationCacheOptions, GenerationControl, GenerationFinishReason,
    GenerationFinishSource, InferenceEngine, PromptCacheLookup, PromptCacheTrace, RuntimeError,
    TokenTextBuffer,
};
use crate::openai::stop_filter::StopSequenceFilter;
use ferrite_inference::prefix_cache::PrefixCacheKey;
use ferrite_inference::scalar::{
    ScalarLlamaSession, accept_token_contexts_batch, accept_token_ids_batch,
};
use std::collections::VecDeque;
use std::sync::{
    Arc,
    mpsc::{self, Receiver, RecvTimeoutError, SyncSender, TryRecvError, TrySendError},
};
use std::thread;
use std::time::{Duration, Instant};
use tokio::sync::mpsc as tokio_mpsc;

const EVENT_CHANNEL_CAPACITY: usize = 8;
const IDLE_POLL_INTERVAL: Duration = Duration::from_millis(1);
const BATCH_ADMISSION_WINDOW: Duration = Duration::from_millis(5);

#[derive(Clone, Debug)]
pub struct BatchScheduler {
    sender: SyncSender<ScheduledJob>,
    max_batch_streams: usize,
    max_queued_jobs: usize,
}

impl BatchScheduler {
    pub fn start(
        engine: Arc<InferenceEngine>,
        max_batch_streams: usize,
    ) -> Result<Self, RuntimeError> {
        Self::start_with_queue(engine, max_batch_streams, max_batch_streams.max(1))
    }

    pub fn start_with_queue(
        engine: Arc<InferenceEngine>,
        max_batch_streams: usize,
        max_queued_jobs: usize,
    ) -> Result<Self, RuntimeError> {
        let max_batch_streams = max_batch_streams.max(1);
        let max_queued_jobs = max_queued_jobs.max(1);
        let (sender, receiver) = mpsc::sync_channel(max_queued_jobs);
        thread::Builder::new()
            .name("ferrite-batch-scheduler".to_owned())
            .spawn(move || scheduler_loop(engine, receiver, max_batch_streams))
            .map_err(|error| {
                RuntimeError::new(format!("failed to start batch scheduler: {error}"))
            })?;
        Ok(Self {
            sender,
            max_batch_streams,
            max_queued_jobs,
        })
    }

    pub fn max_batch_streams(&self) -> usize {
        self.max_batch_streams
    }

    pub fn max_queued_jobs(&self) -> usize {
        self.max_queued_jobs
    }

    pub fn submit(
        &self,
        prompt: String,
        max_tokens: usize,
        stop_sequences: Vec<String>,
        cache_options: GenerationCacheOptions,
    ) -> Result<tokio_mpsc::Receiver<BatchedGenerationEvent>, RuntimeError> {
        let (events, receiver) = tokio_mpsc::channel(EVENT_CHANNEL_CAPACITY);
        self.sender
            .try_send(ScheduledJob {
                prompt,
                max_tokens,
                stop_sequences,
                cache_options,
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
    cache_options: GenerationCacheOptions,
    events: tokio_mpsc::Sender<BatchedGenerationEvent>,
}

#[derive(Debug)]
struct PreparedJob<'model> {
    session: Option<ScalarLlamaSession<'model>>,
    events: tokio_mpsc::Sender<BatchedGenerationEvent>,
    stop_sequences: Vec<String>,
    prompt_token_ids: Vec<usize>,
    prefix_cache_key: PrefixCacheKey,
    cache_options: GenerationCacheOptions,
    prompt_cache_trace: Option<PromptCacheTrace>,
    cached_prompt_tokens: usize,
    prefill_start: usize,
    cache_store_needed: bool,
    max_tokens: usize,
    first_token_id: Option<usize>,
}

impl<'model> PreparedJob<'model> {
    fn new(engine: &'model InferenceEngine, job: ScheduledJob) -> Result<Self, RuntimeError> {
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
        engine.validate_kv_capacity(prompt_token_ids.len(), job.max_tokens)?;

        let prefix_cache_key =
            engine.prefix_cache_key_for_tokens(&prompt_token_ids, &job.cache_options);
        let mut session = engine.start_session()?;
        let mut prompt_cache_trace = None;
        let mut cached_prompt_tokens = 0;
        let mut prefill_start = 0;
        let mut first_token_id = None;
        let mut cache_store_needed = job.cache_options.prefix_cache_enabled();
        if job.cache_options.prefix_cache_enabled() {
            let lookup = engine.prefix_cache_lookup(&prefix_cache_key)?;
            if job.cache_options.prompt_cache_trace_enabled() {
                prompt_cache_trace = Some(lookup.to_trace(&prefix_cache_key, true));
            }
            if let Some(cached) = lookup.into_value() {
                session
                    .restore_cache_snapshot(cached.snapshot())
                    .map_err(|error| {
                        RuntimeError::new(format!("failed to restore prompt cache: {error}"))
                    })?;
                cached_prompt_tokens = cached.snapshot().cached_token_count();
                prefill_start = cached_prompt_tokens;
                if prefill_start == prompt_token_ids.len() {
                    first_token_id = cached.next_token_id();
                    cache_store_needed = first_token_id.is_none();
                    if first_token_id.is_none() {
                        prefill_start = prefill_start.checked_sub(1).ok_or_else(|| {
                            RuntimeError::new(
                                "prefix cache hit cannot recover a token for an empty prompt",
                            )
                        })?;
                        cached_prompt_tokens = prefill_start;
                        session.truncate_cache(prefill_start).map_err(|error| {
                            RuntimeError::new(format!(
                                "failed to truncate prompt cache for greedy recovery: {error}"
                            ))
                        })?;
                    }
                }
            }
        } else if job.cache_options.prompt_cache_trace_enabled() {
            prompt_cache_trace = Some(PromptCacheTrace::new(
                false,
                prefix_cache_key.namespace().map(str::to_owned),
                prefix_cache_key.prefix_token_count(),
                prefix_cache_key.prefix_token_hash(),
                PromptCacheLookup::Disabled,
            ));
        }

        Ok(Self {
            session: Some(session),
            events: job.events,
            stop_sequences: job.stop_sequences,
            prompt_token_ids,
            prefix_cache_key,
            cache_options: job.cache_options,
            prompt_cache_trace,
            cached_prompt_tokens,
            prefill_start,
            cache_store_needed,
            max_tokens: job.max_tokens,
            first_token_id,
        })
    }

    fn into_active(self, engine: &InferenceEngine) -> Result<ActiveJob<'model>, RuntimeError> {
        let first_token_id = self.first_token_id.ok_or_else(|| {
            RuntimeError::new("batch scheduler invariant failed: prefill produced no token")
        })?;
        let mut active = ActiveJob {
            session: self.session,
            events: self.events,
            pending_events: VecDeque::new(),
            stop_filter: StopSequenceFilter::new(self.stop_sequences),
            prompt_tokens: self.prompt_token_ids.len(),
            cached_prompt_tokens: self.cached_prompt_tokens,
            prompt_cache_trace: self.prompt_cache_trace,
            remaining_tokens: self.max_tokens,
            decode_input_token_id: None,
            generated_token_ids: Vec::with_capacity(self.max_tokens),
            token_texts: Vec::with_capacity(self.max_tokens),
            token_id_chunks: Vec::with_capacity(self.max_tokens),
            token_text_buffer: TokenTextBuffer::new(),
            finished: false,
        };
        active.process_token(engine, first_token_id)?;
        Ok(active)
    }
}

#[derive(Debug)]
struct ActiveJob<'model> {
    session: Option<ScalarLlamaSession<'model>>,
    events: tokio_mpsc::Sender<BatchedGenerationEvent>,
    pending_events: VecDeque<BatchedGenerationEvent>,
    stop_filter: StopSequenceFilter,
    prompt_tokens: usize,
    cached_prompt_tokens: usize,
    prompt_cache_trace: Option<PromptCacheTrace>,
    remaining_tokens: usize,
    decode_input_token_id: Option<usize>,
    generated_token_ids: Vec<usize>,
    token_texts: Vec<String>,
    token_id_chunks: Vec<Vec<usize>>,
    token_text_buffer: TokenTextBuffer,
    finished: bool,
}

impl<'model> ActiveJob<'model> {
    fn process_token(
        &mut self,
        engine: &InferenceEngine,
        token_id: usize,
    ) -> Result<(), RuntimeError> {
        self.generated_token_ids.push(token_id);
        self.remaining_tokens = self.remaining_tokens.saturating_sub(1);
        if engine.tokenizer.is_end_of_generation_token(token_id) {
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
        .with_token_id_chunks(std::mem::take(&mut self.token_id_chunks))?
        .with_cached_prompt_tokens(self.cached_prompt_tokens)?
        .with_optional_prompt_cache_trace(self.prompt_cache_trace.take())?;
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
            let reservation = self.events.try_reserve();
            match reservation {
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
            };
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
        let mut pending_jobs = Vec::with_capacity(max_batch_streams.saturating_sub(active.len()));
        if active.is_empty() && !input_closed {
            match receiver.recv() {
                Ok(job) => pending_jobs.push(job),
                Err(_) => input_closed = true,
            };
            let admission_deadline = Instant::now() + BATCH_ADMISSION_WINDOW;
            while pending_jobs.len() < max_batch_streams && !input_closed {
                let remaining = admission_deadline.saturating_duration_since(Instant::now());
                if remaining.is_zero() {
                    break;
                }
                match receiver.recv_timeout(remaining) {
                    Ok(job) => pending_jobs.push(job),
                    Err(RecvTimeoutError::Timeout) => break,
                    Err(RecvTimeoutError::Disconnected) => input_closed = true,
                };
            }
        }
        while active.len() + pending_jobs.len() < max_batch_streams && !input_closed {
            match receiver.try_recv() {
                Ok(job) => pending_jobs.push(job),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => input_closed = true,
            };
        }
        if !pending_jobs.is_empty() {
            admit_jobs(&engine, &mut active, pending_jobs);
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
                    Ok(job) => admit_jobs(&engine, &mut active, vec![job]),
                    Err(RecvTimeoutError::Timeout) => {}
                    Err(RecvTimeoutError::Disconnected) => input_closed = true,
                };
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

fn admit_jobs<'model>(
    engine: &'model InferenceEngine,
    active: &mut Vec<ActiveJob<'model>>,
    jobs: Vec<ScheduledJob>,
) {
    let mut prepared = Vec::with_capacity(jobs.len());
    for job in jobs {
        if job.events.is_closed() {
            continue;
        }
        let error_sender = job.events.clone();
        match PreparedJob::new(engine, job) {
            Ok(job) => prepared.push(job),
            Err(error) => {
                let _ = error_sender.blocking_send(BatchedGenerationEvent::Failed(error));
            }
        };
    }
    if prepared.is_empty() {
        return;
    }

    if let Err(error) = prefill_jobs(&mut prepared) {
        let error = RuntimeError::new(format!("failed to batch prefill prompt: {error}"));
        for job in prepared {
            let _ = job
                .events
                .blocking_send(BatchedGenerationEvent::Failed(error.clone()));
        }
        return;
    }

    for mut job in prepared {
        if job.events.is_closed() {
            continue;
        }
        let error_sender = job.events.clone();
        if job.cache_options.prefix_cache_enabled() && job.cache_store_needed {
            let cache_result = (|| {
                let first_token_id = job.first_token_id.ok_or_else(|| {
                    RuntimeError::new(
                        "batch scheduler invariant failed: cached prefill produced no token",
                    )
                })?;
                let snapshot = job
                    .session
                    .as_mut()
                    .ok_or_else(|| {
                        RuntimeError::new(
                            "batch scheduler invariant failed: cached prefill session is missing",
                        )
                    })?
                    .cache_snapshot()
                    .map_err(|error| RuntimeError::new(error.to_string()))?;
                engine.store_prefix_cache_greedy_value(
                    job.prefix_cache_key.clone(),
                    snapshot,
                    first_token_id,
                )
            })();
            if let Err(error) = cache_result {
                let _ = error_sender.blocking_send(BatchedGenerationEvent::Failed(error));
                continue;
            }
        }
        match job.into_active(engine) {
            Ok(job) => active.push(job),
            Err(error) => {
                let _ = error_sender.blocking_send(BatchedGenerationEvent::Failed(error));
            }
        };
    }
}

fn prefill_jobs(jobs: &mut [PreparedJob<'_>]) -> Result<(), RuntimeError> {
    let prompt_token_ids = jobs
        .iter()
        .map(|job| job.prompt_token_ids.as_slice())
        .collect::<Vec<_>>();
    let cache_options = jobs
        .iter()
        .map(|job| &job.cache_options)
        .collect::<Vec<_>>();
    let prompt_groups = equal_prompt_groups(&prompt_token_ids, &cache_options);
    let representatives = prompt_groups
        .iter()
        .filter(|group| group_has_open_receiver(jobs, group))
        .filter_map(|group| group.first().copied())
        .collect::<Vec<_>>();
    let max_prompt_len = representatives
        .iter()
        .map(|index| jobs[*index].prompt_token_ids.len())
        .max()
        .unwrap_or(0);
    for position in 0..max_prompt_len {
        let context_indices = representatives
            .iter()
            .copied()
            .filter(|index| {
                representative_has_open_receiver(jobs, &prompt_groups, *index)
                    && jobs[*index].first_token_id.is_none()
                    && position >= jobs[*index].prefill_start
                    && position + 1 < jobs[*index].prompt_token_ids.len()
            })
            .collect::<Vec<_>>();
        advance_prefill_contexts(jobs, &context_indices, position)?;

        let final_indices = representatives
            .iter()
            .copied()
            .filter(|index| {
                representative_has_open_receiver(jobs, &prompt_groups, *index)
                    && jobs[*index].first_token_id.is_none()
                    && position >= jobs[*index].prefill_start
                    && position + 1 == jobs[*index].prompt_token_ids.len()
            })
            .collect::<Vec<_>>();
        advance_prefill_finals(jobs, &final_indices, position)?;
    }

    restore_equal_prompt_sessions(jobs, prompt_groups)?;
    Ok(())
}

fn equal_prompt_groups(
    prompts: &[&[usize]],
    cache_options: &[&GenerationCacheOptions],
) -> Vec<Vec<usize>> {
    debug_assert_eq!(prompts.len(), cache_options.len());
    let mut groups = Vec::<Vec<usize>>::new();
    for (index, prompt) in prompts.iter().enumerate() {
        if let Some(group) = groups.iter_mut().find(|group| {
            prompts[group[0]] == *prompt && cache_options[group[0]] == cache_options[index]
        }) {
            group.push(index);
        } else {
            groups.push(vec![index]);
        }
    }
    groups
}

fn group_has_open_receiver(jobs: &[PreparedJob<'_>], group: &[usize]) -> bool {
    group.iter().any(|index| !jobs[*index].events.is_closed())
}

fn representative_has_open_receiver(
    jobs: &[PreparedJob<'_>],
    groups: &[Vec<usize>],
    representative: usize,
) -> bool {
    groups
        .iter()
        .find(|group| group.first() == Some(&representative))
        .is_some_and(|group| group_has_open_receiver(jobs, group))
}

fn restore_equal_prompt_sessions(
    jobs: &mut [PreparedJob<'_>],
    prompt_groups: Vec<Vec<usize>>,
) -> Result<(), RuntimeError> {
    for group in prompt_groups {
        if !group_has_open_receiver(jobs, &group) {
            continue;
        }
        let Some((&representative, duplicates)) = group.split_first() else {
            continue;
        };
        if duplicates.is_empty() {
            continue;
        }
        let first_token_id = jobs[representative].first_token_id.ok_or_else(|| {
            RuntimeError::new("batch scheduler invariant failed: prefill produced no token")
        })?;
        let snapshot = jobs[representative]
            .session
            .as_mut()
            .ok_or_else(|| {
                RuntimeError::new("batch scheduler invariant failed: prefill session is missing")
            })?
            .cache_snapshot()
            .map_err(|error| RuntimeError::new(error.to_string()))?;

        for index in duplicates {
            if jobs[*index].events.is_closed() {
                continue;
            }
            jobs[*index]
                .session
                .as_mut()
                .ok_or_else(|| {
                    RuntimeError::new(
                        "batch scheduler invariant failed: duplicate prefill session is missing",
                    )
                })?
                .restore_cache_snapshot(&snapshot)
                .map_err(|error| RuntimeError::new(error.to_string()))?;
            jobs[*index].first_token_id = Some(first_token_id);
        }
    }
    Ok(())
}

fn advance_prefill_contexts(
    jobs: &mut [PreparedJob<'_>],
    indices: &[usize],
    position: usize,
) -> Result<(), RuntimeError> {
    if indices.is_empty() {
        return Ok(());
    }
    let (mut sessions, token_ids) = take_prefill_inputs(jobs, indices, position)?;
    let result = if sessions.len() == 1 {
        sessions[0].accept_token_context_only(token_ids[0])
    } else {
        accept_token_contexts_batch(&mut sessions, &token_ids)
    };
    restore_prefill_sessions(jobs, indices, sessions);
    result.map_err(|error| RuntimeError::new(error.to_string()))
}

fn advance_prefill_finals(
    jobs: &mut [PreparedJob<'_>],
    indices: &[usize],
    position: usize,
) -> Result<(), RuntimeError> {
    if indices.is_empty() {
        return Ok(());
    }
    let (mut sessions, token_ids) = take_prefill_inputs(jobs, indices, position)?;
    let result = if sessions.len() == 1 {
        sessions[0]
            .accept_token_id(token_ids[0])
            .map(|token_id| vec![token_id])
    } else {
        accept_token_ids_batch(&mut sessions, &token_ids)
    };
    restore_prefill_sessions(jobs, indices, sessions);
    for (index, token_id) in indices
        .iter()
        .copied()
        .zip(result.map_err(|error| RuntimeError::new(error.to_string()))?)
    {
        jobs[index].first_token_id = Some(token_id);
    }
    Ok(())
}

fn take_prefill_inputs<'model>(
    jobs: &mut [PreparedJob<'model>],
    indices: &[usize],
    position: usize,
) -> Result<(Vec<ScalarLlamaSession<'model>>, Vec<usize>), RuntimeError> {
    let mut sessions = Vec::with_capacity(indices.len());
    let mut token_ids = Vec::with_capacity(indices.len());
    for index in indices {
        sessions.push(jobs[*index].session.take().ok_or_else(|| {
            RuntimeError::new("batch scheduler invariant failed: prefill session is missing")
        })?);
        token_ids.push(jobs[*index].prompt_token_ids[position]);
    }
    Ok((sessions, token_ids))
}

fn restore_prefill_sessions<'model>(
    jobs: &mut [PreparedJob<'model>],
    indices: &[usize],
    sessions: Vec<ScalarLlamaSession<'model>>,
) {
    for (index, session) in indices.iter().copied().zip(sessions) {
        jobs[index].session = Some(session);
    }
}

#[cfg(test)]
mod tests {
    use super::{BatchScheduler, BatchedGenerationEvent, equal_prompt_groups};
    use crate::runtime::{
        GenerationCacheOptions, GenerationFinishReason, GenerationFinishSource, InferenceEngine,
    };
    use std::sync::Arc;

    #[test]
    fn groups_equal_prompts_by_first_arrival() {
        let prompts: [&[usize]; 5] = [&[1, 2], &[3], &[1, 2], &[4, 5], &[3]];
        let options: [GenerationCacheOptions; 5] =
            std::array::from_fn(|_| GenerationCacheOptions::default());
        let option_refs = options.iter().collect::<Vec<_>>();

        assert_eq!(
            equal_prompt_groups(&prompts, &option_refs),
            [vec![0, 2], vec![1, 4], vec![3]]
        );
    }

    #[test]
    fn isolates_equal_prompts_across_cache_namespaces() {
        let prompts: [&[usize]; 3] = [&[1, 2], &[1, 2], &[1, 2]];
        let options = [
            GenerationCacheOptions::from_namespace(Some("tenant-a".to_owned()))
                .with_prefix_cache_enabled(true),
            GenerationCacheOptions::from_namespace(Some("tenant-b".to_owned()))
                .with_prefix_cache_enabled(true),
            GenerationCacheOptions::from_namespace(Some("tenant-a".to_owned()))
                .with_prefix_cache_enabled(true),
        ];
        let option_refs = options.iter().collect::<Vec<_>>();

        assert_eq!(
            equal_prompt_groups(&prompts, &option_refs),
            [vec![0, 2], vec![1]]
        );
    }

    #[test]
    fn scheduler_stops_on_explicit_eot_token() -> Result<(), Box<dyn std::error::Error>> {
        let model_path = std::env::temp_dir().join(format!(
            "ferrite-scheduler-eot-fixture-{}.gguf",
            std::process::id()
        ));
        std::fs::write(
            &model_path,
            ferrite_fixtures::scalar_llama_f32_gguf_fixture_with_eot_token_id(2),
        )?;
        let engine = Arc::new(InferenceEngine::load(&model_path)?);
        std::fs::remove_file(&model_path)?;
        let scheduler = BatchScheduler::start(engine, 1)?;
        let mut events = scheduler.submit(
            "hello".to_owned(),
            4,
            Vec::new(),
            GenerationCacheOptions::default(),
        )?;

        let generated = match events.blocking_recv() {
            Some(BatchedGenerationEvent::Finished(generated)) => generated,
            Some(BatchedGenerationEvent::Token { text, token_ids }) => {
                return Err(format!(
                    "terminal EOT became visible: text={text:?}, token_ids={token_ids:?}"
                )
                .into());
            }
            Some(BatchedGenerationEvent::Failed(error)) => return Err(error.into()),
            None => return Err("scheduler closed before finishing generation".into()),
        };

        assert_eq!(generated.finish_reason(), GenerationFinishReason::Stop);
        assert_eq!(generated.finish_source(), GenerationFinishSource::Eos);
        assert_eq!(generated.completion_tokens(), 1);
        assert_eq!(generated.text(), "");
        Ok(())
    }
}
