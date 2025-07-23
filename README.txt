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

#dockerコンテナ上ではGUIを確認する方法
①dokcer機動
docker run -it -p 5901:5901 mywasabi:latest bash

②build
rustup target add x86_64-unknown-uefi
cargo build --target x86_64-unknown-uefi
cp target/x86_64-unknown-uefi/debug/wasabi.efi mnt/EFI/BOOT/BOOTX64.EFI

③qemuの仮想マシンをVNCサーバとして起動
qemu-system-x86_64 \
  -bios third_party/ovmf/RELEASEX64_OVMF.fd \
  -drive format=raw,file=fat:rw:mnt \
  -vnc :1

上記の②③をcargo runで実行可能

④別タブからポート5901の確認
sudo netstat -tlnp | grep 5901

⑤別タブから確認
vncviewer localhost:1
