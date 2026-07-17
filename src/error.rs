use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

pub type AppResult<T> = Result<T, AppError>;

#[derive(thiserror::Error, Debug)]
pub enum AppError {
    #[error("未找到资源")]
    NotFound,
    #[error("{0}")]
    BadRequest(String),
    #[error("未认证，请先登录")]
    Unauthorized,
    #[error("权限不足")]
    Forbidden,
    #[error("{0}")]
    Conflict(String),
    #[error("文件过大（上限 {limit} 字节）")]
    PayloadTooLarge { limit: usize },
    #[error("不支持的图片格式")]
    UnsupportedFormat,
    #[error("存储空间不足（配额已满）")]
    QuotaExceeded,
    #[error(transparent)]
    Sql(#[from] sqlx::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl AppError {
    fn status(&self) -> StatusCode {
        match self {
            AppError::NotFound => StatusCode::NOT_FOUND,
            AppError::BadRequest(_) => StatusCode::BAD_REQUEST,
            AppError::Unauthorized => StatusCode::UNAUTHORIZED,
            AppError::Forbidden => StatusCode::FORBIDDEN,
            AppError::Conflict(_) => StatusCode::CONFLICT,
            AppError::PayloadTooLarge { .. } => StatusCode::PAYLOAD_TOO_LARGE,
            AppError::UnsupportedFormat => StatusCode::UNSUPPORTED_MEDIA_TYPE,
            AppError::QuotaExceeded => StatusCode::INSUFFICIENT_STORAGE,
            AppError::Sql(sqlx::Error::RowNotFound) => StatusCode::NOT_FOUND,
            AppError::Sql(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::Io(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::Other(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// Whether this error should be logged at warn (unexpected) vs traced at debug (expected).
    pub fn is_expected(&self) -> bool {
        matches!(
            self,
            AppError::NotFound
                | AppError::BadRequest(_)
                | AppError::Unauthorized
                | AppError::Forbidden
                | AppError::Conflict(_)
                | AppError::PayloadTooLarge { .. }
                | AppError::UnsupportedFormat
                | AppError::QuotaExceeded
                | AppError::Sql(sqlx::Error::RowNotFound)
        )
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.status();
        if !self.is_expected() {
            tracing::error!(error = %self.as_ref(), "request failed");
        }
        let body = Json(json!({ "error": self.to_string(), "code": status.as_u16() }));
        (status, body).into_response()
    }
}

// Treat &AppError as displayable in tracing.
impl AsRef<dyn std::error::Error> for AppError {
    fn as_ref(&self) -> &(dyn std::error::Error + 'static) {
        self
    }
}
