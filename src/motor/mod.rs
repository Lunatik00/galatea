//! Los motores de generación. `Motores` decide según el flujo: `fijo` va al motor Fijo,
//! cualquier otro a ComfyUI. Un solo trabajo corre a la vez (la GPU no se reparte).
pub mod comfy;
pub mod fijo;

use crate::{flujos::Flujo, prendas::Tipo, sesion};
use std::{
    path::PathBuf,
    sync::{Arc, atomic::AtomicUsize},
};
use tokio::sync::{Semaphore, mpsc};

/// Decrementa `esperando` al soltarse, sin importar si el bloque que lo sostiene termina
/// normalmente o por un pánico: así el contador nunca queda en falso por una tarea que
/// entra en pánico a mitad de un trabajo.
struct GuardaTurno<'a> {
    esperando: &'a AtomicUsize,
}

impl Drop for GuardaTurno<'_> {
    fn drop(&mut self) {
        self.esperando
            .fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Avance {
    /// Cuántos trabajos hay delante (0 = el siguiente en entrar).
    EnCola(usize),
    /// 0.0 a 1.0.
    Progreso(f32),
    /// `trabajo.destino` ya está escrito.
    Listo,
    Error(String),
}

#[derive(Clone, Debug)]
pub struct Trabajo {
    pub flujo: Flujo,
    pub persona: PathBuf,
    pub prenda: PathBuf,
    pub prenda_id: String,
    pub tipo: Tipo,
    pub destino: PathBuf,
    /// Id de sesión abreviado: nombra archivos en ComfyUI y aparece en el log.
    pub etiqueta: String,
}

pub struct Motores {
    pub fijo: fijo::Fijo,
    pub comfy: Option<comfy::Comfy>,
    turno: Arc<Semaphore>,
    esperando: Arc<std::sync::atomic::AtomicUsize>,
}

impl Motores {
    pub fn nuevo(fijo: fijo::Fijo, comfy: Option<comfy::Comfy>) -> Self {
        Self {
            fijo,
            comfy,
            turno: Arc::new(Semaphore::new(1)),
            esperando: Arc::default(),
        }
    }

    /// Encola el trabajo y devuelve el canal por el que llegan los avances.
    /// El último mensaje es siempre `Listo` o `Error`, y después el canal se cierra.
    pub fn generar(self: &Arc<Self>, trabajo: Trabajo) -> mpsc::Receiver<Avance> {
        let (tx, rx) = mpsc::channel(32);
        let motores = Arc::clone(self);
        tokio::spawn(async move {
            use std::sync::atomic::Ordering;
            let delante = motores.esperando.fetch_add(1, Ordering::SeqCst);
            // Vive hasta el final del bloque, incluso si algo entra en pánico dentro:
            // se decrementa recién cuando el trabajo termina, no al tomar el turno, porque
            // mientras corre sigue ocupando el lugar que los que llegan después esperan.
            let _guarda = GuardaTurno {
                esperando: &motores.esperando,
            };
            let _ = tx.send(Avance::EnCola(delante)).await;
            let _permiso = motores
                .turno
                .acquire()
                .await
                .expect("el semáforo no se cierra");
            let resultado = if trabajo.flujo.es_fijo() {
                motores.fijo.generar(&trabajo, &tx).await
            } else {
                match &motores.comfy {
                    Some(c) => c.generar(&trabajo, &tx).await,
                    None => Err("El motor de imágenes no está configurado: define GALATEA_COMFY_URL (por ejemplo http://127.0.0.1:8188) o usa el flujo Fijo.".to_string()),
                }
            };
            let ultimo = match resultado {
                Ok(()) => Avance::Listo,
                Err(mensaje) => {
                    tracing::warn!(
                        "trabajo {} falló: {}",
                        trabajo.etiqueta,
                        sesion::redactar(&mensaje)
                    );
                    Avance::Error(mensaje)
                }
            };
            let _ = tx.send(ultimo).await;
        });
        rx
    }

    /// ¿Responde ComfyUI? El Fijo siempre está disponible.
    pub async fn disponible(&self) -> Result<(), String> {
        match &self.comfy {
            Some(c) => c.disponible().await,
            None => Err("sin GALATEA_COMFY_URL: solo el flujo Fijo".to_string()),
        }
    }
}

#[cfg(test)]
mod pruebas {
    use super::GuardaTurno;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn la_guarda_decrementa_aunque_el_cuerpo_entre_en_panico() {
        let contador = AtomicUsize::new(1);
        let resultado = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guarda = GuardaTurno {
                esperando: &contador,
            };
            panic!("simulado para probar la guarda");
        }));
        assert!(resultado.is_err());
        assert_eq!(contador.load(Ordering::SeqCst), 0);
    }
}
