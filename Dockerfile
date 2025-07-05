# ベースイメージはUbuntu 22.04
FROM ubuntu:22.04

# 必要なパッケージのインストール（まとめてapt updateも実行）
RUN apt-get update && apt-get install -y \
  git build-essential ninja-build meson \
  libglib2.0-dev libpixman-1-dev zlib1g-dev libffi-dev gettext \
  python3-venv python3-pip clang vim netcat-openbsd qemu-system-x86 \
  && rm -rf /var/lib/apt/lists/*

# Python tomliをpipでインストール
RUN python3 -m pip install tomli

# Rustのインストール
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

# Rustの環境変数を設定（後続のRUNやコンテナ起動時に反映させるため）
ENV PATH="/root/.cargo/bin:${PATH}"

# QEMUソースのclone & ビルド
RUN git clone https://gitlab.com/qemu-project/qemu.git /root/qemu \
  && cd /root/qemu \
  && mkdir build \
  && cd build \
  && ../configure --target-list=x86_64-softmmu \
  && ninja

# wasabiリポジトリのclone
RUN mkdir -p /root/repo \
  && git clone https://github.com/HashigoHashi/wasabi.git /root/repo/wasabi

WORKDIR /root/repo/wasabi

# コンテナ起動時にbashを立ち上げる
CMD ["bash"]