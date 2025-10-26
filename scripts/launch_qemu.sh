#!/bin/bash -e
SCRIPT_PATH="${BASH_SOURCE[0]:-$0}"
PROJ_ROOT="$(cd "$(dirname "$(dirname "$SCRIPT_PATH")")" && pwd)"
cd "${PROJ_ROOT}"

PATH_TO_EFI="$1"
rm -rf mnt
mkdir -p mnt/EFI/BOOT/
cp ${PATH_TO_EFI} mnt/EFI/BOOT/BOOTX64.EFI
set +e
mkdir -p log
qemu-system-x86_64 \
  -m 4G \
  -bios third_party/ovmf/RELEASEX64_OVMF.fd \
  -drive format=raw,file=fat:rw:mnt \
  -chardev stdio,id=char_com1,mux=on,logfile=log/com1.txt \
  -serial chardev:char_com1 \
  -device isa-debug-exit,iobase=0xf4,iosize=0x01 #docker上で起動する場合vncを使用するのでここを-vnc :1に変更する
RETCODE=$?
set -e
if [ $RETCODE -eq 0 ]; then
  exit 0
elif [ $RETCODE -eq 3 ]; then
  printf "\nPASS\n"
  exit 0
else
  printf "\nFAIL: QEMU returned $RETCODE\n"
  exit 1
fi