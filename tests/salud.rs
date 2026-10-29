mod comun;

#[tokio::test]
async fn salud_responde_con_la_version() {
    let s = comun::Servidor::nuevo().await;
    let r = s.cliente.get(s.url("/salud")).send().await.unwrap();
    assert_eq!(r.status(), 200);
    let cuerpo: serde_json::Value = r.json().await.unwrap();
    assert_eq!(cuerpo["programa"], "galatea");
}

// La ruta genérica que no coincide con ninguna ya no da 404 en JSON: desde la Tarea 2 el
// fallback sirve el cliente (index.html) para que la SPA maneje sus propias rutas. Ver
// `una_ruta_sin_archivo_devuelve_index_para_la_spa` en tests/estaticos.rs. Un 404 real
// sigue existiendo cuando una ruta SÍ coincide pero el recurso no está (por ejemplo
// `/prendas/nada/miniatura.png`, cubierto en tests/web.rs).
