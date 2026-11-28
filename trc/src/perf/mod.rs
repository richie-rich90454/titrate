//! VM fast-path helpers for the performance phases of the roadmap (Phases 5–7).
//!
//! Everything here is additive metadata or a fixed-size probe. Nothing in the
//! existing VM is changed by this module: wiring the helpers into `Vm::new`
//! and the run loop is a separate change. The helpers are deliberately
//! allocation-free on their hot paths so that wiring them in cannot regress
//! startup or dispatch latency.

mod dispatch;
mod inline_cache;
mod specialization;
mod startup;

pub use dispatch::{NativeDispatch, NativeDispatchError, DISPATCH_CAPACITY};
pub use inline_cache::{InlineCache, Prim, PrimShape, INLINE_CACHE_SLOTS};
pub use specialization::{Hints, HOT_THRESHOLD};
pub use startup::{
    measure_pipeline, measure_vm_start, PipelineTiming, StartupProbe, STARTUP_BUDGET_MS,
};
