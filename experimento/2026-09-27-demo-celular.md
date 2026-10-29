# Demo desde el celular: verificación desde el host (2026-09-27)

Este checklist es del cliente web en el navegador; el del APK está en `2026-09-27-apk.md`.

La prueba en un celular físico **todavía no se hizo**: la sesión que armó Caddy no tenía un
celular a mano. Lo que sigue es lo comprobado desde el propio host, contra la IP de la red
local, con el mismo camino que usa el celular (Caddy → Galatea → ComfyUI con CatVTON).
Fotos: las Creative Commons de `pruebas/imagenes/` (ver `LICENCIAS.md`).

## Verificado desde el host

| Qué | Resultado |
|---|---|
| Caddy escucha solo en `192.168.0.10` | TCP 8444 y 8091, UDP 8444 (HTTP/3); `127.0.0.1:8444` no responde |
| `https://192.168.0.10:8444/salud` con la CA de Caddy | JSON de salud, `comfy: ok`, flujos `fijo` y `catvton` sin pesos faltantes |
| Sin la CA | `curl: (60) … unable to get local issuer certificate` (el celular necesita instalarla) |
| `http://192.168.0.10:8091/caddy-root.crt` | 200, `Content-Type: application/x-x509-ca-cert`, idéntico a `datos/caddy/caddy/pki/authorities/local/root.crt` |
| Otra ruta en `:8091` | 302 a `https://192.168.0.10:8444/…` |
| Petición con `Sec-Fetch-Site: cross-site` | 403 |
| SSE a través de Caddy | los eventos `progreso` llegan uno a uno, cada ~0,4 s (sin búfer) |
| Recorrido completo por HTTPS | crear sesión 201 → generar 202 → `listo` → `resultado/1.png` 200 con la marca «imagen generada con IA» → `DELETE` 204 → resultado y foto 404 |
| Segunda generación, misma foto y misma prenda | `resultado/2.png` 200 en 0,5 s: ComfyUI reusa los nodos en caché (semilla fija 42) y solo vuelve a guardar con el prefijo nuevo; la imagen es la misma |

Tiempos con CatVTON en la RTX 3090 (ComfyUI ya caliente, 1280×1735):

| Medición | Segundos |
|---|---|
| Primera generación de la sesión, de «generar» a `listo` | 21,0 |
| Misma foto y prenda otra vez | 0,5 |
| Prueba ignorada `comfy_real` (incluye cargar el modelo en la GPU) | 78 |

## Para hacer en el celular (pendiente)

Con `scripts/demo.sh` corriendo en el host y el celular en la misma Wi-Fi:

1. [ ] Abrir `http://192.168.0.10:8091/caddy-root.crt` e instalar la CA
       (Android: Ajustes → Seguridad → Instalar certificado → CA; iPhone: Perfil descargado →
       Instalar, y después Ajustes → General → Información → Ajustes de confianza de certificados).
2. [ ] Abrir `https://192.168.0.10:8444/`: sin aviso de certificado.
3. [ ] La cámara abre a la primera (anotar si pidió permiso y si hubo que recargar).
4. [ ] Recorrido completo con `?flujos=1` y «Fijo (sin GPU)»: foto, resultado, «Terminar y borrar».
5. [ ] Recorrido completo con CatVTON: elegir prenda, foto, barra de avance, resultado con marca.
6. [ ] Segunda prenda con la misma foto.
7. [ ] «Terminar y borrar» y el mensaje de confirmación.
8. [ ] Si la cámara no abre: «Subir foto» con la cámara nativa.

Anotar:

- Modelo de celular y versión del sistema:
- Navegador y versión:
- ¿La cámara abrió a la primera?:
- Segundos desde «generar» hasta ver el resultado:
- Tropiezos:
