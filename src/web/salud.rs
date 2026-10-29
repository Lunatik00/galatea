//! GET /salud: versión, ComfyUI, pesos por flujo, sesiones vivas.
use crate::{Estado, flujos};
use axum::{Json, extract::State};
use serde_json::{Value, json};

pub async fn ver(State(estado): State<Estado>) -> Json<Value> {
    let comfy = match estado.motores.disponible().await {
        Ok(()) => "ok".to_string(),
        Err(e) => e,
    };
    let lista: Vec<Value> = estado.flujos.iter().map(|f| json!({ "nombre": f.nombre, "faltantes": flujos::pesos_faltantes(f, &estado.conf.modelos) })).collect();
    Json(json!({
        "programa": "galatea",
        "version": env!("CARGO_PKG_VERSION"),
        "comfy": comfy,
        "flujos": lista,
        "sesiones": estado.sesiones.cantidad().await,
    }))
}
