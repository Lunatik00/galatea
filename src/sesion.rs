//! Sesiones de la demo: una foto, sus resultados, y su borrado. Único módulo que crea y
//! borra carpetas bajo `datos/sesiones/`.
use std::{
    collections::HashMap,
    io,
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::RwLock;

pub const NOMBRE_PERSONA: &str = "persona.jpg";

#[derive(Clone, Debug)]
pub struct Sesion {
    pub id: String,
    pub creada: Instant,
    pub carpeta: PathBuf,
    pub resultados: u32,
}

#[derive(Clone)]
pub struct Sesiones {
    interior: Arc<RwLock<HashMap<String, Sesion>>>,
    raiz: PathBuf,
    vencimiento: Duration,
}

/// Los primeros seis caracteres: lo único de un id que puede ir al log.
pub fn abreviar(id: &str) -> &str {
    &id[..id.len().min(6)]
}

/// Reemplaza cada segmento de 32 o más dígitos hexadecimales (el largo de un id de
/// sesión) por sus primeros 6 caracteres y «…»; el resto del texto queda igual. Así un
/// mensaje de error que arrastra una ruta con un id completo puede ir a un log.
pub fn redactar(texto: &str) -> String {
    texto
        .split('/')
        .map(|trozo| {
            if trozo.len() >= 32 && trozo.chars().all(|c| c.is_ascii_hexdigit()) {
                format!("{}…", &trozo[..6])
            } else {
                trozo.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("/")
}

impl Sesiones {
    /// Vacía `raiz` por completo: una sesión de una ejecución anterior ya venció por definición.
    pub fn nueva(raiz: &Path, vencimiento: Duration) -> io::Result<Self> {
        if raiz.exists() {
            std::fs::remove_dir_all(raiz)?;
        }
        std::fs::create_dir_all(raiz)?;
        Ok(Self {
            interior: Arc::new(RwLock::new(HashMap::new())),
            raiz: raiz.to_path_buf(),
            vencimiento,
        })
    }

    pub fn generar_id() -> String {
        let mut bytes = [0u8; 16];
        getrandom::fill(&mut bytes).expect("el sistema debería poder dar bytes aleatorios");
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }

    pub async fn crear(&self, foto: &[u8]) -> io::Result<String> {
        let id = Self::generar_id();
        let carpeta = self.raiz.join(&id);
        tokio::fs::create_dir_all(&carpeta).await?;
        tokio::fs::write(carpeta.join(NOMBRE_PERSONA), foto).await?;
        self.interior.write().await.insert(
            id.clone(),
            Sesion {
                id: id.clone(),
                creada: Instant::now(),
                carpeta,
                resultados: 0,
            },
        );
        tracing::info!("sesión {} creada", abreviar(&id));
        Ok(id)
    }

    pub async fn obtener(&self, id: &str) -> Option<Sesion> {
        self.interior.read().await.get(id).cloned()
    }

    pub fn persona(&self, id: &str) -> PathBuf {
        self.raiz.join(id).join(NOMBRE_PERSONA)
    }

    /// Reserva el siguiente número de resultado y devuelve su ruta (aún no escrita).
    pub async fn reservar_resultado(&self, id: &str) -> Option<(u32, PathBuf)> {
        let mut mapa = self.interior.write().await;
        let s = mapa.get_mut(id)?;
        s.resultados += 1;
        Some((
            s.resultados,
            s.carpeta.join(format!("resultado-{}.png", s.resultados)),
        ))
    }

    pub async fn terminar(&self, id: &str) -> bool {
        let quitada = self.interior.write().await.remove(id);
        match quitada {
            Some(s) => {
                if let Err(e) = tokio::fs::remove_dir_all(&s.carpeta).await {
                    tracing::error!(
                        "no se pudo borrar la carpeta de la sesión {}: {e}",
                        abreviar(id)
                    );
                }
                tracing::info!("sesión {} terminada y borrada", abreviar(id));
                true
            }
            None => false,
        }
    }

    /// Si la sesión ya no está en el mapa pero algo dejó su carpeta en disco (un trabajo
    /// que terminó justo cuando se borraba), la borra. Devuelve true si borró algo.
    pub async fn borrar_restos(&self, id: &str) -> bool {
        if self.interior.read().await.contains_key(id) {
            return false;
        }
        let carpeta = self.raiz.join(id);
        if !carpeta.exists() {
            return false;
        }
        match tokio::fs::remove_dir_all(&carpeta).await {
            Ok(()) => {
                tracing::info!("restos de la sesión {} borrados", abreviar(id));
                true
            }
            Err(e) => {
                tracing::error!(
                    "no se pudieron borrar los restos de la sesión {}: {e}",
                    abreviar(id)
                );
                false
            }
        }
    }

    /// Borra las sesiones más viejas que `vencimiento` respecto de `ahora`. Devuelve cuántas.
    pub async fn vencer_antiguas(&self, ahora: Instant) -> usize {
        let vencidas: Vec<String> = self
            .interior
            .read()
            .await
            .values()
            .filter(|s| ahora.duration_since(s.creada) >= self.vencimiento)
            .map(|s| s.id.clone())
            .collect();
        let mut n = 0;
        for id in vencidas {
            if self.terminar(&id).await {
                n += 1;
            }
        }
        n
    }

    pub async fn cantidad(&self) -> usize {
        self.interior.read().await.len()
    }

    /// Solo pruebas: retrasa `creada` para simular el paso del tiempo sin dormir.
    #[cfg(any(test, feature = "pruebas"))]
    pub async fn envejecer_para_pruebas(&self, id: &str, cuanto: Duration) {
        if let Some(s) = self.interior.write().await.get_mut(id) {
            s.creada = Instant::now()
                .checked_sub(cuanto)
                .expect("Instant no debería desbordar");
        }
    }
}

#[cfg(test)]
mod pruebas {
    use super::redactar;

    #[test]
    fn redactar_recorta_solo_los_segmentos_largos() {
        let id = "0123456789abcdef0123456789abcdef";
        assert_eq!(
            redactar(&format!(
                "no se pudo copiar a datos/sesiones/{id}/resultado-1.png: motivo"
            )),
            "no se pudo copiar a datos/sesiones/012345…/resultado-1.png: motivo"
        );
        assert_eq!(redactar("sesión abc123 creada"), "sesión abc123 creada");
    }
}
