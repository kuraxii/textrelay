use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::Router;

use crate::transport::{Transport, TransportError};

use super::html::HTML;

/// 顶层错误转换为 HTTP 响应
impl IntoResponse for TransportError {
    fn into_response(self) -> Response {
        let (status, body) = match &self {
            TransportError::NotConnected(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.clone()),
            TransportError::Timeout => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
            TransportError::Protocol(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.clone()),
        };
        (status, body).into_response()
    }
}

/// `GET /` — 返回手机端 HTML 页面
async fn index() -> impl IntoResponse {
    (StatusCode::OK, [("Content-Type", "text/html; charset=utf-8")], HTML)
}

/// `POST /commit` — 接收文本并提交到 fcitx5
async fn commit(
    State(transport): State<Arc<dyn Transport>>,
    body: String,
) -> Result<impl IntoResponse, TransportError> {
    let text = body.trim().to_string();
    if text.is_empty() {
        return Ok((StatusCode::OK, "EMPTY".to_string()));
    }

    match transport.commit(&text).await? {
        true => Ok((StatusCode::OK, "OK".to_string())),
        false => Ok((StatusCode::OK, "NO_FOCUS".to_string())),
    }
}

/// 构建路由器
pub fn router(transport: Arc<dyn Transport>) -> Router {
    Router::new()
        .route("/", get(index))
        .route("/commit", post(commit))
        .with_state(transport)
}