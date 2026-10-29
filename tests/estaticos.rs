//! El servidor sirve el cliente compilado (GALATEA_WEB) y responde CORS al origen de la app.
mod comun;

use galatea::Conf;
use reqwest::{Method, header};
use tempfile::TempDir;

#[tokio::test]
async fn la_raiz_es_el_cliente_svelte() {
    let s = comun::Servidor::nuevo().await;
    let r = s.cliente.get(s.url("/")).send().await.unwrap();
    assert_eq!(r.status(), 200);
    assert!(r.text().await.unwrap().contains("cliente de prueba"));
}

#[tokio::test]
async fn sirve_index_y_assets_de_la_carpeta_web() {
    let s = comun::Servidor::nuevo().await;
    let r = s.cliente.get(s.url("/index.html")).send().await.unwrap();
    assert_eq!(r.status(), 200);
    assert!(
        r.headers()[header::CONTENT_TYPE]
            .to_str()
            .unwrap()
            .starts_with("text/html")
    );
    assert!(r.text().await.unwrap().contains("cliente de prueba"));

    let r = s
        .cliente
        .get(s.url("/assets/prueba.js"))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 200);
    let tipo = r.headers()[header::CONTENT_TYPE]
        .to_str()
        .unwrap()
        .to_string();
    assert!(tipo.contains("javascript"), "content-type fue {tipo}");
}

#[tokio::test]
async fn una_ruta_sin_archivo_devuelve_index_para_la_spa() {
    let s = comun::Servidor::nuevo().await;
    let r = s.cliente.get(s.url("/ajustes")).send().await.unwrap();
    assert_eq!(r.status(), 200);
    assert!(r.text().await.unwrap().contains("cliente de prueba"));
}

#[tokio::test]
async fn la_api_sigue_ganando_al_fallback() {
    let s = comun::Servidor::nuevo().await;
    let r = s.cliente.get(s.url("/prendas")).send().await.unwrap();
    assert_eq!(r.status(), 200);
    assert!(
        r.headers()[header::CONTENT_TYPE]
            .to_str()
            .unwrap()
            .starts_with("application/json")
    );
}

#[tokio::test]
async fn preflight_desde_el_origen_de_la_app_se_acepta() {
    let s = comun::Servidor::nuevo().await;
    let r = s
        .cliente
        .request(Method::OPTIONS, s.url("/sesiones"))
        .header(header::ORIGIN, "https://localhost")
        .header(header::ACCESS_CONTROL_REQUEST_METHOD, "POST")
        .header(header::ACCESS_CONTROL_REQUEST_HEADERS, "content-type")
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 200);
    assert_eq!(
        r.headers()[header::ACCESS_CONTROL_ALLOW_ORIGIN],
        "https://localhost"
    );
    let metodos = r.headers()[header::ACCESS_CONTROL_ALLOW_METHODS]
        .to_str()
        .unwrap()
        .to_string();
    for m in ["GET", "HEAD", "POST", "DELETE"] {
        assert!(metodos.contains(m), "faltó {m} en {metodos}");
    }
}

#[tokio::test]
async fn la_miniatura_pedida_desde_la_app_lleva_cors() {
    // En el APK las <img> van con crossorigin="anonymous": la petición lleva Origin
    // (que Caddy deja pasar) y el navegador exige access-control-allow-origin en la respuesta.
    let s = comun::Servidor::nuevo().await;
    for id in ["roja", "azul"] {
        let r = s
            .cliente
            .get(s.url(&format!("/prendas/{id}/miniatura.png")))
            .header(header::ORIGIN, "https://localhost")
            .send()
            .await
            .unwrap();
        assert_eq!(r.status(), 200, "miniatura de {id}");
        assert_eq!(
            r.headers()[header::ACCESS_CONTROL_ALLOW_ORIGIN],
            "https://localhost",
            "miniatura de {id}"
        );
        assert!(
            r.headers()[header::CONTENT_TYPE]
                .to_str()
                .unwrap()
                .starts_with("image/png")
        );
    }
}

#[tokio::test]
async fn un_origen_ajeno_no_recibe_cabeceras_cors() {
    let s = comun::Servidor::nuevo().await;
    let r = s
        .cliente
        .get(s.url("/prendas"))
        .header(header::ORIGIN, "https://otro.example")
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 200);
    assert!(
        r.headers()
            .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
            .is_none()
    );
}

#[tokio::test]
async fn sin_origenes_configurados_no_hay_cors() {
    let temporal = TempDir::new().unwrap();
    let mut conf = Conf::para_pruebas(temporal.path());
    conf.origenes = vec![];
    let s = comun::Servidor::con_conf(conf, temporal).await;
    let r = s
        .cliente
        .get(s.url("/prendas"))
        .header(header::ORIGIN, "https://localhost")
        .send()
        .await
        .unwrap();
    assert!(
        r.headers()
            .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
            .is_none()
    );
}

#[tokio::test]
async fn sin_cliente_compilado_preparar_falla_con_instruccion() {
    let temporal = TempDir::new().unwrap();
    let mut conf = Conf::para_pruebas(temporal.path());
    conf.web = temporal.path().join("no-existe");
    let error = galatea::preparar(&conf)
        .await
        .err()
        .expect("debía fallar")
        .to_string();
    assert!(error.contains("scripts/web.sh"), "mensaje: {error}");
}
