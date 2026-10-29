<script>
  import { onDestroy, onMount } from "svelte";
  import { avisar, descartarFoto, estado, fotoLista, usarFoto } from "../estado.js";
  import { url } from "../api.js";

  let video = $state();
  let camara = null;
  let previa = $state("");

  async function abrirCamara() {
    if (camara) return;
    try {
      camara = await navigator.mediaDevices.getUserMedia({
        video: { facingMode: { ideal: "environment" }, width: { ideal: 1080 }, height: { ideal: 1440 } },
        audio: false,
      });
      video.srcObject = camara;
    } catch (e) {
      const previo = $estado.mensaje?.error ? $estado.mensaje.texto + " " : "";
      avisar(previo + "No se pudo abrir la cámara (" + e.message + "). Usa «Subir foto».", Boolean(previo));
    }
  }
  function cerrarCamara() {
    if (camara) { camara.getTracks().forEach(t => t.stop()); camara = null; }
    if (video) video.srcObject = null;
  }
  function capturar() {
    if (!camara) { avisar("La cámara no está abierta. Usa «Subir foto»."); return; }
    const canvas = document.createElement("canvas");
    canvas.width = video.videoWidth; canvas.height = video.videoHeight;
    canvas.getContext("2d").drawImage(video, 0, 0);
    canvas.toBlob(mostrarPrevia, "image/jpeg", 0.92);
  }
  function archivo(e) {
    const f = e.target.files[0];
    if (f) mostrarPrevia(f);
    e.target.value = "";
  }
  function mostrarPrevia(blob) {
    if (previa) URL.revokeObjectURL(previa);
    previa = URL.createObjectURL(blob);
    fotoLista(blob);
  }
  function repetir() {
    previa = "";
    descartarFoto();
    abrirCamara();
  }
  async function usar() {
    cerrarCamara();
    await usarFoto();
  }

  onMount(abrirCamara);
  onDestroy(() => { cerrarCamara(); if (previa) URL.revokeObjectURL(previa); });
  // Si el store perdió la foto (sesión vencida), la vista previa también desaparece.
  $effect(() => { if (!$estado.foto && previa) { previa = ""; abrirCamara(); } });
</script>

<div class="prenda-elegida">
  <img data-imagen="prenda-elegida" alt="" crossorigin="anonymous" src={url($estado.prenda.miniatura)}><span data-nombre-prenda>{$estado.prenda.nombre}</span>
</div>
<div class="visor">
  <video bind:this={video} autoplay playsinline muted hidden={Boolean(previa)}></video>
  {#if previa}<img data-imagen="previa" alt="" src={previa}>{/if}
  <div class="silueta" aria-hidden="true"></div>
</div>
<ul class="guia"><li>De pie</li><li>Cuerpo entero</li><li>Brazos separados</li></ul>
{#if !previa}
  <div class="acciones">
    <button class="principal" data-accion="capturar" onclick={capturar}>Tomar foto</button>
    <label class="secundario">Subir foto<input type="file" accept="image/*" capture="environment" data-accion="archivo" hidden onchange={archivo}></label>
  </div>
{:else}
  <div class="acciones" data-confirmar>
    <button class="principal" data-accion="usar" onclick={usar}>Usar esta</button>
    <button class="secundario" data-accion="repetir" onclick={repetir}>Repetir</button>
  </div>
{/if}
