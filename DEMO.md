# La demo de Galatea

## Antes de salir de casa
1. `scripts/demo.sh` en una terminal; espera «GPU: … libre N GB» y la URL.
2. En el celular, misma Wi-Fi: instala el certificado desde `http://<GALATEA_IP>:8091/caddy-root.crt`
   (Android: Ajustes → Seguridad → Instalar certificado → CA; iPhone: Perfil descargado → Instalar,
   y luego Ajustes → General → Información → Ajustes de confianza de certificados).
   El APK (`adb install -r datos/apk/galatea-debug.apk`, sin certificado) ya muestra prendas y resultados en el
   emulador, pero «Descargar» no guarda nada y falta probarlo en un teléfono
   (ver `experimento/2026-09-27-apk.md`). Hasta esa prueba, la demo va por el navegador.
3. Abre `https://<GALATEA_IP>:8444/` y haz un recorrido completo con el flujo Fijo (`?flujos=1`)
   para comprobar cámara y borrado. Después vuelve al flujo por defecto.
4. Ensayo de FASHN VTON 1.5 (el flujo por defecto): la primera generación carga el modelo y
   baja el parser humano, y cuánto tarda está **por medir** (`scripts/demo.sh` deja el límite
   en 240 s; con Leffa eran ~160 s). En el navegador del host abre `http://127.0.0.1:8766/`, elige una
   prenda superior, sube con «Subir foto» la foto CC `pruebas/imagenes/persona-cc.jpg` y **espera
   a que salga el resultado**; las siguientes, en caliente, también están por medir (Leffa: ~17 s). Pulsa «Terminar» al acabar. Reiniciar
   Galatea no descarga los modelos (viven en el contenedor de ComfyUI).
5. Prendas: copia las fotos de la tienda a `prendas/` y complétalas en `prendas/prendas.toml`;
   reinicia Galatea para regenerar miniaturas. FASHN admite prendas superiores, pantalones y
   vestidos; los otros flujos solo superiores (con los demás muestran un aviso y no generan).

## Los ocho minutos
1. (1 min) El problema: «¿cómo me quedaría a mí?» y por qué las fotos en maniquí no responden.
2. (1 min) El aviso de privacidad, leído en voz alta: se borra al terminar.
3. (3 min) Elegir prenda, tomar la foto a la voluntaria, esperar mirando la barra, ver el resultado
   y comparar con la original. Probar una segunda prenda con la misma foto.
4. (1 min) «Terminar y borrar» delante de la voluntaria; mostrar el mensaje de confirmación.
5. (2 min) Qué viene: tallas con medidas reales, modelos genéricas sin foto, correr fuera de casa.

## Si algo falla
- ComfyUI no responde → en el celular, `?flujos=1` y elegir «Fijo (sin GPU)»: la demo sigue con
  imágenes pregeneradas. En la terminal, `docker compose logs --tail 50 comfyui`.
- La cámara no abre → «Subir foto» usa la cámara nativa del celular.
- Certificado rechazado → revisar que la hora del celular esté bien y que el certificado
  instalado sea el de `datos/caddy/caddy/pki/authorities/local/root.crt` actual (si se borró `datos/caddy`,
  hay que reinstalarlo).
- La app dice “No encuentro el servidor” → Ajustes en la app, comprobar la dirección; si la CA
  cambió, regenerar el APK con `scripts/apk.sh`.

## Al terminar
- `datos/comfy/input/galatea-prenda-*.jpg` son las prendas del catálogo subidas a ComfyUI, no datos
  personales; si cambia el catálogo, se borran con `rm -f datos/comfy/input/galatea-prenda-*`.
