//! docs/specs/CORE.md §2.

mod common;

use forge_core::{Category, NodeType, ParamDesc, Plugin, Registry, RegistryError};

fn reg_one(t: NodeType) -> Result<(), RegistryError> {
    Registry::new().register(t)
}

#[test]
fn builtins_contain_null() {
    let r = Registry::with_builtins();
    let null = r.get("forge.null").expect("null registered");
    assert_eq!(null.category, Category::Object);
    assert_eq!(null.label, "Null");
    assert_eq!(null.params.len(), 3, "transform params only");
    assert!(Registry::new().get("forge.null").is_none());
}

#[test]
fn type_ids_need_a_namespace_and_valid_chars() {
    for bad in [
        "null",
        "",
        ".null",
        "forge.",
        "forge..null",
        "Forge.null",
        "forge.nu ll",
    ] {
        assert!(
            matches!(
                reg_one(NodeType::new(bad, "X", Category::Object)),
                Err(RegistryError::InvalidTypeId(_))
            ),
            "{bad:?} should be rejected"
        );
    }
    for good in ["forge.null", "acme.tools.box-2", "a.b_c"] {
        assert_eq!(reg_one(NodeType::new(good, "X", Category::Object)), Ok(()));
    }
}

#[test]
fn duplicate_types_are_rejected() {
    let mut r = Registry::with_builtins();
    assert!(matches!(
        r.register(NodeType::new("forge.null", "Again", Category::Object)),
        Err(RegistryError::DuplicateType(_))
    ));
}

#[test]
fn param_ids_and_defaults_are_checked() {
    let t = |p: ParamDesc| NodeType::new("a.t", "T", Category::Object).param(p);
    assert!(matches!(
        reg_one(t(ParamDesc::bool("Bad", "B", true))),
        Err(RegistryError::InvalidParamId { .. })
    ));
    assert!(matches!(
        reg_one(t(ParamDesc::bool("", "B", true))),
        Err(RegistryError::InvalidParamId { .. })
    ));
    assert!(matches!(
        reg_one(t(ParamDesc::float("f", "F", 5.0).range(0.0, 1.0))),
        Err(RegistryError::InvalidDefault { .. })
    ));
    assert!(matches!(
        reg_one(t(ParamDesc::float("f", "F", f64::NAN))),
        Err(RegistryError::InvalidDefault { .. })
    ));
    assert!(matches!(
        reg_one(t(ParamDesc::enumeration("e", "E", "zz", &[("a", "A")]))),
        Err(RegistryError::InvalidDefault { .. })
    ));
    assert!(matches!(
        reg_one(t(ParamDesc::enumeration(
            "e",
            "E",
            "a",
            &[("a", "A"), ("a", "B")]
        ))),
        Err(RegistryError::InvalidEnum { .. })
    ));
    assert!(matches!(
        reg_one(t(ParamDesc::enumeration("e", "E", "a", &[]))),
        Err(RegistryError::InvalidEnum { .. })
    ));
    let dup = NodeType::new("a.t", "T", Category::Object)
        .param(ParamDesc::bool("x", "X", true))
        .param(ParamDesc::int("x", "X2", 1));
    assert!(matches!(
        reg_one(dup),
        Err(RegistryError::DuplicateParam { .. })
    ));
    assert_eq!(reg_one(t(ParamDesc::float("radius.x", "R", 0.5))), Ok(()));
}

struct HalfBroken;

impl Plugin for HalfBroken {
    fn id(&self) -> &str {
        "broken"
    }
    fn register(&self, r: &mut Registry) -> Result<(), RegistryError> {
        r.register(NodeType::new("broken.ok", "Ok", Category::Object))?;
        r.register(NodeType::new("broken", "Bad id", Category::Object))
    }
}

#[test]
fn plugin_install_is_atomic() {
    let mut r = Registry::with_builtins();
    assert!(r.install(&HalfBroken).is_err());
    assert!(r.get("broken.ok").is_none());
    assert!(r.get("forge.null").is_some());
    r.install(&common::TestPlugin).unwrap();
    assert!(r.get("test.box").is_some());
}

#[test]
fn listing_is_sorted_and_filterable() {
    let r = common::registry();
    let ids: Vec<&str> = r.types().map(|t| t.id.as_str()).collect();
    assert_eq!(
        ids,
        [
            "forge.null",
            "forge.smoothing",
            "test.bend",
            "test.box",
            "test.mat",
            "test.sel"
        ]
    );
    let tags: Vec<&str> = r.types_in(Category::Tag).map(|t| t.id.as_str()).collect();
    assert_eq!(tags, ["forge.smoothing", "test.sel"]);
    let bx = r.get("test.box").unwrap();
    assert_eq!(bx.find_param("segments").unwrap().label, "Segments");
    assert!(bx.find_param("nope").is_none());
}

#[test]
fn categories_know_where_they_live() {
    for c in [
        Category::Object,
        Category::Generator,
        Category::Deformer,
        Category::Effector,
        Category::Field,
    ] {
        assert!(c.is_hierarchy(), "{c:?}");
    }
    for c in [
        Category::Tag,
        Category::Material,
        Category::Shader,
        Category::RenderEngine,
    ] {
        assert!(!c.is_hierarchy(), "{c:?}");
    }
}
