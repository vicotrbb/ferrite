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
mod catalog_tests;
#[cfg(test)]
mod routes_tests;
#[cfg(test)]
mod service_tier_tests;
#[cfg(test)]
mod stream_options_tests;
#[cfg(test)]
mod unsupported_tests;
