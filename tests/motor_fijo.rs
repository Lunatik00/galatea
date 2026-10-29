use galatea::{
    flujos,
    motor::{Avance, Motores, Trabajo, fijo::Fijo},
    prendas::Tipo,
};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

fn trabajo(flujos: &[flujos::Flujo], prenda_id: &str, destino: PathBuf) -> Trabajo {
    Trabajo {
        flujo: flujos::buscar(flujos, "fijo").unwrap().clone(),
        persona: PathBuf::from("pruebas/imagenes/persona-prueba.jpg"),
        prenda: PathBuf::from("pruebas/prendas/roja.png"),
        prenda_id: prenda_id.to_string(),
        tipo: Tipo::Superior,
        destino,
        etiqueta: "abcdef".to_string(),
    }
}

async fn recoger(mut rx: tokio::sync::mpsc::Receiver<Avance>) -> Vec<Avance> {
    let mut v = Vec::new();
    while let Some(a) = rx.recv().await {
        v.push(a);
    }
    v
}

#[tokio::test]
async fn fijo_copia_la_imagen_de_la_prenda_si_existe() {
    let flujos = flujos::cargar(Path::new("pruebas/flujos")).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let destino = dir.path().join("resultado-1.png");
    let motores = Arc::new(Motores::nuevo(
        Fijo::nuevo(Path::new("pruebas/flujos/fijo")),
        None,
    ));
    let avances = recoger(motores.generar(trabajo(&flujos, "roja", destino.clone()))).await;
    assert_eq!(avances.last(), Some(&Avance::Listo));
    assert!(avances.iter().any(|a| matches!(a, Avance::Progreso(_))));
    assert_eq!(
        std::fs::read(&destino).unwrap(),
        std::fs::read("pruebas/flujos/fijo/roja.png").unwrap()
    );
}

#[tokio::test]
async fn fijo_usa_generico_si_no_hay_imagen_de_la_prenda() {
    let flujos = flujos::cargar(Path::new("pruebas/flujos")).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let destino = dir.path().join("r.png");
    let motores = Arc::new(Motores::nuevo(
        Fijo::nuevo(Path::new("pruebas/flujos/fijo")),
        None,
    ));
    let avances = recoger(motores.generar(trabajo(&flujos, "azul", destino.clone()))).await;
    assert_eq!(avances.last(), Some(&Avance::Listo));
    assert_eq!(
        std::fs::read(&destino).unwrap(),
        std::fs::read("pruebas/flujos/fijo/generico.png").unwrap()
    );
}

#[tokio::test]
async fn sin_carpeta_fijo_el_error_dice_que_falta() {
    let flujos = flujos::cargar(Path::new("pruebas/flujos")).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let motores = Arc::new(Motores::nuevo(
        Fijo::nuevo(&dir.path().join("no-existe")),
        None,
    ));
    let avances =
        recoger(motores.generar(trabajo(&flujos, "roja", dir.path().join("r.png")))).await;
    match avances.last() {
        Some(Avance::Error(m)) => assert!(m.contains("generico.png"), "{m}"),
        otro => panic!("esperaba Error, hubo {otro:?}"),
    }
}

#[tokio::test]
async fn un_flujo_de_comfy_sin_comfy_configurado_da_error_claro() {
    let flujos = flujos::cargar(Path::new("pruebas/flujos")).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let motores = Arc::new(Motores::nuevo(
        Fijo::nuevo(Path::new("pruebas/flujos/fijo")),
        None,
    ));
    let mut t = trabajo(&flujos, "roja", dir.path().join("r.png"));
    t.flujo = flujos::buscar(&flujos, "prueba").unwrap().clone();
    let avances = recoger(motores.generar(t)).await;
    match avances.last() {
        Some(Avance::Error(m)) => assert!(m.contains("GALATEA_COMFY_URL"), "{m}"),
        otro => panic!("esperaba Error, hubo {otro:?}"),
    }
}

#[tokio::test]
async fn dos_trabajos_a_la_vez_el_segundo_espera_su_turno() {
    let flujos = flujos::cargar(Path::new("pruebas/flujos")).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let motores = Arc::new(Motores::nuevo(
        Fijo::nuevo(Path::new("pruebas/flujos/fijo")),
        None,
    ));
    // El primero tarda un segundo para que el segundo lo encuentre ocupando el turno.
    // Con el runtime de un hilo de #[tokio::test], las tareas arrancan en orden de spawn.
    let mut primero = trabajo(&flujos, "roja", dir.path().join("1.png"));
    primero.flujo.espera_segundos = 1;
    let rx1 = motores.generar(primero);
    let rx2 = motores.generar(trabajo(&flujos, "roja", dir.path().join("2.png")));
    let (a1, a2) = tokio::join!(recoger(rx1), recoger(rx2));
    assert_eq!(a1.last(), Some(&Avance::Listo));
    assert_eq!(a2.last(), Some(&Avance::Listo));
    assert_eq!(a1.first(), Some(&Avance::EnCola(0)));
    assert_eq!(
        a2.first(),
        Some(&Avance::EnCola(1)),
        "el segundo vio uno delante: {a2:?}"
    );
}

#[tokio::test]
async fn el_contador_de_cola_no_queda_filtrado_entre_trabajos() {
    let flujos = flujos::cargar(Path::new("pruebas/flujos")).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let motores = Arc::new(Motores::nuevo(
        Fijo::nuevo(Path::new("pruebas/flujos/fijo")),
        None,
    ));
    // Dos trabajos, uno tras otro, cada uno agotado hasta `Listo` antes de lanzar el
    // siguiente: si `esperando` quedara sin decrementar del todo, el tercero vería un
    // número mayor que 0 delante suyo aunque no haya nadie más en la cola.
    recoger(motores.generar(trabajo(&flujos, "roja", dir.path().join("1.png")))).await;
    recoger(motores.generar(trabajo(&flujos, "roja", dir.path().join("2.png")))).await;
    let avances =
        recoger(motores.generar(trabajo(&flujos, "roja", dir.path().join("3.png")))).await;
    assert_eq!(avances.first(), Some(&Avance::EnCola(0)), "{avances:?}");
}
