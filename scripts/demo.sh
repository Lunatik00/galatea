#!/usr/bin/env bash
# Levanta la demo completa: apaga lo que compite por la GPU, sube ComfyUI y Caddy, arranca
# Galatea en el host y muestra la URL para el celular. Ctrl-C apaga Galatea (los contenedores
# quedan arriba; `docker compose down` los baja).
# Por ahora todos los flujos admiten solo prendas superiores: un vestido o un pantalón de
# prendas/prendas.toml muestra un aviso en la página y no se genera.
set -euo pipefail
cd "$(dirname "$0")/.."
# GALATEA_IP (la IP del host en la red local) vive en .env, fuera de git.
[ -f .env ] && { set -a; . ./.env; set +a; }

echo "== GPU antes:"; nvidia-smi --query-gpu=memory.used,memory.total --format=csv,noheader
# Solo lo que ocupa la GPU; el resto de Aylin (Caddy, Hermes, voz) sigue arriba.
for c in crescent-ollama scout-llamacpp visjar-vllm-1; do
  if docker ps --format '{{.Names}}' | grep -qx "$c"; then
    echo "deteniendo $c (ocupa la GPU)"; docker stop "$c" >/dev/null
  fi
done

: "${GALATEA_IP:?define GALATEA_IP en .env con la IP del host en la red local, por ejemplo GALATEA_IP=192.168.0.10}"

mkdir -p datos/modelos datos/comfy/{input,output,user,temp} datos/hf-cache datos/caddy
docker compose up -d comfyui caddy
echo -n "esperando a ComfyUI"; for _ in $(seq 1 60); do curl -sf http://127.0.0.1:8188/system_stats >/dev/null && break; echo -n .; sleep 2; done; echo
curl -s http://127.0.0.1:8188/system_stats | jq -r '.devices[0] | "GPU: \(.name) · libre \(.vram_free/1e9|floor) GB"'

export GALATEA_COMFY_URL=${GALATEA_COMFY_URL:-http://127.0.0.1:8188}
# FASHN VTON 1.5: sin máscara (conserva brazos y cara, que Leffa y CatVTON redibujan),
# Apache-2.0 salvo el parser humano, ~8 GB de VRAM según FASHN. Sus segundos en la 3090 y
# lo que tarda la primera generación (carga el modelo y el parser) están **por medir**
# (experimento/2026-09-27-nodos.md, «FASHN VTON 1.5»): haz una de ensayo antes de que llegue
# gente (DEMO.md, «Antes de salir de casa»). Con Leffa: GALATEA_FLUJO_POR_DEFECTO=leffa.
export GALATEA_FLUJO_POR_DEFECTO=${GALATEA_FLUJO_POR_DEFECTO:-fashn}
# Con el límite por omisión (120 s) la primera generación en frío de Leffa (~160 s) se
# cortaría siempre; la de FASHN está por medir.
export GALATEA_LIMITE_SEG=${GALATEA_LIMITE_SEG:-240}
export RUST_LOG=${RUST_LOG:-info}
echo
echo "== En el celular (misma red Wi-Fi):"
echo "   1) instala el certificado una vez: http://$GALATEA_IP:8091/caddy-root.crt"
echo "   2) abre https://$GALATEA_IP:8444/   (con ?flujos=1 para elegir motor)"
echo
scripts/web.sh
export GALATEA_WEB=${GALATEA_WEB:-web/dist}
exec nix run . -- servir
