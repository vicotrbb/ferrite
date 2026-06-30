mod auth;
mod catalog;
mod cors;
pub mod error;
mod generation;
mod guards;
mod json;
pub mod prompt;
pub mod routes;
pub mod schema;
mod stream_generation;
pub mod streaming;

#[cfg(test)]
mod auth_tests;
#[cfg(test)]
mod availability_tests;
#[cfg(test)]
mod catalog_tests;
#[cfg(test)]
mod chat_content_part_validation_tests;
#[cfg(test)]
mod chat_message_tool_tests;
#[cfg(test)]
mod chat_message_validation_tests;
#[cfg(test)]
mod chat_metadata_validation_tests;
#[cfg(test)]
mod chat_option_tests;
#[cfg(test)]
mod chat_request_validation_tests;
#[cfg(test)]
mod completion_concurrency_tests;
#[cfg(test)]
mod completion_option_tests;
#[cfg(test)]
mod completion_prompt_validation_tests;
#[cfg(test)]
mod completion_unsupported_tests;
#[cfg(test)]
mod cors_tests;
#[cfg(test)]
mod health_tests;
#[cfg(test)]
mod request_error_tests;
#[cfg(test)]
mod response_shape_assertions;
#[cfg(test)]
mod response_shape_tests;
#[cfg(test)]
mod response_stream_shape_tests;
#[cfg(test)]
mod route_streaming_tests;
#[cfg(test)]
mod routes_tests;
#[cfg(test)]
mod seed_tests;
#[cfg(test)]
mod service_tier_tests;
mod stop_filter;
#[cfg(test)]
mod stop_sequences_tests;
#[cfg(test)]
mod stream_options_tests;
#[cfg(test)]
mod test_support;
#[cfg(test)]
mod token_limit_tests;
#[cfg(test)]
mod unsupported_tests;
#[cfg(test)]
mod user_identifier_tests;
