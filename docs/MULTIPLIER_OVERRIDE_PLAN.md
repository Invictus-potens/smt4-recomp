# Resource multiplier overrides — design

## What the watchpoint found

Ran a probe build (`~/dev/smt4-probe`) with `code/recomp.h` patched to:

1. Sync `ctx->r[15] = block_address` on every `BLOCK_TO`, so a memory-write
   watchpoint can read an approximate guest PC (block-granular,
   ~6–10 instructions).
2. Inline a check inside `mem_write8/16/32_in` for a few known
   save-data addresses: Macca `0x08291C8C`, App Points `0x0829B470`,
   Hero Max Exp `0x08291CC0`. On a hit it calls `smt4_watch`, a Rust
   `extern "C"` fn in the probe that logs via `log::warn!`.

Needed because the hot path writes directly into mapped pages via
`memcpy` and skips the Host `write32` callback entirely — nothing
would hit a watchpoint on the Rust side.

Raw log from a short play session saved as
`docs/watchpoint-run-1.log`. Hits grouped by (resource, writer PC):

```
6  Macca      0x08291C8C   pc=0x00295EDC
3  Macca      0x08291C8C   pc=0x00295EB8
1  Macca      0x08291C8C   pc=0x0026FAA8
1  HeroExp    0x08291CC0   pc=0x003C2124
1  HeroExp    0x08291CC0   pc=0x0018CF74
2  AppPoints  0x0829B470   pc=0x001A1044
1  AppPoints  0x0829B470   pc=0x001DD828
```

The two Macca pcs (`0x00295EB8`/`0x00295EDC`) are 36 bytes apart in
the same function.

## The function: `f_00295EA8` in `code086.c:37329`

Reads as a generic signed `addResource(id, delta)` — not Macca-specific:

```c
// disassembly intent, from code086.c lines 37329–37458:
void f_00295EA8(uint32_t id /* r0 */, int32_t delta /* r1 */) {
    void *ptr = f_002A8A08(id);        // r5 saves id, r0 becomes ptr
    uint32_t cur = *(uint32_t*)ptr;    // 00295EBC: ldr r0, [r0]
    cmp(delta, 0);                     // 00295EC0
    *(uint32_t*)ptr = cur + delta;     // 00295EC8: str (cur+delta)
    if (delta <= 0 /* BLE */) {        // 00295ECC
        // clamp branch at 00295EDC: if resulting value went negative,
        // write 0 back instead
        goto clamp_at_zero;
    }
}
```

Two things this tells us:

- **A single override covers every resource.** `f_002A8A08(id)` is the
  dispatch table that maps a resource ID (Macca, EXP, each stat, App
  Points, HP, MP, …) to the right slot in the save data. The adder is
  shared.
- **The ID we need is whatever `r0` is on entry to `f_00295EA8`.** We
  don't actually need to know its numeric value upfront — we can let
  the lookup run and compare the returned pointer to our known
  addresses, which already tells us which resource.

## Override plan

Add a `RECOMP_OVERRIDE` for `f_00295EA8` that:

1. Reads `delta = (int32_t)ctx->r[1]` on entry.
2. If `delta > 0`, calls `f_002A8A08` once to peek at the returned
   pointer (same `ctx->r[0]` as the real call will see).
3. Picks a multiplier based on the pointer:
   - `0x08291C8C` → Macca × *macca_mult*
   - `0x08291CC0` → Hero EXP × *exp_mult*
   - `0x0829B470` → App Points × *app_mult*
   - any other → leave delta alone
4. Writes the boosted `delta` back to `ctx->r[1]` and calls through to
   the original `f_00295EA8`.

Negative deltas (spending Macca/App Points) stay untouched so the
clamp branch at `L_00295EDC` still handles them correctly.

Multipliers come from a config file (`settings.toml` already in the
project) so you can tune them per playthrough without rebuilding.

## Open questions before implementing

- The three Macca-write pcs include `0x0026FAA8`, outside
  `f_00295EA8`. Could be the "load save file into memory" path (bulk
  copy of the save struct), not a gameplay gain — a save
  reload-shouldn't double your money. Need to look at `0x0026FAA8`
  and confirm it's init-only before deciding to leave it alone.
- Same for the second App Points pc `0x001DD828` and the two Hero
  Exp pcs `0x003C2124` / `0x0018CF74`. Each one might be:
  - another caller of `f_00295EA8` (irrelevant — the override already
    catches those from `f_00295EA8`'s side), or
  - direct writes bypassing `f_00295EA8` (relevant — would need their
    own overrides or we'd miss some gains). Quick C read of each
    address decides it.
- Need to confirm `f_002A8A08` is purely a lookup (no side effects,
  idempotent) so calling it twice (once to peek, once as the real
  call) is safe. If it does caching or state, we'd peek via a plain
  memory read of the returned pointer stored at a known offset
  instead.

Next: open `code086.c` around `0x0026FAA8` and `code0??.c` around the
other 3 pcs; then write the override, build, test with the save state
to confirm gained Macca jumps to `×mult`.
