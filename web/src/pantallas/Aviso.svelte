<script>
  import { abrirAjustes, entendido } from "../estado.js";
  import { esNativo } from "../nativo.js";
</script>

<h2>Antes de la foto</h2>
<p>La foto sirve solo para mostrar cómo queda la prenda. Se borra al pulsar «Terminar y borrar»{esNativo() ? "" : ", al cerrar la página"} o a los quince minutos, y no se usa para entrenar nada. No se fotografía a menores.</p>
<button class="principal" data-accion="entendido" onclick={entendido}>Entendido</button>
<button class="enlace" data-accion="abrir-ajustes" onclick={abrirAjustes}>Ajustes del servidor</button>
