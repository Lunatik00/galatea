<script>
  import { onMount } from "svelte";
  import { elegirFlujo, estado, otraPrenda, repetirFoto, terminar } from "../estado.js";
  import { flujos, personaUrl, resultadoUrl } from "../api.js";

  let verOriginal = $state(false);
  let lista = $state([]);
  // Derivados primitivos: las URL solo cambian cuando cambia la sesión o el resultado.
  const id = $derived($estado.sesion.id);
  const numero = $derived($estado.sesion.resultado);
  const resultado = $derived(resultadoUrl(id, numero));
  const original = $derived(personaUrl(id));

  // «Descargar» usa un blob: en el APK la imagen es de otro origen y `download` no vale ahí.
  // Un blob es del mismo origen. Mientras llega (o si falla), el enlace va a la URL directa.
  let descarga = $state("");
  $effect(() => {
    const direccion = resultado;
    let vivo = true;
    let blob = "";
    fetch(direccion)
      .then(r => { if (!r.ok) throw new Error(`error ${r.status}`); return r.blob(); })
      .then(b => { if (vivo) { blob = URL.createObjectURL(b); descarga = blob; } })
      .catch(() => {});
    return () => {
      vivo = false;
      if (blob) URL.revokeObjectURL(blob);
      descarga = "";
    };
  });

  onMount(async () => {
    if ($estado.conFlujos) { try { lista = await flujos(); } catch { lista = []; } }
  });
</script>

<div class="comparar">
  <img data-imagen="resultado" alt="Persona con la prenda puesta (imagen generada con IA)" crossorigin="anonymous" src={resultado} hidden={verOriginal}>
  <img data-imagen="original" alt="Foto original" crossorigin="anonymous" src={original} hidden={!verOriginal}>
</div>
<button class="secundario" data-accion="comparar" onclick={() => (verOriginal = !verOriginal)}>
  {verOriginal ? "Ver resultado" : "Ver original"}
</button>
{#if $estado.conFlujos}
  <label data-selector-flujos>Motor
    <select data-flujos value={$estado.flujo} onchange={e => elegirFlujo(e.target.value)}>
      {#each lista as f (f.nombre)}
        <option value={f.nombre} disabled={!f.disponible}>{f.disponible ? f.titulo : `${f.titulo} (faltan pesos)`}</option>
      {/each}
    </select>
  </label>
{/if}
<div class="acciones">
  <button class="principal" data-accion="otra-prenda" onclick={otraPrenda}>Otra prenda</button>
  <button class="secundario" data-accion="repetir-foto" onclick={repetirFoto}>Repetir foto</button>
  <a class="secundario" data-accion="descargar" download="galatea.png" href={descarga || resultado}>Descargar</a>
  <button class="peligro" data-accion="terminar" onclick={terminar}>Terminar y borrar</button>
</div>
