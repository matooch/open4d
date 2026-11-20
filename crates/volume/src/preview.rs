//! The viewport preview of a volume: a coarse iso-surface computed off the main thread, the
//! box of the grid and slice samples (docs/specs/VOLUME.md section 11).

use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use forge_core::{EvalCtx, NodeId, Scene};
use forge_geometry::PolyMesh;
use glam::{DVec3, IVec3};

use crate::node::{BuildOptions, Volume, VolumeBuildError, mesh_volume_node_with};

/// Cells along the longest axis of a preview (VOLUME.md section 11.1).
pub const DEFAULT_PREVIEW_CELLS: u32 = 48;

/// What the viewport shows of a volume.
#[derive(Debug, Clone, PartialEq)]
pub struct Preview {
    /// The coarse iso-surface in the mesher's space.
    pub mesh: PolyMesh,
    /// The box of the active voxels in the mesher's space.
    pub bounds: Option<(DVec3, DVec3)>,
    /// The voxel size the preview was built with (at least the builder's).
    pub voxel_size: f64,
    pub active_voxels: usize,
}

/// The box (mesher space) of the active voxels of a volume.
pub fn volume_bounds(volume: &Volume) -> Option<(DVec3, DVec3)> {
    let (lo, hi) = volume.grid.bounds()?;
    let (a, b) = (
        volume.grid.index_to_world(lo),
        volume.grid.index_to_world(hi),
    );
    let mut min = DVec3::splat(f64::MAX);
    let mut max = DVec3::splat(f64::MIN);
    for c in 0..8 {
        let p = DVec3::new(
            if c & 1 == 0 { a.x } else { b.x },
            if c & 2 == 0 { a.y } else { b.y },
            if c & 4 == 0 { a.z } else { b.z },
        );
        let q = volume.to_parent.transform_point3(p);
        min = min.min(q);
        max = max.max(q);
    }
    Some((min, max))
}

/// Builds the preview of a mesher node synchronously at about `max_cells` cells.
pub fn build_preview(
    scene: &Scene,
    mesher: NodeId,
    ctx: &EvalCtx,
    max_cells: u32,
) -> Result<Option<Preview>, VolumeBuildError> {
    let opts = BuildOptions {
        max_cells: Some(max_cells),
    };
    Ok(
        mesh_volume_node_with(scene, mesher, ctx, &opts)?.map(|(mesh, volume)| Preview {
            mesh,
            bounds: volume_bounds(&volume),
            voxel_size: volume.grid.voxel_size(),
            active_voxels: volume.grid.active_count(),
        }),
    )
}

/// One sample of a slice: the world position (mesher space) and the field value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SliceSample {
    pub position: DVec3,
    pub value: f32,
}

/// The active voxels of the plane `axis = index` (0, 1, 2 for x, y, z) in lattice indices.
pub fn slice_samples(volume: &Volume, axis: usize, index: i32) -> Vec<SliceSample> {
    volume
        .grid
        .active_voxels()
        .into_iter()
        .filter(|(i, _)| i.to_array()[axis] == index)
        .map(|(i, value)| SliceSample {
            position: volume
                .to_parent
                .transform_point3(volume.grid.index_to_world(i)),
            value,
        })
        .collect()
}

/// The lattice index range `(min, max)` of the active voxels along `axis`.
pub fn slice_range(volume: &Volume, axis: usize) -> Option<(i32, i32)> {
    let (lo, hi): (IVec3, IVec3) = volume.grid.bounds()?;
    Some((lo.to_array()[axis], hi.to_array()[axis]))
}

/// How a preview computation is made (a closure so a test can slow it down).
pub type PreviewFn =
    dyn Fn(&Scene, NodeId) -> Result<Option<Preview>, VolumeBuildError> + Send + Sync;

/// What the viewport reads each frame.
#[derive(Debug, Clone)]
pub struct PreviewSnapshot {
    /// The newest finished preview; kept while a newer one is computing.
    pub preview: Option<Arc<Preview>>,
    /// The error of the newest finished computation, if it failed.
    pub error: Option<String>,
    /// A computation is running or queued.
    pub busy: bool,
    /// Requests made so far.
    pub requested: u64,
    /// Computations finished so far.
    pub finished: u64,
}

struct Shared {
    pending: Option<(Scene, NodeId)>,
    running: bool,
    preview: Option<Arc<Preview>>,
    error: Option<String>,
    requested: u64,
    finished: u64,
}

/// Computes previews on a worker thread. Requests coalesce: while one runs, only the newest
/// later request is kept; `snapshot` never waits (VOLUME.md section 11.2).
pub struct PreviewWorker {
    shared: Arc<(Mutex<Shared>, Condvar)>,
    compute: Arc<PreviewFn>,
}

impl PreviewWorker {
    pub fn new(compute: Arc<PreviewFn>) -> PreviewWorker {
        PreviewWorker {
            shared: Arc::new((
                Mutex::new(Shared {
                    pending: None,
                    running: false,
                    preview: None,
                    error: None,
                    requested: 0,
                    finished: 0,
                }),
                Condvar::new(),
            )),
            compute,
        }
    }

    /// A worker that builds the real preview at `max_cells`.
    pub fn standard(max_cells: u32) -> PreviewWorker {
        PreviewWorker::new(Arc::new(move |scene, mesher| {
            build_preview(scene, mesher, &EvalCtx::default(), max_cells)
        }))
    }

    /// Queues a computation for `mesher` in `scene` and returns at once.
    pub fn request(&self, scene: &Scene, mesher: NodeId) {
        let (lock, _) = &*self.shared;
        let mut s = lock.lock().expect("preview state");
        s.requested += 1;
        s.pending = Some((scene.clone(), mesher));
        if s.running {
            return;
        }
        s.running = true;
        let shared = Arc::clone(&self.shared);
        let compute = Arc::clone(&self.compute);
        std::thread::spawn(move || run(&shared, compute.as_ref()));
    }

    pub fn snapshot(&self) -> PreviewSnapshot {
        let (lock, _) = &*self.shared;
        let s = lock.lock().expect("preview state");
        PreviewSnapshot {
            preview: s.preview.clone(),
            error: s.error.clone(),
            busy: s.running || s.pending.is_some(),
            requested: s.requested,
            finished: s.finished,
        }
    }

    /// Waits until no computation is running or queued; `false` on timeout.
    pub fn wait_idle(&self, timeout: Duration) -> bool {
        let (lock, cv) = &*self.shared;
        let s = lock.lock().expect("preview state");
        let (s, _) = cv
            .wait_timeout_while(s, timeout, |s| s.running || s.pending.is_some())
            .expect("preview state");
        !(s.running || s.pending.is_some())
    }
}

fn run(shared: &(Mutex<Shared>, Condvar), compute: &PreviewFn) {
    let (lock, cv) = shared;
    loop {
        let job = {
            let mut s = lock.lock().expect("preview state");
            match s.pending.take() {
                Some(j) => j,
                None => {
                    s.running = false;
                    cv.notify_all();
                    return;
                }
            }
        };
        let out = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| compute(&job.0, job.1)))
            .unwrap_or_else(|_| Err(VolumeBuildError::Failed("the preview panicked".into())));
        let mut s = lock.lock().expect("preview state");
        s.finished += 1;
        match out {
            Ok(p) => {
                s.preview = p.map(Arc::new);
                s.error = None;
            }
            // the last good preview stays on screen
            Err(e) => s.error = Some(e.to_string()),
        }
        cv.notify_all();
    }
}
