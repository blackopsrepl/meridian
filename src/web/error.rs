use axum::Json;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use maud::{DOCTYPE, html};
use serde::Serialize;
use thiserror::Error;

use crate::astro::{ChartError, ElectionError, TimingError};
use crate::store::StoreError;

#[derive(Debug, Error)]
pub enum WebError {
    #[error("{0}")]
    BadRequest(String),
    #[error("chart not found")]
    NotFound,
    #[error(transparent)]
    Calculation(#[from] ChartError),
    #[error(transparent)]
    Timing(#[from] TimingError),
    #[error(transparent)]
    Election(#[from] ElectionError),
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error("could not serialize response: {0}")]
    Serialization(#[from] serde_json::Error),
}

impl WebError {
    #[must_use]
    pub const fn status(&self) -> StatusCode {
        match self {
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::Calculation(_) | Self::Timing(_) | Self::Election(_) => {
                StatusCode::UNPROCESSABLE_ENTITY
            }
            Self::Store(_) | Self::Serialization(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    #[must_use]
    pub fn api_response(&self) -> Response {
        #[derive(Serialize)]
        struct ErrorBody<'a> {
            error: &'a str,
            status: u16,
        }
        (
            self.status(),
            Json(ErrorBody {
                error: &self.to_string(),
                status: self.status().as_u16(),
            }),
        )
            .into_response()
    }
}

impl IntoResponse for WebError {
    fn into_response(self) -> Response {
        let status = self.status();
        let markup = html! {
            (DOCTYPE)
            html lang="en" {
                head {
                    meta charset="utf-8";
                    meta name="viewport" content="width=device-width, initial-scale=1";
                    title { (status.as_u16()) " · Meridian" }
                    link rel="stylesheet" href="/assets/app.css";
                }
                body class="error-page" {
                    main class="error-card" {
                        a class="brand-mark standalone" href="/" aria-label="Meridian home" { "M" }
                        p class="eyebrow" { "Calculation interrupted" }
                        h1 { (status.as_u16()) }
                        p class="error-message" { (self.to_string()) }
                        a class="button primary" href="/" { "Return to the observatory" }
                    }
                }
            }
        };
        (status, Html(markup.into_string())).into_response()
    }
}
