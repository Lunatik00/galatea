use galatea::sesion::{self, Sesiones};
use std::time::{Duration, Instant};

#[tokio::test]
async fn crear_guarda_la_foto_y_terminar_la_borra() {
    let dir = tempfile::tempdir().unwrap();
    let s = Sesiones::nueva(dir.path(), Duration::from_secs(900)).unwrap();
    let id = s.crear(b"no es un jpg pero da igual aqui").await.unwrap();
    assert_eq!(id.len(), 32);
    assert!(s.persona(&id).is_file());
    let (n, ruta) = s.reservar_resultado(&id).await.unwrap();
    assert_eq!(n, 1);
    assert!(ruta.ends_with("resultado-1.png"));
    assert_eq!(s.reservar_resultado(&id).await.unwrap().0, 2);
    assert!(s.terminar(&id).await);
    assert!(!s.persona(&id).exists());
    assert!(s.obtener(&id).await.is_none());
    assert!(!s.terminar(&id).await, "terminar dos veces devuelve false");
}

#[tokio::test]
async fn vencer_borra_solo_las_viejas() {
    let dir = tempfile::tempdir().unwrap();
    let s = Sesiones::nueva(dir.path(), Duration::from_secs(60)).unwrap();
    let vieja = s.crear(b"a").await.unwrap();
    let nueva = s.crear(b"b").await.unwrap();
    // «ahora» es el reloj inyectado: 61 s después de crear ambas no vence nada distinto,
    // así que forzamos la antigüedad de una tocando su `creada`.
    s.envejecer_para_pruebas(&vieja, Duration::from_secs(120))
        .await;
    let vencidas = s.vencer_antiguas(Instant::now()).await;
    assert_eq!(vencidas, 1);
    assert!(s.obtener(&vieja).await.is_none());
    assert!(s.obtener(&nueva).await.is_some());
    assert_eq!(s.cantidad().await, 1);
}

#[test]
fn nueva_vacia_la_carpeta_al_arrancar() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("basura")).unwrap();
    std::fs::write(dir.path().join("basura/persona.jpg"), b"x").unwrap();
    let _s = Sesiones::nueva(dir.path(), Duration::from_secs(1)).unwrap();
    assert!(!dir.path().join("basura").exists());
    assert!(dir.path().is_dir());
}

#[test]
fn abreviar_deja_seis_caracteres() {
    assert_eq!(sesion::abreviar("0123456789abcdef"), "012345");
    assert_eq!(sesion::abreviar("abc"), "abc");
}
