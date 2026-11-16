//! Renders effects to intents on background threads, as Vixen's `PreRender` does.

use rayon::prelude::*;
use std::panic::AssertUnwindSafe;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use vixen_core::{EffectIntents, ElementTree, MarkCollection, NodeId, TimeSpan, Uuid};
use vixen_effects::framework::{AudioSource, EffectDescriptor, Params, RenderCtx};

/// What every render in a sequence shares.
pub struct RenderEnv<'a> {
    pub video_cache: Option<&'a vixen_effects::framework::VideoCacheSession>,
    pub tree: &'a ElementTree,
    pub frame_ms: i32,
    pub marks: &'a [MarkCollection],
    pub audio: Option<&'a dyn AudioSource>,
    pub module_data_root: Option<&'a Path>,
}

/// One effect to render.
pub struct EffectJob<'a> {
    pub id: Uuid,
    pub descriptor: &'static EffectDescriptor,
    pub params: &'a Params,
    pub targets: &'a [NodeId],
    pub start: TimeSpan,
    pub span: TimeSpan,
}

/// Renders one effect. An effect that panics renders nothing and reports the failure: one bad effect must
/// never take down a running show, just as a C# render exception only leaves that effect dark.
pub fn render_effect(env: &RenderEnv, job: &EffectJob, cancel: &AtomicBool) -> Result<EffectIntents, String> {
    let ctx = RenderCtx { video_cache: env.video_cache.map(|session| vixen_effects::framework::VideoCacheContext { session, instance: job.id }),
        tree: env.tree,
        targets: job.targets,
        start_time: job.start,
        time_span: job.span,
        frame_ms: env.frame_ms,
        marks: env.marks,
        audio: env.audio,
        module_data_root: env.module_data_root,
        cancel,
    };
    let render = job.descriptor.render;
    std::panic::catch_unwind(AssertUnwindSafe(|| render(&ctx, job.params))).map_err(|payload| {
        let text = payload
            .downcast_ref::<&str>()
            .map(|s| s.to_string())
            .or_else(|| payload.downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "unknown panic".to_string());
        format!("{} effect {} failed to render: {text}", job.descriptor.name, job.id)
    })
}

/// How far a batch render has got, for the editor's status bar.
#[derive(Default)]
pub struct RenderProgress {
    pub total: AtomicUsize,
    pub done: AtomicUsize,
    pub failed: AtomicUsize,
}

/// Renders a batch in parallel, returning each effect's intents in job order. A failed effect yields empty
/// intents and increments `progress.failed`, with its message passed to `on_error`.
pub fn render_all(
    env: &RenderEnv,
    jobs: &[EffectJob],
    cancel: &AtomicBool,
    progress: &RenderProgress,
    on_error: &(dyn Fn(String) + Sync),
) -> Vec<Arc<EffectIntents>> {
    progress.total.store(jobs.len(), Ordering::Relaxed);
    progress.done.store(0, Ordering::Relaxed);
    progress.failed.store(0, Ordering::Relaxed);
    // `RenderEnv` holds a `&dyn AudioSource`, which is Sync by the trait's bounds.
    jobs.par_iter()
        .map(|job| {
            let result = if cancel.load(Ordering::Relaxed) {
                Ok(EffectIntents::new())
            } else {
                render_effect(env, job, cancel)
            };
            progress.done.fetch_add(1, Ordering::Relaxed);
            match result {
                Ok(intents) => Arc::new(intents),
                Err(message) => {
                    progress.failed.fetch_add(1, Ordering::Relaxed);
                    on_error(message);
                    Arc::new(EffectIntents::new())
                }
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use vixen_effects::framework::EffectGroup;

    fn render_ok(_ctx: &RenderCtx, _p: &Params) -> EffectIntents {
        EffectIntents::new()
    }

    fn render_panics(_ctx: &RenderCtx, _p: &Params) -> EffectIntents {
        panic!("boom");
    }

    const fn descriptor(render: vixen_effects::framework::RenderFn) -> EffectDescriptor {
        EffectDescriptor {
            type_id: Uuid::nil(),
            name: "Test",
            group: EffectGroup::Basic,
            author: "",
            description: "",
            version: "",
            data_type: "",
            data_ns: "",
            info_link: "",
            information: "",
            icon: "",
            params: &[],
            render,
            visibility: None,
            param_changed: None,
            param_changed_ctx: None,
            targets_changed: None,
            validate: None,
            force_visual: false,
            force_visual_when: None,
            visual: None,
            selection: None,
            supports_location: false,
            ported: true,
        }
    }

    static OK: EffectDescriptor = descriptor(render_ok);
    static BAD: EffectDescriptor = descriptor(render_panics);

    #[test]
    fn a_panicking_effect_is_contained_and_reported() {
        let tree = ElementTree::new();
        let env = RenderEnv { video_cache: None, tree: &tree, frame_ms: 50, marks: &[], audio: None, module_data_root: None };
        let p = Params::defaults(&[]);
        let id = Uuid::new_v4();
        let jobs = [
            EffectJob { id, descriptor: &BAD, params: &p, targets: &[], start: TimeSpan::ZERO, span: TimeSpan::from_ms_i(100) },
            EffectJob { id: Uuid::new_v4(), descriptor: &OK, params: &p, targets: &[], start: TimeSpan::ZERO, span: TimeSpan::from_ms_i(100) },
        ];
        let progress = RenderProgress::default();
        let errors = parking_lot::Mutex::new(Vec::new());
        // Silence the default panic message for this deliberate panic.
        let hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));
        let out = render_all(&env, &jobs, &AtomicBool::new(false), &progress, &|m| errors.lock().push(m));
        std::panic::set_hook(hook);
        assert_eq!(out.len(), 2);
        assert_eq!(progress.done.load(Ordering::Relaxed), 2);
        assert_eq!(progress.failed.load(Ordering::Relaxed), 1);
        assert!(errors.lock()[0].contains("boom"));
    }
}
