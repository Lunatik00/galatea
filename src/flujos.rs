//! Los motores como archivos: `flujos.toml` describe cada flujo de ComfyUI (formato API)
//! y qué nodos se rellenan. Rust no sabe qué modelo hay detrás.
use crate::prendas::Tipo;
use serde::Deserialize;
use serde_json::Value;
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};

pub const NOMBRE_FIJO: &str = "fijo";

#[derive(Clone, Debug, Deserialize)]
pub struct Peso {
    pub url: String,
    pub destino: String,
    #[serde(default)]
    pub sha256: String,
}

fn campo_imagen() -> String {
    "image".to_string()
}
fn campo_tipo() -> String {
    "cloth_type".to_string()
}
fn espera_por_omision() -> u64 {
    8
}
fn valores_tipo_por_omision() -> HashMap<Tipo, String> {
    HashMap::from([
        (Tipo::Superior, "upper".to_string()),
        (Tipo::Inferior, "lower".to_string()),
        (Tipo::Vestido, "overall".to_string()),
    ])
}

/// Cómo prepara el servidor la foto de la persona antes de mandarla al flujo.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Encuadre {
    /// `encuadre = "3:4"`: relleno blanco hasta 3:4 y recorte del resultado al tamaño original
    /// (ver `crate::encuadre`). Para los nodos que trabajan en un marco fijo de 768×1024.
    TresPorCuatro,
}

impl Encuadre {
    const VALORES: &'static str = "\"3:4\"";

    fn desde_texto(texto: &str) -> Option<Self> {
        match texto {
            "3:4" => Some(Self::TresPorCuatro),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct Flujo {
    pub nombre: String,
    pub titulo: String,
    #[serde(default)]
    pub descripcion: String,
    pub archivo: Option<String>,
    pub nodo_persona: Option<String>,
    pub nodo_prenda: Option<String>,
    pub nodo_tipo: Option<String>,
    pub nodo_salida: Option<String>,
    #[serde(default = "campo_imagen")]
    pub campo_persona: String,
    #[serde(default = "campo_imagen")]
    pub campo_prenda: String,
    #[serde(default = "campo_tipo")]
    pub campo_tipo: String,
    #[serde(default = "valores_tipo_por_omision")]
    pub valores_tipo: HashMap<Tipo, String>,
    #[serde(default)]
    pub tipos: Vec<Tipo>,
    #[serde(default = "espera_por_omision")]
    pub espera_segundos: u64,
    #[serde(default, rename = "peso")]
    pub pesos: Vec<Peso>,
    /// El `encuadre` tal como viene en el TOML; `cargar` lo valida y llena `encuadre`.
    #[serde(default, rename = "encuadre")]
    encuadre_texto: Option<String>,
    /// `None` (sin `encuadre` en el TOML): la foto va tal cual y el resultado no se recorta.
    #[serde(skip)]
    pub encuadre: Option<Encuadre>,
    /// El grafo en formato API, cargado desde `archivo`. `None` en el flujo fijo.
    #[serde(skip)]
    pub grafo: Option<Value>,
}

impl Flujo {
    pub fn es_fijo(&self) -> bool {
        self.nombre == NOMBRE_FIJO
    }

    /// El fijo admite cualquier tipo: devuelve una imagen pregenerada.
    pub fn admite(&self, tipo: Tipo) -> bool {
        self.es_fijo() || self.tipos.contains(&tipo)
    }

    fn fijo_por_omision() -> Flujo {
        Flujo {
            nombre: NOMBRE_FIJO.to_string(),
            titulo: "Fijo (sin GPU)".to_string(),
            descripcion: "Devuelve una imagen pregenerada de fijo/.".to_string(),
            archivo: None,
            nodo_persona: None,
            nodo_prenda: None,
            nodo_tipo: None,
            nodo_salida: None,
            campo_persona: campo_imagen(),
            campo_prenda: campo_imagen(),
            campo_tipo: campo_tipo(),
            valores_tipo: valores_tipo_por_omision(),
            tipos: Vec::new(),
            espera_segundos: espera_por_omision(),
            pesos: Vec::new(),
            encuadre_texto: None,
            encuadre: None,
            grafo: None,
        }
    }
}

#[derive(Deserialize)]
struct Archivo {
    #[serde(default)]
    flujo: Vec<Flujo>,
}

pub struct Entradas<'a> {
    pub persona: &'a str,
    pub prenda: &'a str,
    pub tipo: Tipo,
}

#[derive(Debug, thiserror::Error)]
pub enum ErrorFlujos {
    #[error("no se pudo leer {ruta}: {fuente}")]
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
    #[error("{ruta} no es un flujo válido en formato API de ComfyUI: {fuente}")]
    Json {
        ruta: PathBuf,
        #[source]
        fuente: serde_json::Error,
    },
    #[error("el flujo «{0}» aparece dos veces en flujos.toml")]
    NombreRepetido(String),
    #[error(
        "el flujo «{0}» no tiene archivo; todo flujo salvo «fijo» necesita archivo = \"<nombre>.json\""
    )]
    SinArchivo(String),
    #[error("el flujo «{flujo}» no declara {campo}; sin él no se sabe qué nodo rellenar")]
    SinNodo { flujo: String, campo: &'static str },
    #[error(
        "el flujo «{flujo}» apunta al nodo «{nodo}», que no existe en su JSON; exporta el flujo otra vez y copia el id correcto"
    )]
    NodoInexistente { flujo: String, nodo: String },
    #[error(
        "el nodo «{nodo}» del flujo «{flujo}» no tiene un objeto inputs; el JSON debe estar en formato API (Save (API format) en ComfyUI)"
    )]
    EntradasInvalidas { flujo: String, nodo: String },
    #[error(
        "el flujo «{flujo}» admite el tipo «{tipo}» pero no tiene un valor para él en valores_tipo; agrégalo o quítalo de tipos"
    )]
    SinValorTipo { flujo: String, tipo: &'static str },
    #[error(
        "el flujo «{flujo}» pide encuadre = \"{valor}\", que Galatea no conoce; los valores posibles son {valores}, o quita el campo para mandar la foto tal cual"
    )]
    EncuadreDesconocido {
        flujo: String,
        valor: String,
        valores: &'static str,
    },
}

pub fn cargar(carpeta: &Path) -> Result<Vec<Flujo>, ErrorFlujos> {
    let ruta = carpeta.join("flujos.toml");
    let texto = fs::read_to_string(&ruta).map_err(|fuente| ErrorFlujos::Lectura {
        ruta: ruta.clone(),
        fuente,
    })?;
    let archivo: Archivo = toml::from_str(&texto).map_err(|fuente| ErrorFlujos::Formato {
        ruta: ruta.clone(),
        fuente,
    })?;
    let mut flujos = archivo.flujo;
    if !flujos.iter().any(Flujo::es_fijo) {
        flujos.insert(0, Flujo::fijo_por_omision());
    }
    let mut vistos = HashSet::new();
    for f in flujos.iter_mut() {
        if !vistos.insert(f.nombre.clone()) {
            return Err(ErrorFlujos::NombreRepetido(f.nombre.clone()));
        }
        if let Some(texto) = &f.encuadre_texto {
            f.encuadre = Some(Encuadre::desde_texto(texto).ok_or_else(|| {
                ErrorFlujos::EncuadreDesconocido {
                    flujo: f.nombre.clone(),
                    valor: texto.clone(),
                    valores: Encuadre::VALORES,
                }
            })?);
        }
        if f.es_fijo() {
            continue;
        }
        let nombre_json = f
            .archivo
            .clone()
            .ok_or_else(|| ErrorFlujos::SinArchivo(f.nombre.clone()))?;
        let ruta_json = carpeta.join(nombre_json);
        let texto = fs::read_to_string(&ruta_json).map_err(|fuente| ErrorFlujos::Lectura {
            ruta: ruta_json.clone(),
            fuente,
        })?;
        let grafo: Value = serde_json::from_str(&texto).map_err(|fuente| ErrorFlujos::Json {
            ruta: ruta_json.clone(),
            fuente,
        })?;
        for (campo, nodo) in [
            ("nodo_persona", &f.nodo_persona),
            ("nodo_prenda", &f.nodo_prenda),
            ("nodo_salida", &f.nodo_salida),
        ] {
            let nodo = nodo.as_ref().ok_or(ErrorFlujos::SinNodo {
                flujo: f.nombre.clone(),
                campo,
            })?;
            comprobar_nodo(&grafo, &f.nombre, nodo)?;
        }
        if let Some(nodo) = &f.nodo_tipo {
            comprobar_nodo(&grafo, &f.nombre, nodo)?;
            for tipo in &f.tipos {
                if !f.valores_tipo.contains_key(tipo) {
                    return Err(ErrorFlujos::SinValorTipo {
                        flujo: f.nombre.clone(),
                        tipo: tipo.como_texto(),
                    });
                }
            }
        }
        f.grafo = Some(grafo);
    }
    Ok(flujos)
}

fn comprobar_nodo(grafo: &Value, flujo: &str, nodo: &str) -> Result<(), ErrorFlujos> {
    let Some(n) = grafo.get(nodo) else {
        return Err(ErrorFlujos::NodoInexistente {
            flujo: flujo.to_string(),
            nodo: nodo.to_string(),
        });
    };
    if n.get("inputs").and_then(Value::as_object).is_some() {
        Ok(())
    } else {
        Err(ErrorFlujos::EntradasInvalidas {
            flujo: flujo.to_string(),
            nodo: nodo.to_string(),
        })
    }
}

pub fn buscar<'a>(flujos: &'a [Flujo], nombre: &str) -> Option<&'a Flujo> {
    flujos.iter().find(|f| f.nombre == nombre)
}

/// Copia el grafo y escribe los nombres de archivo y el tipo en los nodos declarados.
pub fn rellenar(flujo: &Flujo, entradas: &Entradas) -> Result<Value, ErrorFlujos> {
    let mut grafo = flujo
        .grafo
        .clone()
        .ok_or_else(|| ErrorFlujos::SinArchivo(flujo.nombre.clone()))?;
    poner(
        &mut grafo,
        flujo,
        flujo.nodo_persona.as_deref(),
        "nodo_persona",
        &flujo.campo_persona,
        Value::from(entradas.persona),
    )?;
    poner(
        &mut grafo,
        flujo,
        flujo.nodo_prenda.as_deref(),
        "nodo_prenda",
        &flujo.campo_prenda,
        Value::from(entradas.prenda),
    )?;
    if let Some(nodo) = flujo.nodo_tipo.as_deref() {
        let valor = flujo
            .valores_tipo
            .get(&entradas.tipo)
            .ok_or(ErrorFlujos::SinValorTipo {
                flujo: flujo.nombre.clone(),
                tipo: entradas.tipo.como_texto(),
            })?;
        poner(
            &mut grafo,
            flujo,
            Some(nodo),
            "nodo_tipo",
            &flujo.campo_tipo,
            Value::from(valor.as_str()),
        )?;
    }
    Ok(grafo)
}

fn poner(
    grafo: &mut Value,
    flujo: &Flujo,
    nodo: Option<&str>,
    campo: &'static str,
    clave: &str,
    valor: Value,
) -> Result<(), ErrorFlujos> {
    let nodo = nodo.ok_or(ErrorFlujos::SinNodo {
        flujo: flujo.nombre.clone(),
        campo,
    })?;
    let Some(n) = grafo.get_mut(nodo) else {
        return Err(ErrorFlujos::NodoInexistente {
            flujo: flujo.nombre.clone(),
            nodo: nodo.to_string(),
        });
    };
    let entradas = n
        .get_mut("inputs")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| ErrorFlujos::EntradasInvalidas {
            flujo: flujo.nombre.clone(),
            nodo: nodo.to_string(),
        })?;
    entradas.insert(clave.to_string(), valor);
    Ok(())
}

/// Los `destino` de `[[flujo.peso]]` que no están bajo `modelos/`.
pub fn pesos_faltantes(flujo: &Flujo, modelos: &Path) -> Vec<String> {
    flujo
        .pesos
        .iter()
        .filter(|p| !modelos.join(&p.destino).is_file())
        .map(|p| p.destino.clone())
        .collect()
}
