/* Resource multipliers for Shin Megami Tensei IV (USA), chosen from Zakuro's
 * cheat menu. See docs/MULTIPLIERS.md for how each hook was found.
 *
 * Control bytes: cheats/00040000000E5C00.txt has entries like
 * "[Macca x2] 204EFF00 00000002" that write a multiplier into a control
 * byte every frame. The bytes live in the slack between the end of .bss
 * (0x004EFE34) and the end of its last page (0x004F0000): mapped, zeroed,
 * and never touched by the game.
 *
 * Zakuro doesn't undo a cheat when it's turned off, so a hook zeroes the
 * byte after reading it and keeps the value for KEEP_MS to cover several
 * gains in one frame. Turn the cheat off and nothing writes the byte again,
 * so it reads 0 and the multiplier drops back to x1.
 *
 * Only positive gains are boosted; spending and losses are untouched.
 */

#include <stdint.h>
#include <time.h>
#include "overrides.h"

/* Rust side, src/main.rs: logs each boost (C stdio isn't reliable in the
 * Windows GUI app). */
extern void smt4_log_override(const char *resource, uint32_t lr,
                              uint32_t total_before, int32_t delta_in, int32_t delta_out,
                              int32_t num, int32_t den);

#define CTRL_MACCA      0x004EFF00u
#define CTRL_HERO_EXP   0x004EFF01u
#define CTRL_APP_POINTS 0x004EFF02u
#define CTRL_DEMON_EXP  0x004EFF03u

#define SAVE_MACCA      0x08291C8Cu
#define SAVE_HERO_EXP   0x08291CC0u
#define SAVE_APP_POINTS 0x0829B470u

/* a cheat rewrites its byte every frame, ~16-33 ms */
#define KEEP_MS 100

typedef struct {
    uint32_t address;
    uint8_t cached;
    uint64_t at_ms;
} Knob;

static Knob macca_knob = { CTRL_MACCA, 0, 0 };
static Knob hero_exp_knob = { CTRL_HERO_EXP, 0, 0 };
static Knob app_knob = { CTRL_APP_POINTS, 0, 0 };
static Knob demon_exp_knob = { CTRL_DEMON_EXP, 0, 0 };

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

/* a positive gain times its knob's multiplier, saturating */
static uint32_t boost(Context *ctx, Knob *knob, uint32_t gain, const char *name,
                      uint32_t total_address, uint32_t from) {
    if ((int32_t)gain <= 0) return gain;
    uint32_t mult = knob_read(ctx, knob);
    if (mult <= 1) return gain;
    int64_t boosted = (int64_t)(int32_t)gain * mult;
    if (boosted > INT32_MAX) boosted = INT32_MAX;
    smt4_log_override(name, from, mem_read32(ctx, total_address), (int32_t)gain, (int32_t)boosted, (int32_t)mult, 1);
    return (uint32_t)boosted;
}

/* addMacca(int32 delta), delta in r0. Fetches &macca from a no-argument
 * getter (f_002A8A08: *(global+4) + 0x8C), adds, notes positive deltas in a
 * macca-earned counter (0x0029C0FC), clamps to [0, max]. Battle rewards and
 * shops enter at 0x00295EAC; 0x00295EA8 is a NOP (mov r0,r0) right before
 * it that a few callers enter at instead. */
RECOMP_OVERRIDE(0x00295EAC) {
    ctx->r[0] = boost(ctx, &macca_knob, ctx->r[0], "Macca", SAVE_MACCA, ctx->r[14]);
    CALL(RECOMP_ORIGINAL(0x00295EAC));
}

RECOMP_OVERRIDE(0x00295EA8) {
    ctx->r[0] = boost(ctx, &macca_knob, ctx->r[0], "Macca", SAVE_MACCA, ctx->r[14]);
    CALL(RECOMP_ORIGINAL(0x00295EA8));
}

/* addAppPoints(int32 delta), delta in r0. Adds to the lifetime-earned
 * counter (base+0x986C) and the current points (base+0x9870), then clamps
 * the current value to the max. */
RECOMP_OVERRIDE(0x0026E8A0) {
    ctx->r[0] = boost(ctx, &app_knob, ctx->r[0], "AppPoints", SAVE_APP_POINTS, ctx->r[14]);
    CALL(RECOMP_ORIGINAL(0x0026E8A0));
}

/* Hero EXP from a battle. The battle-results function (f_0018BFEC) adds it
 * inline, hero.exp + results[+0x35B4], at 0x0018CF84, which isn't a function
 * entry, so tools/build/src/patch.rs rewrites that add to call this. */
uint32_t smt4_boost_exp(Context *ctx, uint32_t gain) {
    return boost(ctx, &hero_exp_knob, gain, "HeroExp", SAVE_HERO_EXP, 0x0018CF84u);
}

/* Demon EXP from a battle. Further down the same function, a loop over the
 * 24 stock slots does, for each demon:
 *     gain = results[0x35B8 + id*4]     (0x0018D190, pointer kept on stack)
 *     demon.exp += gain                 (0x0018D194, demon struct +0x30)
 * and in some cases undoes it at 0x0018D230 by re-reading the gain through
 * that pointer. So the gain is boosted where it's loaded and written back,
 * and every later read, the undo included, sees the same value.
 * tools/build/src/patch.rs turns that load into a call to this. */
uint32_t smt4_boost_demon_exp(Context *ctx, uint32_t address) {
    uint32_t gain = mem_read32(ctx, address);
    uint32_t boosted = boost(ctx, &demon_exp_knob, gain, "DemonExp", address, 0x0018D190u);
    if (boosted != gain)
        mem_write32(ctx, address, boosted);
    return boosted;
}
