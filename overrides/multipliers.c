/* Resource multipliers for Shin Megami Tensei IV, chosen from Zakuro's
 * cheat menu.
 *
 * f_00295EAC is the title's addMacca(int32 delta): delta in r0, it fetches
 * &macca via f_002A8A08 (a no-argument getter: *(global+4) + 0x8C), adds,
 * calls 0x0029C0FC (macca-earned tracking) for positive deltas, then clamps
 * to [0, max]. 0x00295EA8 is a NOP (mov r0,r0) that falls into the same
 * code; a few callers enter there, battle results and shops enter at EAC.
 * Only positive deltas are boosted, so spending is untouched.
 *
 * Control bytes: cheats/00040000000E5C00.txt has entries like
 * "[Macca x2] 204EFF00 00000002" that write the multiplier into a control
 * byte every frame. The bytes live in the slack between the end of .bss
 * (0x004EFE34) and the end of its last page (0x004F0000): mapped, zeroed,
 * and never touched by the game.
 *
 * Zakuro doesn't undo a cheat when it's turned off, so the override zeroes
 * the byte after reading it and keeps the value for KEEP_MS to cover several
 * gains in one frame. Turn the cheat off and nothing writes the byte again,
 * so it reads 0 and the multiplier drops back to x1.
 *
 * NOTE (temporary): f_00295EAC is hooked by renaming the generated symbol to
 * orig_f_00295EAC in code086.o with objcopy and defining f_00295EAC here,
 * because callers have CALL(f_00295EAC) baked into the generated C. A proper
 * RECOMP_OVERRIDE(0x00295EAC) needs a re-run of `3dsrecomp port`.
 */

#include <stdint.h>
#include <time.h>
#include "overrides.h"

/* Rust side, src/main.rs (C stdio isn't reliable in the Windows GUI app). */
extern void smt4_log_override(const char *resource, uint32_t lr,
                              uint32_t total_before, int32_t delta_in, int32_t delta_out,
                              int32_t num, int32_t den);

#define CTRL_MACCA      0x004EFF00u
#define CTRL_EXP        0x004EFF01u /* reserved, hook pending */
#define CTRL_APP_POINTS 0x004EFF02u /* reserved, hook pending */

#define SAVE_MACCA 0x08291C8Cu

/* a cheat rewrites its byte every frame, ~16-33 ms */
#define KEEP_MS 100

typedef struct {
    uint32_t address;
    uint8_t cached;
    uint64_t at_ms;
} Knob;

static Knob macca_knob = { CTRL_MACCA, 0, 0 };
static Knob exp_knob = { CTRL_EXP, 0, 0 };

static uint64_t now_ms(void) {
    struct timespec ts;
    timespec_get(&ts, TIME_UTC);
    return (uint64_t)ts.tv_sec * 1000u + (uint64_t)ts.tv_nsec / 1000000u;
}

/* the multiplier a knob's cheat asks for, 1 when its cheat is off */
static uint32_t knob_read(Context *ctx, Knob *knob) {
    uint8_t value = mem_read8(ctx, knob->address);
    uint64_t now = now_ms();
    if (value) {
        knob->cached = value;
        knob->at_ms = now;
        mem_write8(ctx, knob->address, 0);
        return value;
    }
    if (knob->cached && now - knob->at_ms < KEEP_MS)
        return knob->cached;
    knob->cached = 0;
    return 1;
}

extern void orig_f_00295EAC(Context *ctx);

/* hero EXP gained from a battle, called from the patched add at 0x0018CF84
 * in the battle-results function (see scripts/patch_generated.py) */
uint32_t smt4_boost_exp(Context *ctx, uint32_t gain) {
    if ((int32_t)gain <= 0) return gain;
    uint32_t mult = knob_read(ctx, &exp_knob);
    if (mult <= 1) return gain;
    uint64_t boosted = (uint64_t)gain * mult;
    if (boosted > INT32_MAX) boosted = INT32_MAX;
    smt4_log_override("HeroExp", 0x0018CF84u, mem_read32(ctx, 0x08291CC0u), (int32_t)gain, (int32_t)boosted, (int32_t)mult, 1);
    return (uint32_t)boosted;
}

static void boost_macca(Context *ctx) {
    int32_t delta = (int32_t)ctx->r[0];
    if (delta <= 0) return;
    uint32_t mult = knob_read(ctx, &macca_knob);
    if (mult <= 1) return;
    int64_t boosted = (int64_t)delta * mult;
    if (boosted > INT32_MAX) boosted = INT32_MAX;
    ctx->r[0] = (uint32_t)(int32_t)boosted;
    smt4_log_override("Macca", ctx->r[14], mem_read32(ctx, SAVE_MACCA), delta, (int32_t)boosted, (int32_t)mult, 1);
}

/* hot entry, used by battle rewards and shops */
void f_00295EAC(Context *ctx) {
    boost_macca(ctx);
    CALL(orig_f_00295EAC);
}

/* the NOP entry right before it */
RECOMP_OVERRIDE(0x00295EA8) {
    boost_macca(ctx);
    CALL(RECOMP_ORIGINAL(0x00295EA8));
}
