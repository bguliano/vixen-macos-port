//! F198: the State mapper's Preview. Upstream sends a Live context `TurnOnElements` message, which reaches every open preview and any
//! running controller. A named live context mixes these states with ordinary playback
//! and keeps them lit until the mapper changes or clears its preview.

use crate::app::AppCore;
use uuid::Uuid;
use vixen_core::Color;

/// Lights each element in its colour at full intensity, and everything else off. `discrete` says the element takes discrete colours.
pub fn show(core: &AppCore, elements: &[(Uuid, Color, bool)]) {
    core.live.state_preview(core, elements);
}

/// Everything off, as closing the mapper or turning Preview off leaves it.
pub fn clear(core: &AppCore) {
    core.live.clear("State Preview");
}

// Native UI test observation only. The sink is installed only under VIXEN_UI_TEST.
// It observes the same evaluated frame stream as controllers/previews without
// starting either hardware or a window-server surface.
#[derive(Default)]
struct ProbeSnapshot { frames: u64, colors: Vec<Vec<u32>> }
pub struct FrameProbe(std::sync::Arc<parking_lot::Mutex<ProbeSnapshot>>);
struct ProbeSink(std::sync::Weak<parking_lot::Mutex<ProbeSnapshot>>);
impl vixen_engine::FrameSink for ProbeSink {
    fn frame(&mut self, _: vixen_core::TimeSpan, states: &[vixen_core::CombinedState], _: bool) {
        let Some(shared) = self.0.upgrade() else { return };
        let mut snapshot = shared.lock();
        snapshot.frames += 1;
        snapshot.colors = states.iter().map(|s| {
            s.lighting.map(|v| v.full_color()).into_iter()
                .chain(s.rgb.map(|v| v.full_color()))
                .chain(s.discrete.iter().map(|v| v.full_color()))
                .map(|c| c.to_argb_u32()).filter(|c| c & 0xffffff != 0).collect()
        }).collect();
    }
}
impl FrameProbe {
    pub fn attach(core: &crate::app::AppCore) -> Option<Self> {
        if std::env::var_os("VIXEN_UI_TEST").is_none() { return None; }
        let shared = std::sync::Arc::new(parking_lot::Mutex::new(ProbeSnapshot::default()));
        core.playback.add_sink(Box::new(ProbeSink(std::sync::Arc::downgrade(&shared))));
        Some(Self(shared))
    }
    pub fn snapshot(&self) -> String {
        let snapshot = self.0.lock();
        serde_json::json!({"frames":snapshot.frames,"lit":snapshot.colors.iter().filter(|c| !c.is_empty()).count(),"colors":snapshot.colors}).to_string()
    }
}
