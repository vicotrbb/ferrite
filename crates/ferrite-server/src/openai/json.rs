use super::error::OpenAiHttpError;
use axum::{
    async_trait,
    extract::{rejection::JsonRejection, FromRequest, Request},
    Json,
};

pub struct OpenAiJson<T>(pub T);

#[async_trait]
impl<S, T> FromRequest<S> for OpenAiJson<T>
where
    Json<T>: FromRequest<S, Rejection = JsonRejection>,
    S: Send + Sync,
{
    type Rejection = OpenAiHttpError;

    async fn from_request(request: Request, state: &S) -> Result<Self, Self::Rejection> {
        Json::<T>::from_request(request, state)
            .await
            .map(|Json(value)| Self(value))
            .map_err(|error| OpenAiHttpError::invalid_request(error.to_string()))
    }
}
