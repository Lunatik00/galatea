<script>
  import { onMount } from "svelte";
  import { abrirAjustes, activarFlujos, alCerrarPagina, estado } from "./estado.js";
  import { salud } from "./api.js";
  import Aviso from "./pantallas/Aviso.svelte";
  import Prendas from "./pantallas/Prendas.svelte";
  import Camara from "./pantallas/Camara.svelte";
  import Generando from "./pantallas/Generando.svelte";
  import Resultado from "./pantallas/Resultado.svelte";
  import Ajustes from "./pantallas/Ajustes.svelte";

  const pantallas = { aviso: Aviso, prendas: Prendas, camara: Camara, generando: Generando, resultado: Resultado, ajustes: Ajustes };
  let version = $state("");
  const Pantalla = $derived(pantallas[$estado.pantalla]);

  onMount(() => {
    if (new URLSearchParams(location.search).get("flujos") === "1") activarFlujos();
    salud().then(s => { version = s.version; }).catch(() => {});
    window.addEventListener("pagehide", alCerrarPagina);
    return () => window.removeEventListener("pagehide", alCerrarPagina);
  });
</script>

<header>
  <h1>Galatea</h1>
  {#if version}<span class="version">v{version}</span>{/if}
</header>
<main>
  <section data-estado={$estado.pantalla}>
    <Pantalla />
  </section>
  {#if $estado.mensaje}
    <p class="mensaje" class:error={$estado.mensaje.error} data-mensaje>
      {$estado.mensaje.texto}
      {#if $estado.mensaje.ajustes && $estado.pantalla !== "ajustes"}
        <button class="secundario" data-accion="ir-ajustes" onclick={abrirAjustes}>Ajustes</button>
      {/if}
    </p>
  {/if}
</main>
