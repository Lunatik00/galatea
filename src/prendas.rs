//! Las prendas precargadas: `prendas.toml` más sus imágenes. Solo lectura y validación.
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

pub const LADO_MINIATURA: u32 = 256;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Tipo {
    Superior,
    Inferior,
    Vestido,
}

impl Tipo {
    pub fn como_texto(self) -> &'static str {
        match self {
            Tipo::Superior => "superior",
            Tipo::Inferior => "inferior",
            Tipo::Vestido => "vestido",
        }
    }

    pub fn desde_texto(texto: &str) -> Option<Tipo> {
        match texto {
            "superior" => Some(Tipo::Superior),
            "inferior" => Some(Tipo::Inferior),
            "vestido" => Some(Tipo::Vestido),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Prenda {
    pub id: String,
    pub nombre: String,
    pub archivo: String,
    pub tipo: Tipo,
}

#[derive(Deserialize)]
struct Archivo {
    #[serde(default)]
    prenda: Vec<Prenda>,
}

#[derive(Debug, thiserror::Error)]
pub enum ErrorPrendas {
    #[error("no se pudo leer {ruta}: {fuente}. Crea el archivo con al menos una [[prenda]].")]
    Lectura {
        ruta: PathBuf,
        #[source]
        fuente: std::io::Error,
    },
    #[error("{ruta} tiene un error de formato: {fuente}")]
    Formato {
        ruta: PathBuf,
        #[source]
        fuente: toml::de::Error,
    },
    #[error("{ruta} no tiene ninguna prenda; agrega al menos una entrada [[prenda]]")]
    Vacio { ruta: PathBuf },
    #[error("la prenda «{0}» aparece dos veces en prendas.toml; los id deben ser únicos")]
    IdRepetido(String),
    #[error("el id «{0}» solo puede llevar letras minúsculas, números y guiones")]
    IdInvalido(String),
    #[error("la prenda «{id}» apunta a {ruta}, que no existe; revisa el campo archivo")]
    ArchivoFaltante { id: String, ruta: PathBuf },
    #[error(
        "no se pudo crear la carpeta de miniaturas {ruta}: {fuente}; revisa permisos de GALATEA_DATOS"
    )]
    Carpeta {
        ruta: PathBuf,
        #[source]
        fuente: std::io::Error,
    },
    #[error("no se pudo abrir la imagen de la prenda «{id}»: {fuente}")]
    Imagen {
        id: String,
        #[source]
        fuente: image::ImageError,
    },
    #[error("no se pudo escribir la miniatura de «{id}» en {ruta}: {fuente}")]
    Miniatura {
        id: String,
        ruta: PathBuf,
        #[source]
        fuente: std::io::Error,
    },
}

pub fn cargar(carpeta: &Path) -> Result<Vec<Prenda>, ErrorPrendas> {
    let ruta = carpeta.join("prendas.toml");
    let texto = fs::read_to_string(&ruta).map_err(|fuente| ErrorPrendas::Lectura {
        ruta: ruta.clone(),
        fuente,
    })?;
    let archivo: Archivo = toml::from_str(&texto).map_err(|fuente| ErrorPrendas::Formato {
        ruta: ruta.clone(),
        fuente,
    })?;
    if archivo.prenda.is_empty() {
        return Err(ErrorPrendas::Vacio { ruta });
    }
    let mut vistos = HashSet::new();
    for p in &archivo.prenda {
        if !id_valido(&p.id) {
            return Err(ErrorPrendas::IdInvalido(p.id.clone()));
        }
        if !vistos.insert(p.id.clone()) {
            return Err(ErrorPrendas::IdRepetido(p.id.clone()));
        }
        let imagen = self::ruta(carpeta, p);
        if !imagen.is_file() {
            return Err(ErrorPrendas::ArchivoFaltante {
                id: p.id.clone(),
                ruta: imagen,
            });
        }
    }
    Ok(archivo.prenda)
}

fn id_valido(id: &str) -> bool {
    !id.is_empty()
        && id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

pub fn ruta(carpeta: &Path, prenda: &Prenda) -> PathBuf {
    carpeta.join(&prenda.archivo)
}

pub fn buscar<'a>(prendas: &'a [Prenda], id: &str) -> Option<&'a Prenda> {
    prendas.iter().find(|p| p.id == id)
}

/// Escribe `destino/<id>.png` con el lado mayor en `LADO_MINIATURA`.
pub fn generar_miniaturas(
    carpeta: &Path,
    prendas: &[Prenda],
    destino: &Path,
) -> Result<(), ErrorPrendas> {
    fs::create_dir_all(destino).map_err(|fuente| ErrorPrendas::Carpeta {
        ruta: destino.to_path_buf(),
        fuente,
    })?;
    for p in prendas {
        let imagen = image::open(ruta(carpeta, p)).map_err(|fuente| ErrorPrendas::Imagen {
            id: p.id.clone(),
            fuente,
        })?;
        let miniatura = imagen.thumbnail(LADO_MINIATURA, LADO_MINIATURA);
        let salida = destino.join(format!("{}.png", p.id));
        miniatura
            .save(&salida)
            .map_err(|e| ErrorPrendas::Miniatura {
                id: p.id.clone(),
                ruta: salida.clone(),
                fuente: std::io::Error::other(e),
            })?;
    }
    Ok(())
}
