dokerでubuntuを起動

docker run -it --rm ubuntu:22.04 bash

ビルドツール・依存パッケージのインストール
apt update
apt install -y git build-essential ninja-build meson \
  libglib2.0-dev libpixman-1-dev zlib1g-dev libffi-dev gettext

Python3のインストール
apt install -y python3-venv python3-pip
python3 -m pip install tomli



QEMUのソースコードを取得
git clone https://gitlab.com/qemu-project/qemu.git
cd qemu
mkdir build && cd build

QEMUを、x86_64用の仮想マシンだけビルド対象にして、ビルド設定を準備する
../configure --target-list=x86_64-softmmu


QEMUをビルド
ninja

QEMUのバージョンを確認
./qemu-system-x86_64 --version