use galatea::{
    flujos::{self, Encuadre, Entradas, ErrorFlujos},
    prendas::Tipo,
};
use std::path::Path;

#[test]
fn carga_los_flujos_de_prueba_con_su_grafo() {
    let flujos = flujos::cargar(Path::new("pruebas/flujos")).unwrap();
    let nombres: Vec<_> = flujos.iter().map(|f| f.nombre.as_str()).collect();
    assert_eq!(nombres, ["fijo", "prueba"]);
    let prueba = flujos::buscar(&flujos, "prueba").unwrap();
    assert!(prueba.grafo.is_some());
    assert!(prueba.admite(Tipo::Vestido));
    assert!(flujos::buscar(&flujos, "fijo").unwrap().es_fijo());
    assert!(
        flujos::buscar(&flujos, "fijo")
            .unwrap()
            .admite(Tipo::Inferior)
    );
}

#[test]
fn agrega_fijo_si_el_toml_no_lo_declara() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::copy("pruebas/flujos/prueba.json", dir.path().join("prueba.json")).unwrap();
    std::fs::write(
        dir.path().join("flujos.toml"),
        r#"
[[flujo]]
nombre = "prueba"
titulo = "P"
archivo = "prueba.json"
nodo_persona = "1"
nodo_prenda = "2"
nodo_salida = "4"
tipos = ["superior"]
"#,
    )
    .unwrap();
    let flujos = flujos::cargar(dir.path()).unwrap();
    assert!(flujos.iter().any(|f| f.es_fijo()));
}

#[test]
fn rellenar_cambia_solo_los_nodos_declarados() {
    let flujos = flujos::cargar(Path::new("pruebas/flujos")).unwrap();
    let prueba = flujos::buscar(&flujos, "prueba").unwrap();
    let lleno = flujos::rellenar(
        prueba,
        &Entradas {
            persona: "p.jpg",
            prenda: "r.png",
            tipo: Tipo::Inferior,
        },
    )
    .unwrap();
    assert_eq!(lleno["1"]["inputs"]["image"], "p.jpg");
    assert_eq!(lleno["2"]["inputs"]["image"], "r.png");
    assert_eq!(lleno["3"]["inputs"]["cloth_type"], "lower");
    assert_eq!(lleno["3"]["inputs"]["steps"], 2, "lo demás no cambia");
    assert_eq!(lleno["4"]["inputs"]["filename_prefix"], "galatea");
    // el original queda intacto
    assert_eq!(
        prueba.grafo.as_ref().unwrap()["1"]["inputs"]["image"],
        "PERSONA.jpg"
    );
}

#[test]
fn rechaza_nodos_que_no_existen_en_el_grafo() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::copy("pruebas/flujos/prueba.json", dir.path().join("prueba.json")).unwrap();
    std::fs::write(
        dir.path().join("flujos.toml"),
        r#"
[[flujo]]
nombre = "roto"
titulo = "R"
archivo = "prueba.json"
nodo_persona = "99"
nodo_prenda = "2"
nodo_salida = "4"
tipos = ["superior"]
"#,
    )
    .unwrap();
    let e = flujos::cargar(dir.path()).unwrap_err();
    assert!(
        matches!(e, ErrorFlujos::NodoInexistente { ref nodo, .. } if nodo == "99"),
        "{e}"
    );
}

#[test]
fn pesos_faltantes_lista_los_que_no_estan() {
    let flujos = flujos::cargar(Path::new("pruebas/flujos")).unwrap();
    let prueba = flujos::buscar(&flujos, "prueba").unwrap();
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(
        flujos::pesos_faltantes(prueba, dir.path()),
        vec!["prueba/peso.safetensors".to_string()]
    );
    std::fs::create_dir_all(dir.path().join("prueba")).unwrap();
    std::fs::write(dir.path().join("prueba/peso.safetensors"), b"x").unwrap();
    assert!(flujos::pesos_faltantes(prueba, dir.path()).is_empty());
    assert!(
        flujos::pesos_faltantes(flujos::buscar(&flujos, "fijo").unwrap(), dir.path()).is_empty()
    );
}

#[test]
fn carga_los_flujos_reales_de_la_demo() {
    let flujos = flujos::cargar(Path::new("flujos")).unwrap();
    let catvton = flujos::buscar(&flujos, "catvton").unwrap();
    assert!(catvton.grafo.is_some());
    assert!(flujos::buscar(&flujos, "fijo").is_some());
    let lleno = flujos::rellenar(
        catvton,
        &Entradas {
            persona: "p.jpg",
            prenda: "r.jpg",
            tipo: Tipo::Superior,
        },
    )
    .unwrap();
    assert_eq!(lleno["1"]["inputs"]["image"], "p.jpg");
    assert_eq!(lleno["2"]["inputs"]["image"], "r.jpg");
}

#[test]
fn fashn_escribe_la_categoria_del_nodo_para_cada_tipo() {
    let flujos = flujos::cargar(Path::new("flujos")).unwrap();
    let fashn = flujos::buscar(&flujos, "fashn").unwrap();
    for (tipo, categoria) in [
        (Tipo::Superior, "tops"),
        (Tipo::Inferior, "bottoms"),
        (Tipo::Vestido, "one-pieces"),
    ] {
        assert!(fashn.admite(tipo));
        let lleno = flujos::rellenar(
            fashn,
            &Entradas {
                persona: "p.jpg",
                prenda: "r.jpg",
                tipo,
            },
        )
        .unwrap();
        assert_eq!(lleno["1"]["inputs"]["image"], "p.jpg");
        assert_eq!(lleno["2"]["inputs"]["image"], "r.jpg");
        assert_eq!(lleno["4"]["class_type"], "FashnVtonInference");
        assert_eq!(lleno["4"]["inputs"]["category"], categoria);
        assert_eq!(lleno["5"]["inputs"]["filename_prefix"], "galatea");
    }
}

#[test]
fn rechaza_nodos_con_inputs_invalidos() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("malo.json"),
        r#"{
            "1": { "class_type": "LoadImage", "inputs": "no-es-un-objeto" },
            "2": { "class_type": "LoadImage", "inputs": { "image": "PRENDA.jpg" } },
            "4": { "class_type": "SaveImage", "inputs": { "images": ["2", 0] } }
        }"#,
    )
    .unwrap();
    std::fs::write(
        dir.path().join("flujos.toml"),
        r#"
[[flujo]]
nombre = "malo"
titulo = "M"
archivo = "malo.json"
nodo_persona = "1"
nodo_prenda = "2"
nodo_salida = "4"
tipos = ["superior"]
"#,
    )
    .unwrap();
    let e = flujos::cargar(dir.path()).unwrap_err();
    assert!(
        matches!(e, ErrorFlujos::EntradasInvalidas { ref nodo, .. } if nodo == "1"),
        "{e}"
    );
}

#[test]
fn rellenar_sin_nodo_tipo_no_toca_el_campo_de_tipo() {
    let flujos = flujos::cargar(Path::new("pruebas/flujos")).unwrap();
    let mut prueba = flujos::buscar(&flujos, "prueba").unwrap().clone();
    prueba.nodo_tipo = None;
    let lleno = flujos::rellenar(
        &prueba,
        &Entradas {
            persona: "p.jpg",
            prenda: "r.png",
            tipo: Tipo::Vestido,
        },
    )
    .unwrap();
    assert_eq!(lleno["1"]["inputs"]["image"], "p.jpg");
    assert_eq!(lleno["2"]["inputs"]["image"], "r.png");
    // sin nodo_tipo declarado, el campo de tipo del grafo original no se toca
    assert_eq!(lleno["3"]["inputs"]["cloth_type"], "upper");
}

#[test]
fn rechaza_tipos_sin_valor_correspondiente() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::copy("pruebas/flujos/prueba.json", dir.path().join("prueba.json")).unwrap();
    std::fs::write(
        dir.path().join("flujos.toml"),
        r#"
[[flujo]]
nombre = "roto"
titulo = "R"
archivo = "prueba.json"
nodo_persona = "1"
nodo_prenda = "2"
nodo_tipo = "3"
nodo_salida = "4"
tipos = ["vestido"]
valores_tipo = { superior = "upper" }
"#,
    )
    .unwrap();
    let e = flujos::cargar(dir.path()).unwrap_err();
    assert!(
        matches!(e, ErrorFlujos::SinValorTipo { tipo, .. } if tipo == "vestido"),
        "{e}"
    );
}

#[test]
fn lee_el_encuadre_y_sin_el_campo_no_hay_encuadre() {
    let flujos = flujos::cargar(Path::new("pruebas/flujos")).unwrap();
    assert_eq!(
        flujos::buscar(&flujos, "prueba").unwrap().encuadre,
        Some(Encuadre::TresPorCuatro)
    );
    assert_eq!(flujos::buscar(&flujos, "fijo").unwrap().encuadre, None);
    // Los flujos reales: solo leffa encuadra (su nodo estira un marco 768×1024 rellenado).
    let reales = flujos::cargar(Path::new("flujos")).unwrap();
    for (nombre, esperado) in [
        ("leffa", Some(Encuadre::TresPorCuatro)),
        ("klein", None),
        ("catvton", None),
    ] {
        assert_eq!(
            flujos::buscar(&reales, nombre).unwrap().encuadre,
            esperado,
            "{nombre}"
        );
    }
}

#[test]
fn un_encuadre_desconocido_se_rechaza_al_cargar() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::copy("pruebas/flujos/prueba.json", dir.path().join("prueba.json")).unwrap();
    std::fs::write(
        dir.path().join("flujos.toml"),
        r#"
[[flujo]]
nombre = "prueba"
titulo = "P"
archivo = "prueba.json"
nodo_persona = "1"
nodo_prenda = "2"
nodo_salida = "4"
encuadre = "4:3"
tipos = ["superior"]
"#,
    )
    .unwrap();
    match flujos::cargar(dir.path()) {
        Err(e @ ErrorFlujos::EncuadreDesconocido { .. }) => {
            let m = e.to_string();
            assert!(
                m.contains("prueba") && m.contains("4:3") && m.contains("\"3:4\""),
                "{m}"
            );
        }
        otro => panic!("{:?}", otro.map(|f| f.len())),
    }
}
