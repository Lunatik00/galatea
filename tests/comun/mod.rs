// tests/comun/mod.rs
//! Levanta el router real en un puerto efímero con una carpeta temporal.
#![allow(dead_code)]

pub mod comfy_falso;
use galatea::{Conf, construir_router, preparar};
use tempfile::TempDir;

pub struct Servidor {
    pub direccion: String,
    pub cliente: reqwest::Client,
    pub conf: Conf,
    _temporal: TempDir,
}

impl Servidor {
    pub async fn nuevo() -> Self {
        let temporal = TempDir::new().expect("no se pudo crear el directorio temporal");
        let conf = Conf::para_pruebas(temporal.path());
        Self::con_conf(conf, temporal).await
    }

    pub async fn con_conf(conf: Conf, temporal: TempDir) -> Self {
        let escucha = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("sin puerto efímero");
        let puerto = escucha.local_addr().unwrap().port();
        let estado = preparar(&conf)
            .await
            .expect("preparar no debería fallar en pruebas");
        let router = construir_router(estado);
        tokio::spawn(async move {
            axum::serve(escucha, router).await.ok();
        });
        Self {
            direccion: format!("http://127.0.0.1:{puerto}"),
            cliente: reqwest::Client::new(),
            conf,
            _temporal: temporal,
        }
    }

    pub fn url(&self, ruta: &str) -> String {
        format!("{}{}", self.direccion, ruta)
    }
}
