//! Devuelve `carpeta/<prenda_id>.png` o `carpeta/generico.png` tras una espera simulada.
use super::{Avance, Trabajo};
use std::{
    path::{Path, PathBuf},
    time::Duration,
};
use tokio::sync::mpsc;

#[derive(Clone, Debug)]
pub struct Fijo {
    pub carpeta: PathBuf,
}

impl Fijo {
    pub fn nuevo(carpeta: &Path) -> Self {
        Self {
            carpeta: carpeta.to_path_buf(),
        }
    }

    pub async fn generar(
        &self,
        trabajo: &Trabajo,
        tx: &mpsc::Sender<Avance>,
    ) -> Result<(), String> {
        let propia = self.carpeta.join(format!("{}.png", trabajo.prenda_id));
        let generico = self.carpeta.join("generico.png");
        let origen = if propia.is_file() {
            propia
        } else if generico.is_file() {
            generico
        } else {
            return Err(format!(
                "el flujo Fijo no encontró {} ni {}; agrega al menos generico.png en esa carpeta",
                self.carpeta
                    .join(format!("{}.png", trabajo.prenda_id))
                    .display(),
                self.carpeta.join("generico.png").display()
            ));
        };
        let pasos = 10u32;
        let paso = Duration::from_millis(trabajo.flujo.espera_segundos * 1000 / u64::from(pasos));
        for i in 1..=pasos {
            tokio::time::sleep(paso).await;
            let _ = tx.send(Avance::Progreso(i as f32 / pasos as f32)).await;
        }
        // La carpeta de `destino` la crea quien encarga el trabajo (la sesión, o el
        // experimento): si ya no existe, la copia falla y el trabajo termina en error.
        tokio::fs::copy(&origen, &trabajo.destino)
            .await
            .map_err(|e| {
                format!(
                    "no se pudo copiar la imagen fija a {}: {e}",
                    trabajo.destino.display()
                )
            })?;
        Ok(())
    }
}
