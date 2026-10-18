/* Easy recruit for Shin Megami Tensei IV (USA), chosen from Zakuro's cheat
 * menu. See docs/RECRUIT.md.
 *
 * Demon negotiation runs in a flowscript, battle/talk/talk.bf. Its checks
 * (does the answer please the demon, does refusing a demand still work, does
 * the demon run off with what it took, ...) each start by asking a debug
 * setting through script native 0x0259, f_002F127C, which reads it from the
 * talk context at +0x22A8 + index * 4. In the shipped game they're all 0, so
 * the checks go on to roll their odds. scripts/patch_generated.py routes that
 * read through smt4_talk_debug, which, while the [Easy Recruit] cheat is on,
 * answers with the settings that make every check go the player's way.
 *
 * The indices match the debug menu's TalkCtrl entries (FTALK is 0).
 */

#include <stdint.h>
#include <time.h>
#include "overrides.h"

#define CTRL_RECRUIT 0x004EFF05u
#define KEEP_MS 100

enum {
    ANGRY = 7,              /* nonzero: the demon is angry from the start */
    TALKLOOP_SUC_CTRL = 12, /* 1: the answer works, 2: it fails */
    REQLOOP_CHEAT_CTRL = 15,/* 1: the demon takes the gift and runs, 2: it doesn't */
    REQLOOP_CUT_CTRL = 16,  /* 1: the demon cuts the talk short, 2: it doesn't */
    REQLOOP_REJECT_SUC = 20,/* nonzero: refusing a demand still goes well */
    REQLOOP_ACCEPT_FAIL = 21,/* nonzero: giving what it asks still fails */
    REQLOOP_SHORT_SUC = 22, /* nonzero: not having what it asks still goes well */
    REQLOOP_RUNAWAY = 23,   /* nonzero: the demon runs away */
    ROBLOOP_NOT_ANGER = 24, /* nonzero: the demon doesn't get angry at a robbery */
};

static uint8_t cached;
static uint64_t cached_at;

static uint64_t now_ms(void) {
    struct timespec ts;
    timespec_get(&ts, TIME_UTC);
    return (uint64_t)ts.tv_sec * 1000u + (uint64_t)ts.tv_nsec / 1000000u;
}

/* whether the cheat is on; the byte is cleared after reading, as in
 * multipliers.c, since Zakuro doesn't undo a cheat that's turned off */
static int on(Context *ctx) {
    uint8_t value = mem_read8(ctx, CTRL_RECRUIT);
    uint64_t now = now_ms();
    if (value) {
        cached = value;
        cached_at = now;
        mem_write8(ctx, CTRL_RECRUIT, 0);
        return 1;
    }
    if (cached && now - cached_at < KEEP_MS)
        return 1;
    cached = 0;
    return 0;
}

uint32_t smt4_talk_debug(Context *ctx, uint32_t index, uint32_t value) {
    if (!on(ctx))
        return value;
    switch (index) {
        case TALKLOOP_SUC_CTRL: return 1;
        case REQLOOP_CHEAT_CTRL: return 2;
        case REQLOOP_CUT_CTRL: return 2;
        case REQLOOP_REJECT_SUC: return 1;
        case REQLOOP_SHORT_SUC: return 1;
        case ROBLOOP_NOT_ANGER: return 1;
        case ANGRY:
        case REQLOOP_ACCEPT_FAIL:
        case REQLOOP_RUNAWAY: return 0;
        default: return value;
    }
}
