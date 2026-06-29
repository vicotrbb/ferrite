mod auth;
pub mod error;
mod generation;
mod json;
pub mod prompt;
pub mod routes;
pub mod schema;
pub mod streaming;

#[cfg(test)]
mod auth_tests;
#[cfg(test)]
mod availability_tests;
#[cfg(test)]
mod catalog_tests;
#[cfg(test)]
mod chat_message_tool_tests;
#[cfg(test)]
mod chat_message_validation_tests;
#[cfg(test)]
mod chat_option_tests;
#[cfg(test)]
mod chat_request_validation_tests;
#[cfg(test)]
mod completion_option_tests;
#[cfg(test)]
mod completion_unsupported_tests;
#[cfg(test)]
mod health_tests;
#[cfg(test)]
mod request_error_tests;
#[cfg(test)]
mod response_shape_tests;
#[cfg(test)]
mod routes_tests;
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
