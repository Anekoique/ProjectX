// ramdisk.c — serve block I/O from an in-memory fs.img embedded in the kernel.
// Replaces virtio_disk.c for emulators without virtio-blk.

#include "types.h"
#include "riscv.h"
#include "defs.h"
#include "param.h"
#include "spinlock.h"
#include "sleeplock.h"
#include "fs.h"
#include "buf.h"

extern char _binary_fs_img_start[];
extern char _binary_fs_img_end[];

void
virtio_disk_init(void)
{
  // ramdisk is already in memory — nothing to initialize
}

void
virtio_disk_rw(struct buf *b, int write)
{
  char *addr = _binary_fs_img_start + b->blockno * BSIZE;

  if(write)
    memmove(addr, b->data, BSIZE);
  else
    memmove(b->data, addr, BSIZE);

  b->disk = 0;
}

void
virtio_disk_intr(void)
{
  // ramdisk has no interrupts
}
