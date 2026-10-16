use std::sync::Mutex;
pub struct StartPlayback(Mutex<Option<Pending>>);
struct Pending {
    asin: String,
    requested_play: bool,
}
#[derive(Debug, PartialEq)]
pub enum Action {
    Play,
    Mini,
}
impl StartPlayback {
    pub fn new() -> Self {
        Self(Mutex::new(None))
    }
    pub fn arm(&self, asin: &str) {
        if let Ok(mut pending) = self.0.lock() {
            *pending = Some(Pending {
                asin: asin.into(),
                requested_play: false,
            });
        }
    }
    pub fn cancel(&self) {
        if let Ok(mut pending) = self.0.lock() {
            *pending = None;
        }
    }
    pub fn update(
        &self,
        asin: &str,
        available: bool,
        sample: bool,
        paused: bool,
    ) -> Option<Action> {
        let mut pending = self.0.lock().ok()?;
        let intent = pending.as_mut()?;
        if intent.asin != asin || !available || sample {
            return None;
        }
        if !paused {
            *pending = None;
            return Some(Action::Mini);
        }
        if !intent.requested_play {
            intent.requested_play = true;
            return Some(Action::Play);
        }
        None
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn waits_for_full_playback_and_opens_mini_once() {
        let state = StartPlayback::new();
        state.arm("B012345678");
        assert_eq!(state.update("B012345678", false, false, true), None);
        assert_eq!(state.update("B012345678", true, true, false), None);
        assert_eq!(
            state.update("B012345678", true, false, true),
            Some(Action::Play)
        );
        assert_eq!(state.update("B012345678", true, false, true), None);
        assert_eq!(
            state.update("B012345678", true, false, false),
            Some(Action::Mini)
        );
        assert_eq!(state.update("B012345678", true, false, false), None);
    }
    #[test]
    fn another_title_cannot_complete_or_reopen_a_cancelled_start() {
        let state = StartPlayback::new();
        state.arm("B012345678");
        assert_eq!(state.update("B987654321", true, false, false), None);
        state.arm("B987654321");
        assert_eq!(state.update("B012345678", true, false, false), None);
        state.cancel();
        assert_eq!(state.update("B987654321", true, false, false), None);
    }
}
