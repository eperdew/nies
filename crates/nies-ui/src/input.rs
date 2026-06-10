//! Keyboard → controller mapping shared by both frontends (global spec
//! §5.5 defaults; M4 design spec §5). Pure state-folding — no winit
//! event-loop coupling, so it unit-tests without a window.
//!
//! Uses `physical_key` (layout-independent positions) so the X/Z action
//! buttons sit side-by-side regardless of keyboard layout. Rebinding is
//! M10.

use nies_core::Buttons;
use winit::event::ElementState;
use winit::keyboard::{KeyCode, PhysicalKey};

/// The §5.5 default keyboard mapping for controller 1. `None` for keys
/// that aren't bound.
pub fn map_key(code: KeyCode) -> Option<Buttons> {
    Some(match code {
        KeyCode::ArrowUp => Buttons::UP,
        KeyCode::ArrowDown => Buttons::DOWN,
        KeyCode::ArrowLeft => Buttons::LEFT,
        KeyCode::ArrowRight => Buttons::RIGHT,
        KeyCode::KeyX => Buttons::A,
        KeyCode::KeyZ => Buttons::B,
        KeyCode::Enter => Buttons::START,
        KeyCode::ShiftRight => Buttons::SELECT,
        _ => return None,
    })
}

/// Folds key transitions into the current port-1 button state.
#[derive(Debug, Default)]
pub struct KeyboardState {
    buttons: Buttons,
}

impl KeyboardState {
    /// Fold one key event in. Returns `Some(new_state)` when the mapped
    /// state changed (forward it to `Nes::set_buttons`); `None` for
    /// repeats, unmapped keys, and no-op transitions — keeps the input
    /// journal free of redundant events.
    pub fn on_key(
        &mut self,
        key: PhysicalKey,
        state: ElementState,
        repeat: bool,
    ) -> Option<Buttons> {
        if repeat {
            return None;
        }
        let PhysicalKey::Code(code) = key else {
            return None;
        };
        let bit = map_key(code)?;
        let next = match state {
            ElementState::Pressed => self.buttons.with(bit),
            ElementState::Released => self.buttons.without(bit),
        };
        if next == self.buttons {
            return None;
        }
        self.buttons = next;
        Some(next)
    }

    /// Release everything (window focus loss — the matching key-up
    /// events will never arrive). Returns `Some` only if keys were held.
    pub fn release_all(&mut self) -> Option<Buttons> {
        if self.buttons == Buttons::default() {
            return None;
        }
        self.buttons = Buttons::default();
        Some(self.buttons)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nies_core::Buttons;
    use winit::event::ElementState;
    use winit::keyboard::{KeyCode, PhysicalKey};

    fn press(ks: &mut KeyboardState, code: KeyCode) -> Option<Buttons> {
        ks.on_key(PhysicalKey::Code(code), ElementState::Pressed, false)
    }

    fn release(ks: &mut KeyboardState, code: KeyCode) -> Option<Buttons> {
        ks.on_key(PhysicalKey::Code(code), ElementState::Released, false)
    }

    #[test]
    fn default_mapping_per_spec_5_5() {
        assert_eq!(map_key(KeyCode::ArrowUp), Some(Buttons::UP));
        assert_eq!(map_key(KeyCode::ArrowDown), Some(Buttons::DOWN));
        assert_eq!(map_key(KeyCode::ArrowLeft), Some(Buttons::LEFT));
        assert_eq!(map_key(KeyCode::ArrowRight), Some(Buttons::RIGHT));
        assert_eq!(map_key(KeyCode::KeyX), Some(Buttons::A));
        assert_eq!(map_key(KeyCode::KeyZ), Some(Buttons::B));
        assert_eq!(map_key(KeyCode::Enter), Some(Buttons::START));
        assert_eq!(map_key(KeyCode::ShiftRight), Some(Buttons::SELECT));
        assert_eq!(map_key(KeyCode::Space), None);
    }

    #[test]
    fn press_and_release_fold_into_state() {
        let mut ks = KeyboardState::default();
        assert_eq!(press(&mut ks, KeyCode::KeyX), Some(Buttons::A));
        assert_eq!(
            press(&mut ks, KeyCode::ArrowRight),
            Some(Buttons::A | Buttons::RIGHT)
        );
        assert_eq!(release(&mut ks, KeyCode::KeyX), Some(Buttons::RIGHT));
        assert_eq!(
            release(&mut ks, KeyCode::ArrowRight),
            Some(Buttons::default())
        );
    }

    #[test]
    fn repeats_unmapped_and_noops_return_none() {
        let mut ks = KeyboardState::default();
        // Key repeat: ignored.
        assert_eq!(
            ks.on_key(
                PhysicalKey::Code(KeyCode::KeyX),
                ElementState::Pressed,
                true
            ),
            None
        );
        // Unmapped key: ignored.
        assert_eq!(press(&mut ks, KeyCode::Space), None);
        // Releasing an unpressed key: no state change, no event.
        assert_eq!(release(&mut ks, KeyCode::KeyZ), None);
        // Double press without release: second is a no-op.
        assert_eq!(press(&mut ks, KeyCode::KeyX), Some(Buttons::A));
        assert_eq!(press(&mut ks, KeyCode::KeyX), None);
    }

    #[test]
    fn release_all_clears_held_state_once() {
        let mut ks = KeyboardState::default();
        press(&mut ks, KeyCode::KeyX);
        press(&mut ks, KeyCode::ArrowDown);
        assert_eq!(ks.release_all(), Some(Buttons::default()));
        assert_eq!(ks.release_all(), None); // already clear: no event spam
    }
}
