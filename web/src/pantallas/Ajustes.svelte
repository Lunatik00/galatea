<script>
  import { cerrarAjustes } from "../estado.js";
  import { guardarServidor, normalizarServidor, porDefecto, servidor, validarServidor } from "../ajustes.js";
  import { salud } from "../api.js";

  let valor = $state(servidor());
  let aviso = $state({ texto: "", error: false });

  async function probar() {
    const invalido = validarServidor(valor);
    if (invalido) { aviso = { texto: invalido, error: true }; return; }
    // Prueba el valor escrito sin tocar el guardado; vacío = el valor por defecto.
    const base = normalizarServidor(valor) || porDefecto();
    try {
      const s = await salud(base);
      if (s?.programa !== "galatea") throw new Error("El servidor respondió algo que no es Galatea");
      aviso = { texto: `Servidor Galatea v${s.version} responde.`, error: false };
    } catch (e) {
      aviso = { texto: e.message, error: true };
    }
  }
  function guardar() {
    const invalido = validarServidor(valor);
    if (invalido) { aviso = { texto: invalido, error: true }; return; }
    guardarServidor(valor);
    valor = servidor();
    aviso = { texto: "Guardado.", error: false };
  }
</script>

<h2>Servidor</h2>
<p>Dirección del servidor Galatea. Vacía = {porDefecto() ? `el valor por defecto, ${porDefecto()}` : "el mismo sitio desde el que se abrió esta página"}.</p>
<input type="url" inputmode="url" placeholder="https://192.168.0.10:8444" bind:value={valor} data-campo="servidor">
{#if aviso.texto}<p class="mensaje" class:error={aviso.error} data-aviso-ajustes>{aviso.texto}</p>{/if}
<div class="acciones">
  <button class="secundario" data-accion="probar" onclick={probar}>Probar</button>
  <button class="principal" data-accion="guardar" onclick={guardar}>Guardar</button>
  <button class="secundario" data-accion="cerrar-ajustes" onclick={cerrarAjustes}>Volver</button>
</div>
