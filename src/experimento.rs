//! Corre la matriz personas × prendas × flujos y deja una tabla CSV y una hoja HTML para
//! calificar a ojo. Usa los mismos motores que la página, pero llama al motor directamente:
//! las imágenes quedan **sin marca de agua** porque son para evaluar, no para mostrar a
//! clientas.
use crate::{
    Estado, flujos,
    motor::{Avance, Trabajo},
    prendas,
};
use anyhow::{Result, anyhow};
use std::{
    path::{Path, PathBuf},
    time::Instant,
};

pub struct Opciones {
    pub personas: PathBuf,
    pub prendas: PathBuf,
    pub flujos: Vec<String>,
    pub salida: PathBuf,
}

pub struct Fila {
    pub flujo: String,
    pub persona: String,
    pub prenda: String,
    pub segundos: f32,
    pub estado: String,
    pub archivo: Option<PathBuf>,
}

pub struct Resumen {
    pub filas: Vec<Fila>,
}

fn es_imagen(ruta: &Path) -> bool {
    matches!(
        ruta.extension()
            .and_then(|x| x.to_str())
            .map(str::to_ascii_lowercase)
            .as_deref(),
        Some("jpg" | "jpeg" | "png")
    )
}

pub async fn correr(estado: &Estado, opciones: Opciones) -> Result<Resumen> {
    let lista_prendas = prendas::cargar(&opciones.prendas)?;
    let mut personas: Vec<PathBuf> = std::fs::read_dir(&opciones.personas)
        .map_err(|e| {
            anyhow!(
                "no se pudo leer la carpeta de personas {}: {e}; indica una carpeta con fotos .jpg/.png en --personas",
                opciones.personas.display()
            )
        })?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| es_imagen(p))
        .collect();
    personas.sort();
    if personas.is_empty() {
        return Err(anyhow!(
            "no hay imágenes .jpg/.png en {}",
            opciones.personas.display()
        ));
    }
    if opciones.flujos.is_empty() {
        return Err(anyhow!(
            "no se indicó ningún flujo; usa --flujos catvton,fijo"
        ));
    }
    // Todos los flujos se validan antes de correr nada: un nombre mal escrito al final de
    // la lista no debe descubrirse después de una hora de GPU.
    let mut elegidos = Vec::new();
    for nombre in &opciones.flujos {
        let flujo = flujos::buscar(&estado.flujos, nombre).ok_or_else(|| {
            anyhow!(
                "no existe el flujo «{nombre}» en {}/flujos.toml",
                estado.conf.flujos.display()
            )
        })?;
        elegidos.push(flujo);
    }
    let total = elegidos.len() * personas.len() * lista_prendas.len();
    let mut filas = Vec::new();
    for flujo in elegidos {
        for (n, persona) in personas.iter().enumerate() {
            let base_persona = persona
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| format!("persona-{n}"));
            for prenda in &lista_prendas {
                let destino = opciones
                    .salida
                    .join(&flujo.nombre)
                    .join(&base_persona)
                    .join(format!("{}.png", prenda.id));
                let inicio = Instant::now();
                let (estado_fila, archivo) = if !flujo.admite(prenda.tipo) {
                    ("tipo no admitido".to_string(), None)
                } else if let Err(e) = crear_carpeta_de(&destino) {
                    (format!("error: {e}"), None)
                } else {
                    let mut rx = estado.motores.generar(Trabajo {
                        flujo: flujo.clone(),
                        persona: persona.clone(),
                        prenda: prendas::ruta(&opciones.prendas, prenda),
                        prenda_id: prenda.id.clone(),
                        tipo: prenda.tipo,
                        destino: destino.clone(),
                        // Nombra los archivos en ComfyUI y aparece en el log: nunca el nombre
                        // de la foto de una persona.
                        etiqueta: format!("exp{n}"),
                    });
                    let mut ultimo = None;
                    while let Some(a) = rx.recv().await {
                        ultimo = Some(a);
                    }
                    match ultimo {
                        Some(Avance::Listo) => ("ok".to_string(), Some(destino.clone())),
                        Some(Avance::Error(m)) => (format!("error: {m}"), None),
                        _ => ("sin respuesta".to_string(), None),
                    }
                };
                let segundos = inicio.elapsed().as_secs_f32();
                eprintln!(
                    "[{}/{total}] {} · persona {} · {} · {segundos:.1} s · {estado_fila}",
                    filas.len() + 1,
                    flujo.nombre,
                    n + 1,
                    prenda.id
                );
                filas.push(Fila {
                    flujo: flujo.nombre.clone(),
                    persona: base_persona.clone(),
                    prenda: prenda.id.clone(),
                    segundos,
                    estado: estado_fila,
                    archivo,
                });
            }
        }
    }
    std::fs::create_dir_all(&opciones.salida)?;
    std::fs::write(opciones.salida.join("tabla.csv"), tabla_csv(&filas))?;
    std::fs::write(
        opciones.salida.join("hoja.html"),
        hoja_html(&filas, &opciones.salida),
    )?;
    Ok(Resumen { filas })
}

/// Los motores no crean carpetas: el experimento prepara `salida/<flujo>/<persona>/`.
fn crear_carpeta_de(destino: &Path) -> std::io::Result<()> {
    match destino.parent() {
        Some(padre) => std::fs::create_dir_all(padre),
        None => Ok(()),
    }
}

fn tabla_csv(filas: &[Fila]) -> String {
    let mut s = String::from("flujo,persona,prenda,segundos,estado,archivo\n");
    for f in filas {
        s.push_str(&format!(
            "{},{},{},{:.1},\"{}\",{}\n",
            f.flujo,
            f.persona,
            f.prenda,
            f.segundos,
            f.estado.replace('"', "'").replace('\n', " "),
            f.archivo
                .as_ref()
                .map(|a| a.display().to_string())
                .unwrap_or_default()
        ));
    }
    s
}

fn escapar(texto: &str) -> String {
    texto
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn hoja_html(filas: &[Fila], raiz: &Path) -> String {
    let mut s = String::from(
        "<!doctype html><meta charset=utf-8><title>Experimento Galatea</title>\
         <style>body{font-family:system-ui;background:#111;color:#eee}figure{display:inline-block;margin:6px;text-align:center}img{height:360px;display:block}figcaption{font-size:12px}</style>\
         <h1>Experimento</h1>\
         <p>Imágenes <strong>sin marca de agua</strong>: son para evaluar los flujos, no para mostrarlas a clientas.</p>\
         <p>Marca en papel: ¿la mostrarías a una clienta? sí / no</p>",
    );
    for f in filas {
        let ruta = f
            .archivo
            .as_ref()
            .and_then(|a| a.strip_prefix(raiz).ok())
            .map(|a| a.display().to_string());
        let pie = format!(
            "{} · {} · {}",
            escapar(&f.flujo),
            escapar(&f.persona),
            escapar(&f.prenda)
        );
        s.push_str(&match ruta {
            Some(r) => format!(
                "<figure><img src=\"{}\"><figcaption>{pie} · {:.0} s</figcaption></figure>",
                escapar(&r),
                f.segundos
            ),
            None => format!(
                "<figure><div style=\"height:360px;width:240px;background:#333\"></div><figcaption>{pie} · {}</figcaption></figure>",
                escapar(&f.estado)
            ),
        });
    }
    s
}
