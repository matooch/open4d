#[path = "../benches/s10/support.rs"]
mod support;

use forge_render_rt::ray::BackendKind;
use std::collections::BTreeMap;
use support::{BaselineError, Identity, Run, fixture, regression_check, settings};

#[test]
fn s10_snapshot_contains_the_full_quad_grid_and_shared_clones() {
    let scene = fixture();
    assert_eq!(scene.meshes.len(), 2);
    assert_eq!(scene.meshes[0].positions.len(), 1_002_001);
    assert_eq!(scene.meshes[0].triangles.len(), 2_000_000);
    // Each consecutive triangle pair splits a quad along the same diagonal.
    for pair in scene.meshes[0].triangles.as_chunks::<2>().0 {
        assert_eq!(pair[0][0], pair[1][0]);
        assert_eq!(pair[0][2], pair[1][1]);
        assert_ne!(pair[0][1], pair[1][2]);
    }
    assert_eq!(scene.meshes[1].triangles.len(), 12);
    assert_eq!(scene.instances.len(), 10_001);
    assert_eq!(scene.instances[0].mesh, 0);
    for (i, instance) in scene.instances[1..].iter().enumerate() {
        assert_eq!(instance.mesh, 1);
        assert_eq!(instance.object_id as usize, i + 2);
        assert_eq!(instance.transform[12], (i % 100) as f64 * 4.0 - 198.0);
        assert_eq!(instance.transform[14], (i / 100) as f64 * 4.0 - 198.0);
    }
    assert_eq!(scene.lights.len(), 1);
}

#[test]
fn s10_times_complete_1080p_frames_and_all_hybrid_effects() {
    for hybrid in [false, true] {
        let settings = settings(hybrid);
        assert_eq!((settings.width, settings.height), (1920, 1080));
        assert_eq!(settings.region, None);
        assert!(settings.aovs.is_empty());
        assert!(!settings.post.taa);
        assert_eq!(settings.hybrid.enabled, hybrid);
        assert_eq!(settings.hybrid.shadows, hybrid);
        assert_eq!(settings.hybrid.reflections, hybrid);
        assert_eq!(settings.hybrid.ao, hybrid);
        assert_eq!(settings.hybrid.gi, hybrid);
    }
}

fn run() -> Run {
    Run {
        identity: Identity::new("lab", "GPU", "Metal", Some(BackendKind::Metal), true),
        median_ns: BTreeMap::from([
            ("frame".into(), 1_000),
            ("g-buffer".into(), 100),
            ("rt-ao".into(), 100),
        ]),
    }
}

#[test]
fn ten_percent_is_allowed_but_one_more_nanosecond_is_a_regression() {
    let baseline = run();
    let mut current = baseline.clone();
    current.median_ns.insert("frame".into(), 1_100);
    current.median_ns.insert("rt-ao".into(), 110);
    assert_eq!(regression_check(&baseline, &current), Ok(()));

    current.median_ns.insert("frame".into(), 1_101);
    current.median_ns.insert("rt-ao".into(), 111);
    assert_eq!(
        regression_check(&baseline, &current),
        Err(BaselineError::Regressed(vec![
            "frame".into(),
            "rt-ao".into()
        ]))
    );
}

#[test]
fn skipped_hybrid_primary_compares_as_zero_without_allowing_missing_passes() {
    let mut baseline = run();
    baseline.median_ns.insert("hybrid-primary".into(), 100);
    let mut skipped = baseline.clone();
    skipped.median_ns.insert("hybrid-primary".into(), 0);
    assert_eq!(regression_check(&baseline, &skipped), Ok(()));
    assert_eq!(regression_check(&skipped, &skipped), Ok(()));
    assert_eq!(
        regression_check(&skipped, &baseline),
        Err(BaselineError::Regressed(vec!["hybrid-primary".into()]))
    );
    skipped.median_ns.remove("hybrid-primary");
    assert_eq!(
        regression_check(&baseline, &skipped),
        Err(BaselineError::Missing("hybrid-primary".into()))
    );
    skipped.median_ns.insert("g-buffer".into(), 0);
    assert_eq!(
        regression_check(&baseline, &skipped),
        Err(BaselineError::Zero("g-buffer".into()))
    );
}

#[test]
fn missing_pass_and_mismatched_workload_identity_are_errors() {
    let baseline = run();
    let mut current = baseline.clone();
    current.median_ns.remove("g-buffer");
    assert_eq!(
        regression_check(&baseline, &current),
        Err(BaselineError::Missing("g-buffer".into()))
    );
    for identity in [
        Identity::new("other-host", "GPU", "Metal", Some(BackendKind::Metal), true),
        Identity::new("lab", "other GPU", "Metal", Some(BackendKind::Metal), true),
        Identity::new("lab", "GPU", "Vulkan", Some(BackendKind::Metal), true),
        Identity::new("lab", "GPU", "Metal", Some(BackendKind::Compute), true),
        Identity::new("lab", "GPU", "Metal", Some(BackendKind::Metal), false),
    ] {
        current.identity = identity;
        assert_eq!(
            regression_check(&baseline, &current),
            Err(BaselineError::Incompatible)
        );
    }
}
