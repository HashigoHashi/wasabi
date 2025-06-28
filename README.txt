#dokerでubuntuを起動
docker run -it --rm ubuntu:22.04 bash

#ビルドツール・依存パッケージのインストール
apt update
apt install -y git build-essential ninja-build meson \
  libglib2.0-dev libpixman-1-dev zlib1g-dev libffi-dev gettext

#Python3のインストール
apt install -y python3-venv python3-pip
python3 -m pip install tomli

#Rustをインストール
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
. "$HOME/.cargo/env"

#Rustのバージョンチェック
rustup --version
rustc --version
cargo --version

#QEMUのソースコードを取得
git clone https://gitlab.com/qemu-project/qemu.git
cd qemu
mkdir build && cd build

#QEMUを、x86_64用の仮想マシンだけビルド対象にして、ビルド設定を準備する
../configure --target-list=x86_64-softmmu


#QEMUをビルド
ninja

#QEMUのバージョンを確認
./qemu-system-x86_64 --version

#ビルドに必要なツールをインストール
apt install -y build-essential qemu-system-x86 netcat-openbsd
apt update && apt install -y clang

#バージョンをチェック
make --version
nc
clang --version

#vimをインストールしたほうがやりやすいので
apt update && apt install -y vim


#wasabiをクローンしてくる
cd /root
mkdir repo && cd repo
git clone https://github.com/HashigoHashi/wasabi.git

cd wasabi
ls -sh third_party/ovmf/RELEASEX64_OVMF.fd



#上記の手順をDockerファイルにしたので以下の手順で構築
cd /home/takahashi_daigo/develop/docker_space/wasabi
docker build -t mywasabi:latest .
docker run -it mywasabi:latest
