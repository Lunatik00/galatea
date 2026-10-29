//! GET /prendas y sus miniaturas.
use crate::{Estado, prendas, web::error::Fallo};
use axum::{
    Json,
    extract::{Path, State},
    http::header,
    response::{IntoResponse, Response},
};
use serde::Serialize;

#[derive(Serialize)]
pub struct PrendaVista {
    pub id: String,
    pub nombre: String,
    pub tipo: &'static str,
    pub miniatura: String,
}

pub async fn listar(State(estado): State<Estado>) -> Json<Vec<PrendaVista>> {
    Json(
        estado
            .prendas
            .iter()
            .map(|p| PrendaVista {
                id: p.id.clone(),
                nombre: p.nombre.clone(),
                tipo: p.tipo.como_texto(),
                miniatura: format!("/prendas/{}/miniatura.png", p.id),
            })
            .collect(),
    )
}

pub async fn miniatura(State(estado): State<Estado>, Path(id): Path<String>) -> Response {
    if prendas::buscar(&estado.prendas, &id).is_none() {
        return Fallo::no_encontrado("no hay una prenda con ese id");
    }
    match tokio::fs::read(
        estado
            .conf
            .datos
            .join("miniaturas")
            .join(format!("{id}.png")),
    )
    .await
    {
        Ok(bytes) => (
            [
                (header::CONTENT_TYPE, "image/png"),
                (header::CACHE_CONTROL, "public, max-age=3600"),
            ],
            bytes,
        )
            .into_response(),
        Err(e) => {
            tracing::error!("miniatura de «{id}» ilegible: {e}");
            Fallo::interno("la miniatura no está en disco; reinicia el servidor para regenerarlas")
        }
    }
}
