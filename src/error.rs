use axum::{http::StatusCode, response::IntoResponse, Json};
use sea_orm::DbErr;
use serde::Serialize;
use strum::IntoStaticStr;

#[derive(IntoStaticStr)]
pub enum APIError {
    Internal,
    NotFound,
    FolderExist,
}

impl APIError {
    fn into_kiosk_version_error<T: Serialize>(
        &self,
        status_code: StatusCode,
        data: Option<T>,
    ) -> axum::response::Response {
        (
            status_code,
            Json(ReturnedResponse {
                kiosk_version_error: ReturnedKioskVersionError {
                    code: self.into(),
                    data: data,
                },
            }),
        )
            .into_response()
    }
}

impl IntoResponse for APIError {
    fn into_response(self) -> axum::response::Response {
        match self {
            APIError::Internal => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
            APIError::NotFound => StatusCode::NOT_FOUND.into_response(),
            APIError::FolderExist => {
                self.into_kiosk_version_error::<()>(StatusCode::UNPROCESSABLE_ENTITY, None)
            }
        }
    }
}

impl From<DbErr> for APIError {
    fn from(_value: DbErr) -> Self {
        APIError::Internal
    }
}

impl From<std::io::Error> for APIError {
    fn from(_: std::io::Error) -> Self {
        APIError::Internal
    }
}

impl From<serde_json::Error> for APIError {
    fn from(e: serde_json::Error) -> Self {
        APIError::Internal
    }
}

impl From<Box<dyn std::error::Error>> for APIError {
    fn from(e: Box<dyn std::error::Error>) -> Self {
        APIError::Internal
    }
}

#[derive(Serialize)]
struct ReturnedResponse<T: Serialize> {
    kiosk_version_error: ReturnedKioskVersionError<T>,
}

#[derive(Serialize)]
struct ReturnedKioskVersionError<T: Serialize> {
    code: &'static str,
    data: Option<T>,
}
