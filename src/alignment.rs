//! Shows the hero's alignment over the game, which the game keeps hidden.
//! See docs/ALIGNMENT.md.
//!
//! Alignment is a signed 16-bit number in the save data, Law positive and
//! Chaos negative. Event scripts change it through script native 0x007D,
//! f_002F5FE4, which calls f_00334064(delta): that adds delta to the halfword
//! at `save + 0x74`, where `save` is f_002B66EC(), `[0x0043E2A8 + 4] + 0x90`.
//! The endings take +9 and up as Law, -9 and down as Chaos, Neutral between.
//!
//! Two cheats turn it on, each writing a control byte every frame, as the
//! multipliers do: [Show Alignment] keeps it in a corner, [Alignment Notices]
//! shows a notice each time it changes. The byte is cleared after reading,
//! so turning the cheat off turns the feature off.

use std::time::{Duration, Instant};

use zakuro::{GameOverlay, Memory, OverlayText};

/// the global whose second word points at the game's state.
const STATE: u32 = 0x0043E2A8;
/// where alignment is from that pointer: + 0x90 to the save, + 0x74 in it.
const ALIGNMENT: u32 = 0x90 + 0x74;
/// the control bytes the cheats write, past the multipliers' and Easy
/// Recruit's.
const SHOW: u32 = 0x004EFF06;
const NOTICES: u32 = 0x004EFF07;
/// how long a cheat counts as on after its byte was last seen set.
const KEEP: Duration = Duration::from_millis(200);
/// a change bigger than any one choice makes, a save loaded or a new game,
/// which is taken as the new value without a notice.
const MOST_ONE_CHOICE: i32 = 30;
/// the endings' thresholds.
const LAW: i32 = 9;
const CHAOS: i32 = -9;

#[derive(Default)]
pub struct Alignment {
    show_seen: Option<Instant>,
    notices_seen: Option<Instant>,
    last: Option<i32>,
}

/// the side of the endings a value is on.
pub fn side(value: i32) -> &'static str {
    if value >= LAW {
        "Law"
    } else if value <= CHAOS {
        "Chaos"
    } else {
        "Neutral"
    }
}

/// "Law (+12)", "Neutral (0)", "Chaos (-15)".
pub fn describe(value: i32) -> String {
    let number = if value > 0 { format!("+{value}") } else { value.to_string() };
    format!("{} ({number})", side(value))
}

/// what a change from before to now says, or none when it shouldn't be told.
pub fn notice(before: i32, now: i32) -> Option<String> {
    let delta = now - before;
    if delta == 0 || delta.abs() > MOST_ONE_CHOICE {
        return None;
    }
    let toward = if delta > 0 { "toward Law" } else { "toward Chaos" };
    let delta = if delta > 0 { format!("+{delta}") } else { delta.to_string() };
    Some(format!("Alignment {delta} {toward}: {}", describe(now)))
}

fn read_u8(memory: &mut Memory, addr: u32) -> u8 {
    let mut byte = [0u8; 1];
    memory.read_bytes(addr, &mut byte);
    byte[0]
}

/// the alignment, once the game has its state in memory.
fn alignment(memory: &mut Memory) -> Option<i32> {
    let state = memory.peek32(STATE + 4);
    // the heap the state is made in; before it's made the pointer is 0
    if !(0x0800_0000..0x1000_0000).contains(&state) {
        return None;
    }
    let mut half = [0u8; 2];
    memory.read_bytes(state + ALIGNMENT, &mut half);
    Some(i16::from_le_bytes(half) as i32)
}

impl Alignment {
    /// whether a cheat's byte is set now or was a moment ago, clearing it.
    fn on(memory: &mut Memory, byte: u32, seen: &mut Option<Instant>) -> bool {
        if read_u8(memory, byte) != 0 {
            memory.write_bytes(byte, &[0]);
            *seen = Some(Instant::now());
        }
        seen.is_some_and(|when| when.elapsed() < KEEP)
    }
}

impl GameOverlay for Alignment {
    fn frame(&mut self, memory: &mut Memory) -> OverlayText {
        let show = Alignment::on(memory, SHOW, &mut self.show_seen);
        let notices = Alignment::on(memory, NOTICES, &mut self.notices_seen);
        let mut text = OverlayText::default();
        if !show && !notices {
            self.last = None;
            return text;
        }
        let Some(now) = alignment(memory) else {
            self.last = None;
            return text;
        };
        if show {
            text.corner = Some(format!("Alignment: {}", describe(now)));
        }
        if notices {
            if let Some(before) = self.last {
                text.notice = notice(before, now);
            }
        }
        self.last = Some(now);
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sides_follow_the_endings() {
        assert_eq!(describe(9), "Law (+9)");
        assert_eq!(describe(8), "Neutral (+8)");
        assert_eq!(describe(0), "Neutral (0)");
        assert_eq!(describe(-8), "Neutral (-8)");
        assert_eq!(describe(-9), "Chaos (-9)");
    }

    #[test]
    fn notices_tell_choices_not_loads() {
        assert_eq!(notice(3, 8).as_deref(), Some("Alignment +5 toward Law: Neutral (+8)"));
        assert_eq!(notice(-5, -15).as_deref(), Some("Alignment -10 toward Chaos: Chaos (-15)"));
        assert_eq!(notice(4, 4), None);
        assert_eq!(notice(0, 60), None);
    }
}
