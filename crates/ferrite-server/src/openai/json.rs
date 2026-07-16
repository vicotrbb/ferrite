use super::{auth::ensure_authorized, error::OpenAiHttpError};
use crate::state::ServerState;
use axum::{
    Json, async_trait,
    extract::{FromRequest, Request, rejection::JsonRejection},
};

pub struct AuthorizedOpenAiJson<T>(pub T);

#[async_trait]
impl<T> FromRequest<ServerState> for AuthorizedOpenAiJson<T>
where
    Json<T>: FromRequest<ServerState, Rejection = JsonRejection>,
    T: Send,
{
    type Rejection = OpenAiHttpError;

    async fn from_request(request: Request, state: &ServerState) -> Result<Self, Self::Rejection> {
        ensure_authorized(state, request.headers())?;
        Json::<T>::from_request(request, state)
            .await
            .map(|Json(value)| Self(value))
            .map_err(|error| OpenAiHttpError::invalid_request(error.to_string()))
    }
}
