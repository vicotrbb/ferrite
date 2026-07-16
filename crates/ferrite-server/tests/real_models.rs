//! Artifact-gated client and HTTP tests for supported real models.

mod support;

#[path = "real_models/openai_client_real_tier1_qwen_1_5b_q6.rs"]
mod client_qwen_1_5b_q6;
#[path = "real_models/openai_client_real_tier1_qwen_1_5b.rs"]
mod client_qwen_1_5b_q8;
#[path = "real_models/openai_client_real_tier1_qwen_1_5b_q8_long_chat.rs"]
mod client_qwen_1_5b_q8_long_chat;
#[path = "real_models/openai_client_real_tier1_qwen_1_5b_q8_long_completion.rs"]
mod client_qwen_1_5b_q8_long_completion;
#[path = "real_models/openai_client_real_tier1_smollm_1_7b.rs"]
mod client_smollm_1_7b;
#[path = "real_models/openai_client_real_tier0.rs"]
mod client_tier0;
#[path = "real_models/openai_client_real_tier1.rs"]
mod client_tier1;
#[path = "real_models/openai_client_real_tier1_catalog.rs"]
mod client_tier1_catalog;
#[path = "real_models/openai_real_phi3.rs"]
mod http_phi3;
#[path = "real_models/openai_real_tier1_qwen_1_5b_q6_prompts.rs"]
mod http_qwen_1_5b_q6_prompts;
#[path = "real_models/openai_real_tier1_qwen_1_5b_q6_queue.rs"]
mod http_qwen_1_5b_q6_queue;
#[path = "real_models/openai_real_tier1_qwen_1_5b_q6_stop.rs"]
mod http_qwen_1_5b_q6_stop;
#[path = "real_models/openai_real_tier1_qwen_1_5b_q6_streaming_prompts.rs"]
mod http_qwen_1_5b_q6_streaming_prompts;
#[path = "real_models/openai_real_tier1_qwen_1_5b_http.rs"]
mod http_qwen_1_5b_q8;
#[path = "real_models/openai_real_tier1_qwen_1_5b_q8_long_completion_stream.rs"]
mod http_qwen_1_5b_q8_long_completion_stream;
#[path = "real_models/openai_real_tier1_qwen_1_5b_q8_long_queue.rs"]
mod http_qwen_1_5b_q8_long_queue;
#[path = "real_models/openai_real_tier1_qwen_1_5b_q8_long_stream.rs"]
mod http_qwen_1_5b_q8_long_stream;
#[path = "real_models/openai_real_tier1_qwen_1_5b_prompts.rs"]
mod http_qwen_1_5b_q8_prompts;
#[path = "real_models/openai_real_tier1_qwen_1_5b_queue.rs"]
mod http_qwen_1_5b_q8_queue;
#[path = "real_models/openai_real_tier1_qwen_1_5b_q8_stop.rs"]
mod http_qwen_1_5b_q8_stop;
#[path = "real_models/openai_real_tier1_qwen_1_5b_streaming_prompts.rs"]
mod http_qwen_1_5b_q8_streaming_prompts;
#[path = "real_models/openai_real_tier1_qwen_1_5b_throughput.rs"]
mod http_qwen_1_5b_q8_throughput;
#[path = "real_models/openai_real_tier1_smollm_1_7b_chat.rs"]
mod http_smollm_1_7b_chat;
#[path = "real_models/openai_real_tier1_smollm_1_7b_prompts.rs"]
mod http_smollm_1_7b_prompts;
#[path = "real_models/openai_real_tier1_smollm_1_7b_stop.rs"]
mod http_smollm_1_7b_stop;
#[path = "real_models/openai_real_tier1_smollm_1_7b_stop_prompts.rs"]
mod http_smollm_1_7b_stop_prompts;
#[path = "real_models/openai_real_tier1_smollm_1_7b_streaming.rs"]
mod http_smollm_1_7b_streaming;
#[path = "real_models/openai_real_model_http.rs"]
mod http_tier0;
#[path = "real_models/openai_real_tier1_http.rs"]
mod http_tier1;
#[path = "real_models/openai_real_tier1_catalog.rs"]
mod http_tier1_catalog;
#[path = "real_models/openai_real_tier1_stop.rs"]
mod http_tier1_stop;
