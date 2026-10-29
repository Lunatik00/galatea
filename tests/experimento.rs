use galatea::{
    Conf,
    experimento::{self, Opciones},
    preparar,
};

#[tokio::test]
async fn la_matriz_con_fijo_produce_tabla_y_hoja() {
    let temporal = tempfile::tempdir().unwrap();
    let conf = Conf::para_pruebas(temporal.path());
    let estado = preparar(&conf).await.unwrap();
    let personas = temporal.path().join("personas");
    std::fs::create_dir_all(&personas).unwrap();
    std::fs::copy(
        "pruebas/imagenes/persona-prueba.jpg",
        personas.join("a.jpg"),
    )
    .unwrap();
    std::fs::copy(
        "pruebas/imagenes/persona-prueba.jpg",
        personas.join("b.jpg"),
    )
    .unwrap();
    let salida = temporal.path().join("salida");
    let resumen = experimento::correr(
        &estado,
        Opciones {
            personas,
            prendas: "pruebas/prendas".into(),
            flujos: vec!["fijo".to_string()],
            salida: salida.clone(),
        },
    )
    .await
    .unwrap();
    assert_eq!(resumen.filas.len(), 4, "2 personas x 2 prendas x 1 flujo");
    assert!(resumen.filas.iter().all(|f| f.estado == "ok"));
    let csv = std::fs::read_to_string(salida.join("tabla.csv")).unwrap();
    assert!(csv.starts_with("flujo,persona,prenda,segundos,estado,archivo\n"));
    assert_eq!(csv.lines().count(), 5);
    let html = std::fs::read_to_string(salida.join("hoja.html")).unwrap();
    assert!(html.contains("fijo/a/roja.png"));
    assert!(
        !html.contains(&*salida.to_string_lossy()),
        "la hoja usa rutas relativas para poder moverla con sus imágenes"
    );
    assert!(salida.join("fijo/a/roja.png").is_file());
}

#[tokio::test]
async fn un_motor_que_falla_no_corta_la_matriz() {
    let temporal = tempfile::tempdir().unwrap();
    let mut conf = Conf::para_pruebas(temporal.path());
    // Copia de pruebas/flujos donde «prueba» solo admite prendas superiores.
    let carpeta_flujos = temporal.path().join("flujos");
    std::fs::create_dir_all(carpeta_flujos.join("fijo")).unwrap();
    for archivo in ["prueba.json", "fijo/generico.png", "fijo/roja.png"] {
        std::fs::copy(
            std::path::Path::new("pruebas/flujos").join(archivo),
            carpeta_flujos.join(archivo),
        )
        .unwrap();
    }
    let toml = std::fs::read_to_string("pruebas/flujos/flujos.toml")
        .unwrap()
        .replace(
            r#"tipos = ["superior", "inferior", "vestido"]"#,
            r#"tipos = ["superior"]"#,
        );
    std::fs::write(carpeta_flujos.join("flujos.toml"), toml).unwrap();
    conf.flujos = carpeta_flujos;
    let estado = preparar(&conf).await.unwrap();
    let personas = temporal.path().join("personas");
    std::fs::create_dir_all(&personas).unwrap();
    std::fs::copy(
        "pruebas/imagenes/persona-prueba.jpg",
        personas.join("a.jpg"),
    )
    .unwrap();
    std::fs::write(personas.join("notas.txt"), "no es una foto").unwrap();
    let salida = temporal.path().join("salida");
    // «prueba» necesita ComfyUI y para_pruebas no lo configura: la polera roja falla en el
    // motor y el vestido azul ni se intenta.
    let resumen = experimento::correr(
        &estado,
        Opciones {
            personas,
            prendas: "pruebas/prendas".into(),
            flujos: vec!["prueba".to_string(), "fijo".to_string()],
            salida: salida.clone(),
        },
    )
    .await
    .unwrap();
    assert_eq!(resumen.filas.len(), 4, "1 persona x 2 prendas x 2 flujos");
    let (fallidas, buenas): (Vec<_>, Vec<_>) =
        resumen.filas.iter().partition(|f| f.flujo == "prueba");
    let roja = fallidas.iter().find(|f| f.prenda == "roja").unwrap();
    assert!(
        roja.estado.starts_with("error: ") && roja.archivo.is_none(),
        "{}",
        roja.estado
    );
    let azul = fallidas.iter().find(|f| f.prenda == "azul").unwrap();
    assert_eq!(azul.estado, "tipo no admitido");
    assert!(buenas.iter().all(|f| f.estado == "ok"));
    let csv = std::fs::read_to_string(salida.join("tabla.csv")).unwrap();
    assert_eq!(csv.lines().count(), 5);
}

#[tokio::test]
async fn sin_carpeta_de_personas_o_con_flujo_desconocido_falla_claro() {
    let temporal = tempfile::tempdir().unwrap();
    let conf = Conf::para_pruebas(temporal.path());
    let estado = preparar(&conf).await.unwrap();
    let opciones = |personas: std::path::PathBuf, flujo: &str| Opciones {
        personas,
        prendas: "pruebas/prendas".into(),
        flujos: vec![flujo.to_string()],
        salida: temporal.path().join("salida"),
    };
    let error = experimento::correr(&estado, opciones(temporal.path().join("no-existe"), "fijo"))
        .await
        .err()
        .unwrap()
        .to_string();
    assert!(error.contains("carpeta de personas"), "{error}");
    let error = experimento::correr(&estado, opciones("pruebas/imagenes".into(), "inexistente"))
        .await
        .err()
        .unwrap()
        .to_string();
    assert!(error.contains("«inexistente»"), "{error}");
    assert!(!temporal.path().join("salida").exists(), "no corrió nada");
}
