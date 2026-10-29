<script>
  import { estado, reintentar, volverAPrendas } from "../estado.js";
</script>

<h2>Generando…</h2>
<progress max="1" value={$estado.progreso.valor} data-progreso></progress>
<p data-mensaje-progreso>{$estado.progreso.texto}</p>
{#if $estado.fallo}
  <button class="secundario" data-accion="reintentar" onclick={reintentar}>Reintentar</button>
  <button class="secundario" data-accion="volver" onclick={volverAPrendas}>Volver a las prendas</button>
{/if}
