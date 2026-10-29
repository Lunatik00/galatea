//! Errores HTTP en JSON, siempre con un mensaje en español que dice qué pasó y qué hacer.
use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Fallo {
    #[serde(skip)]
    pub estado: StatusCode,
    pub error: String,
}

impl Fallo {
    pub fn no_encontrado(mensaje: &str) -> Response {
        Self {
            estado: StatusCode::NOT_FOUND,
            error: mensaje.to_string(),
        }
        .into_response()
    }
    pub fn invalido(mensaje: impl Into<String>) -> Response {
        Self {
            estado: StatusCode::UNPROCESSABLE_ENTITY,
            error: mensaje.into(),
        }
        .into_response()
    }
    pub fn interno(mensaje: impl Into<String>) -> Response {
        Self {
            estado: StatusCode::INTERNAL_SERVER_ERROR,
            error: mensaje.into(),
        }
        .into_response()
    }
    pub fn no_disponible(mensaje: impl Into<String>) -> Response {
        Self {
            estado: StatusCode::SERVICE_UNAVAILABLE,
            error: mensaje.into(),
        }
        .into_response()
    }
}

impl IntoResponse for Fallo {
    fn into_response(self) -> Response {
        let estado = self.estado;
        (estado, Json(self)).into_response()
    }
}
