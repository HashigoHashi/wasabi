#!/bin/bash -e
SCRIPT_PATH="${BASH_SOURCE[0]:-$0}"
PROJ_ROOT="$(cd "$(dirname "$(dirname "$SCRIPT_PATH")")" && pwd)"
cd "${PROJ_ROOT}"

PATH_TO_EFI="$1"
rm -rf mnt
mkdir -p mnt/EFI/BOOT/
cp ${PATH_TO_EFI} mnt/EFI/BOOT/BOOTX64.EFI
qemu-system-x86_64 \
  -m 4G \
  -bios third_party/ovmf/RELEASEX64_OVMF.fd \
  -drive format=raw,file=fat:rw:mnt \
  -vnc :1