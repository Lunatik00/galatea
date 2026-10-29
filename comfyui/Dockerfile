# ComfyUI v0.35.0 (la misma versión que empaqueta nixpkgs el 2026-09-26) sobre la
# imagen oficial de PyTorch 2.14 + CUDA 12.6. Todo fijado: la imagen por digest, ComfyUI
# por tag, los nodos por commit, pip por requisitos-nodos.txt.
FROM pytorch/pytorch:2.14.0-cuda12.6-cudnn9-runtime@sha256:a77983eb7a3042ccf41a94da547a7f54a47fb70da15c36d67a8e8a329ebaa098

ARG COMFYUI_TAG=v0.35.0
ARG UID=1000
ARG GID=100

# La imagen instala torch en el Python del sistema (Ubuntu 24.04, PEP 668): pip necesita
# permiso explícito para instalar ahí. Dentro del contenedor no hay otro Python que romper.
ENV PIP_BREAK_SYSTEM_PACKAGES=1
# ~/.cache es de root (el montaje de la caché de Hugging Face la crea): matplotlib, que
# importa ComfyUI-Leffa, guarda su caché en /tmp.
ENV MPLCONFIGDIR=/tmp/matplotlib

RUN apt-get update && apt-get install -y --no-install-recommends git curl libgl1 libglib2.0-0 \
    && rm -rf /var/lib/apt/lists/*

# Ubuntu 24.04 trae un usuario «ubuntu» con uid 1000: se quita para que ese uid sea
# «comfy» con HOME=/home/comfy (ahí se monta la caché de Hugging Face).
RUN userdel -r ubuntu 2>/dev/null || true \
    && (groupadd -g ${GID} -o comfy 2>/dev/null || true) \
    && useradd -m -u ${UID} -g ${GID} -o comfy

WORKDIR /comfy
# restricciones-pip.txt fija cada paquete a la versión probada (ver el archivo).
COPY restricciones-pip.txt /comfy/restricciones-pip.txt
RUN git clone --depth 1 --branch ${COMFYUI_TAG} https://github.com/comfyanonymous/ComfyUI.git app \
    && pip install --no-cache-dir -c /comfy/restricciones-pip.txt -r app/requirements.txt

# Nodos personalizados, cada uno a un commit fijo. Van en /comfy/custom_nodes y no en
# app/custom_nodes porque --base-directory también mueve la carpeta de nodos.
# CatVTON Wrapper: solo se importan py/cat_vton.py y py/func.py, así que DensePose y
# SCHP (detectron2) no se cargan y basta la imagen runtime.
# ComfyUI-Leffa: corre parsing, openpose, DensePose y la difusión de Leffa (MIT) dentro
# del nodo; su detectron2 va por torchvision.ops, sin compilar. Lee models/Leffa.
# ComfyUI-FASHN-VTON: FASHN VTON v1.5 con pesos locales (models/fashn-vton). Su
# requirements.txt no se instala: todo lo que pide ya está salvo fashn-human-parser, que
# va con --no-deps porque pide opencv-python (y la imagen ya trae cv2 por
# opencv-python-headless). onnxruntime-gpu tampoco se instala: ver requisitos-nodos.txt.
COPY requisitos-nodos.txt /comfy/requisitos-nodos.txt
RUN set -eux; mkdir -p custom_nodes; cd custom_nodes; \
    git clone https://github.com/chflame163/ComfyUI_CatVTON_Wrapper.git \
      && git -C ComfyUI_CatVTON_Wrapper checkout e2ae9b62c53a7c2e009f2448975c6909c83f73b5; \
    git clone https://github.com/StartHua/Comfyui_segformer_b2_clothes.git \
      && git -C Comfyui_segformer_b2_clothes checkout 681721fbea6947e7bbc4ebb6192ed60bd8b473cb; \
    git clone https://github.com/Bionic-AI-Solutions/ComfyUI-Leffa.git \
      && git -C ComfyUI-Leffa checkout 90536512f7a9ebcf8f3ff431ee7728073f7e73b8; \
    git clone https://github.com/drphero/ComfyUI-FASHN-VTON.git \
      && git -C ComfyUI-FASHN-VTON checkout 7baa084e0e03862064681f27e2fa76f1e2666fe6; \
    pip install --no-cache-dir -c /comfy/restricciones-pip.txt -r /comfy/requisitos-nodos.txt; \
    pip install --no-cache-dir --no-deps -c /comfy/restricciones-pip.txt fashn-human-parser==0.1.1

# Los modelos, la entrada y la salida viven fuera de la imagen.
RUN mkdir -p /comfy/models /comfy/input /comfy/output /comfy/temp /comfy/user \
    && chown -R ${UID}:${GID} /comfy
USER ${UID}:${GID}
EXPOSE 8188
CMD ["python", "app/main.py", "--listen", "0.0.0.0", "--port", "8188", \
     "--base-directory", "/comfy", "--disable-auto-launch"]
