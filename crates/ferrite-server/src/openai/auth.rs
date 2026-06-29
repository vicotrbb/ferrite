use super::error::OpenAiHttpError;
use crate::state::ServerState;
use axum::http::{header, HeaderMap};

pub(super) fn ensure_authorized(
    state: &ServerState,
    headers: &HeaderMap,
) -> Result<(), OpenAiHttpError> {
    let Some(api_key) = state.api_key() else {
        return Ok(());
    };
    let Some(header) = headers.get(header::AUTHORIZATION) else {
        return Err(OpenAiHttpError::authentication_required(
            "missing Authorization bearer token",
        ));
    };
    let Ok(header) = header.to_str() else {
        return Err(OpenAiHttpError::authentication_required(
            "invalid Authorization bearer token",
        ));
    };
    if bearer_token_matches(header, api_key) {
        Ok(())
    } else {
        Err(OpenAiHttpError::authentication_required(
            "invalid Authorization bearer token",
        ))
    }
}

fn bearer_token_matches(header: &str, api_key: &str) -> bool {
    let mut parts = header.split_whitespace();
    let Some(scheme) = parts.next() else {
        return false;
    };
    let Some(token) = parts.next() else {
        return false;
    };
    parts.next().is_none() && scheme.eq_ignore_ascii_case("Bearer") && token == api_key
}
