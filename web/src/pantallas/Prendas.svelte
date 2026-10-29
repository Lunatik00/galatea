<script>
  import { onMount } from "svelte";
  import { avisar, elegirPrenda, ocultarMensaje } from "../estado.js";
  import { prendas, url } from "../api.js";
  let lista = $state([]);
  let fallo = $state(false);
  async function cargar() {
    fallo = false;
    try { lista = await prendas(); } catch (e) { fallo = true; avisar(e.message, true, { ajustes: e.clase === "red" }); }
  }
  function reintentar() {
    ocultarMensaje();
    cargar();
  }
  onMount(cargar);
</script>

<h2>Elige una prenda</h2>
{#if fallo}
  <button class="secundario" data-accion="reintentar-prendas" onclick={reintentar}>Reintentar</button>
{/if}
<div class="cuadricula" data-lista-prendas>
  {#each lista as p (p.id)}
    <button data-prenda={p.id} onclick={() => elegirPrenda(p)}>
      <!-- crossorigin: en el APK la imagen viene de otro origen y así lleva Origin, que Caddy deja pasar -->
      <img alt="" crossorigin="anonymous" src={url(p.miniatura)}><span>{p.nombre}</span>
    </button>
  {/each}
</div>
