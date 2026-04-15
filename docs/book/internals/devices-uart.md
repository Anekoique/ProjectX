# UART 16550

National Semiconductor 16550-compatible UART at `0x1000_0000`,
PLIC source 10.

## Registers

| Offset | DLAB=0 read | DLAB=0 write | DLAB=1 |
|--------|-------------|--------------|--------|
| 0 | RBR (RX data) | THR (TX data) | DLL (divisor low) |
| 1 | IER | IER | DLM (divisor high) |
| 2 | IIR (read) | FCR (write) | IIR / FCR |
| 3 | LCR | LCR | LCR |
| 4 | MCR | MCR | MCR |
| 5 | LSR | — | LSR |
| 6 | MSR | MCR | MSR |
| 7 | SCR | SCR | SCR |

- `LCR[7]` is DLAB — toggles register meaning for offsets 0/1.
- `LSR.DR` = RX ready (derived from `rx_fifo`).
- `LSR.THRE` / `LSR.TEMT` = always set (TX is synchronous to stdout).

## Modes

### Default (stdio, batch-friendly)

- TX → host stdout.
- RX → host stdin. Non-blocking poll per tick.

### PTY mode (`DEBUG=y`)

- TX → PTY master.
- RX → PTY master.
- Attach the slave with `screen /dev/ttysXXX 115200`. xemu prints the
  slave path at startup.

### Keyboard am-test

`TEST=k` runs a bare-metal kernel that polls RBR and echoes to TX —
the canonical interactive smoke test.

## Interrupts

`irq_line()` = `!rx_fifo.is_empty() && (ier & 0x1)`.

THRE interrupts (`ier & 0x2`) are also supported: when the guest
writes THR and re-arms IER, the next tick promotes `thre_pending`
into `thre_ip` and re-syncs the IRQ state.

## Ctrl-A X

xemu intercepts the `Ctrl-A X` escape sequence (QEMU-style) to exit
cleanly from firmware-boot modes without needing a guest `poweroff`.
