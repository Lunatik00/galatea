//! GET /flujos: qué motores hay y cuáles tienen sus pesos.
use crate::{Estado, flujos};
use axum::{Json, extract::State};
use serde::Serialize;

#[derive(Serialize)]
pub struct FlujoVista {
    pub nombre: String,
    pub titulo: String,
    pub descripcion: String,
    pub tipos: Vec<&'static str>,
    pub disponible: bool,
    pub faltantes: Vec<String>,
}

pub async fn listar(State(estado): State<Estado>) -> Json<Vec<FlujoVista>> {
    Json(
        estado
            .flujos
            .iter()
            .map(|f| {
                let faltantes = flujos::pesos_faltantes(f, &estado.conf.modelos);
                FlujoVista {
                    nombre: f.nombre.clone(),
                    titulo: f.titulo.clone(),
                    descripcion: f.descripcion.clone(),
                    tipos: if f.es_fijo() {
                        vec!["superior", "inferior", "vestido"]
                    } else {
                        f.tipos.iter().map(|t| t.como_texto()).collect()
                    },
                    disponible: faltantes.is_empty(),
                    faltantes,
                }
            })
            .collect(),
    )
}
