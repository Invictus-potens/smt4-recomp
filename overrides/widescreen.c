/* Widescreen for Shin Megami Tensei IV (USA), chosen from Zakuro's cheat
 * menu. See docs/WIDESCREEN.md.
 *
 * The game builds its 3D projection with f_002B761C(mtx, fovy, aspect,
 * near, far), aspect in s1, which sets m[0][0] = cot(fovy/2) / aspect. Its
 * camera, f_002C1FFC(camera, fovy_degrees, near, far, aspect), aspect in s3,
 * keeps the aspect in the camera too. Both are given 400/240 for the top
 * screen. scripts/patch_generated.py makes both entries pass their aspect
 * through smt4_widen_aspect, which swaps 400/240 for a wider one: the scene
 * gets more view at the sides with the same vertical view (Hor+).
 *
 * The game still draws into the console's 400x240, so the wider scene comes
 * out squeezed, and Zakuro's "Top screen shape" setting has to be set to the
 * same shape to stretch it back out.
 *
 * A [Widescreen ...] cheat writes the shape into the control byte every
 * frame; turning it off drops back to 400/240 the next time the game sets
 * its camera (on the next scene change, not at once).
 */

#include <stdint.h>
#include <string.h>
#include <time.h>
#include "overrides.h"

#define CTRL_WIDESCREEN 0x004EFF04u
#define KEEP_MS 100

#define CONSOLE_ASPECT (400.0f / 240.0f)

static uint8_t cached;
static uint64_t cached_at;

static uint64_t now_ms(void) {
    struct timespec ts;
    timespec_get(&ts, TIME_UTC);
    return (uint64_t)ts.tv_sec * 1000u + (uint64_t)ts.tv_nsec / 1000000u;
}

/* the shape the cheat asks for, 0 when it's off; cleared after reading, as
 * in multipliers.c, since Zakuro doesn't undo a cheat that's turned off */
static uint8_t shape(Context *ctx) {
    uint8_t value = mem_read8(ctx, CTRL_WIDESCREEN);
    uint64_t now = now_ms();
    if (value) {
        cached = value;
        cached_at = now;
        mem_write8(ctx, CTRL_WIDESCREEN, 0);
        return value;
    }
    if (cached && now - cached_at < KEEP_MS)
        return cached;
    cached = 0;
    return 0;
}

uint32_t smt4_widen_aspect(Context *ctx, uint32_t bits) {
    float aspect;
    memcpy(&aspect, &bits, 4);
    if (aspect < CONSOLE_ASPECT - 0.001f || aspect > CONSOLE_ASPECT + 0.001f)
        return bits;
    float wide;
    switch (shape(ctx)) {
        case 1: wide = 16.0f / 9.0f; break;
        case 2: wide = 21.0f / 9.0f; break;
        default: return bits;
    }
    memcpy(&bits, &wide, 4);
    return bits;
}
