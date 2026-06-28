pub mod error;
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
mod unsupported_tests;
