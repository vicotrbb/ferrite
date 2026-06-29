use axum::{
    extract::Request,
    http::{header, HeaderMap, HeaderValue, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};

pub(super) async fn openai_preflight() -> Response {
    let mut response = StatusCode::NO_CONTENT.into_response();
    insert_openai_cors_headers(response.headers_mut());
    response
}

pub(super) async fn add_openai_cors_headers(request: Request, next: Next) -> Response {
    let is_openai_route = request.uri().path().starts_with("/v1/");
    let mut response = next.run(request).await;
    if is_openai_route {
        insert_openai_cors_headers(response.headers_mut());
    }
    response
}

fn insert_openai_cors_headers(headers: &mut HeaderMap) {
    headers.insert(
        header::ACCESS_CONTROL_ALLOW_ORIGIN,
        HeaderValue::from_static("*"),
    );
    headers.insert(
        header::ACCESS_CONTROL_ALLOW_METHODS,
        HeaderValue::from_static("GET, POST, OPTIONS"),
    );
    headers.insert(
        header::ACCESS_CONTROL_ALLOW_HEADERS,
        HeaderValue::from_static("authorization, content-type"),
    );
}
