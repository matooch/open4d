//! docs/specs/NODES.md section 2: port types and conversions.

use forge_core::NodeId;
use forge_nodes::{
    ConvertError, PortType as T, PortValue as V, can_convert, convert, convert_explicit,
};
use serde_json::json;

/// The implicit conversions between two concrete types, from the spec table.
const IMPLICIT: &[(T, T)] = &[
    (T::Int, T::Float),
    (T::Bool, T::Int),
    (T::Bool, T::Float),
    (T::Float, T::Vec3),
    (T::Int, T::Vec3),
    (T::Float, T::Colour),
    (T::Int, T::Colour),
    (T::Vec3, T::Colour),
    (T::Colour, T::Vec3),
];

#[test]
fn the_implicit_table_is_exactly_the_spec() {
    for from in T::ALL {
        for to in T::ALL {
            let expected =
                from == to || from == T::Any || to == T::Any || IMPLICIT.contains(&(from, to));
            assert_eq!(can_convert(from, to), expected, "{from} -> {to}");
        }
    }
}

#[test]
fn implicit_conversions_of_values() {
    assert_eq!(convert(&V::Int(3), T::Float), Ok(V::Float(3.0)));
    assert_eq!(convert(&V::Bool(true), T::Int), Ok(V::Int(1)));
    assert_eq!(convert(&V::Bool(false), T::Float), Ok(V::Float(0.0)));
    assert_eq!(convert(&V::Float(2.0), T::Vec3), Ok(V::Vec3([2.0; 3])));
    assert_eq!(convert(&V::Int(2), T::Colour), Ok(V::Colour([2.0; 3])));
    assert_eq!(
        convert(&V::Vec3([1.0, 2.0, 3.0]), T::Colour),
        Ok(V::Colour([1.0, 2.0, 3.0]))
    );
    assert_eq!(
        convert(&V::Colour([0.1, 0.2, 0.3]), T::Vec3),
        Ok(V::Vec3([0.1, 0.2, 0.3]))
    );
    // identity and Any
    assert_eq!(
        convert(&V::String("x".into()), T::Any),
        Ok(V::String("x".into()))
    );
    assert_eq!(
        convert(&V::Node(Some(NodeId(4))), T::Node),
        Ok(V::Node(Some(NodeId(4))))
    );
}

#[test]
fn lossy_and_unrelated_conversions_are_not_implicit() {
    for (v, to) in [
        (V::Float(1.5), T::Int),
        (V::Float(1.0), T::Bool),
        (V::Int(1), T::Bool),
        (V::Vec3([1.0; 3]), T::Float),
        (V::String("1".into()), T::Float),
        (V::Float(1.0), T::String),
        (V::Float(1.0), T::Matrix),
        (V::Vec3([0.0; 3]), T::Matrix),
        (V::Node(None), T::Int),
    ] {
        assert!(
            matches!(convert(&v, to), Err(ConvertError::Unsupported { .. })),
            "{v:?} -> {to}"
        );
    }
}

#[test]
fn explicit_conversions() {
    assert_eq!(convert_explicit(&V::Float(2.9), T::Int), Ok(V::Int(2)));
    assert_eq!(convert_explicit(&V::Float(-2.9), T::Int), Ok(V::Int(-2)));
    assert!(convert_explicit(&V::Float(f64::NAN), T::Int).is_err());
    assert_eq!(
        convert_explicit(&V::Float(0.0), T::Bool),
        Ok(V::Bool(false))
    );
    assert_eq!(
        convert_explicit(&V::Float(-0.5), T::Bool),
        Ok(V::Bool(true))
    );
    assert_eq!(convert_explicit(&V::Int(0), T::Bool), Ok(V::Bool(false)));
    assert_eq!(
        convert_explicit(&V::Vec3([3.0, 4.0, 0.0]), T::Float),
        Ok(V::Float(5.0))
    );
    let V::Float(l) = convert_explicit(&V::Colour([1.0, 1.0, 1.0]), T::Float).unwrap() else {
        panic!()
    };
    assert!((l - 1.0).abs() < 1e-12);
    let V::Matrix(m) = convert_explicit(&V::Vec3([1.0, 2.0, 3.0]), T::Matrix).unwrap() else {
        panic!()
    };
    assert_eq!(&m[12..], &[1.0, 2.0, 3.0, 1.0]);
    assert_eq!(
        convert_explicit(&V::Matrix(m), T::Vec3),
        Ok(V::Vec3([1.0, 2.0, 3.0]))
    );
    assert_eq!(
        convert_explicit(&V::Int(7), T::String),
        Ok(V::String("7".into()))
    );
    assert_eq!(
        convert_explicit(&V::Bool(true), T::String),
        Ok(V::String("true".into()))
    );
    assert_eq!(
        convert_explicit(&V::String(" 2.5 ".into()), T::Float),
        Ok(V::Float(2.5))
    );
    assert_eq!(
        convert_explicit(&V::String("12".into()), T::Int),
        Ok(V::Int(12))
    );
    assert_eq!(
        convert_explicit(&V::String("true".into()), T::Bool),
        Ok(V::Bool(true))
    );
    assert!(convert_explicit(&V::String("nope".into()), T::Float).is_err());
    assert!(convert_explicit(&V::String("1.5".into()), T::Int).is_err());
    // implicit ones still work through the explicit entry point
    assert_eq!(convert_explicit(&V::Int(2), T::Float), Ok(V::Float(2.0)));
    // some pairs have no conversion at all
    assert!(convert_explicit(&V::Matrix([0.0; 16]), T::Bool).is_err());
    assert!(convert_explicit(&V::Node(None), T::Float).is_err());
}

#[test]
fn json_values_follow_the_port_type() {
    assert_eq!(V::from_json(T::Float, &json!(2)), Ok(V::Float(2.0)));
    assert_eq!(V::from_json(T::Int, &json!(2)), Ok(V::Int(2)));
    assert!(V::from_json(T::Int, &json!(2.5)).is_err());
    assert_eq!(V::from_json(T::Bool, &json!(true)), Ok(V::Bool(true)));
    assert_eq!(
        V::from_json(T::Vec3, &json!([1, 2, 3])),
        Ok(V::Vec3([1.0, 2.0, 3.0]))
    );
    assert!(V::from_json(T::Vec3, &json!([1, 2])).is_err());
    assert_eq!(V::from_json(T::Node, &json!(null)), Ok(V::Node(None)));
    assert_eq!(
        V::from_json(T::Node, &json!(9)),
        Ok(V::Node(Some(NodeId(9))))
    );
    assert!(V::from_json(T::Matrix, &json!([1, 2])).is_err());
    // Any picks the natural type
    assert_eq!(V::from_json(T::Any, &json!(2)), Ok(V::Int(2)));
    assert_eq!(V::from_json(T::Any, &json!(2.5)), Ok(V::Float(2.5)));
    assert_eq!(
        V::from_json(T::Any, &json!([1, 2, 3])),
        Ok(V::Vec3([1.0, 2.0, 3.0]))
    );
    assert_eq!(V::from_json(T::Any, &json!("s")), Ok(V::String("s".into())));
    assert!(V::from_json(T::Any, &json!({"a": 1})).is_err());
    // values round trip through JSON for the concrete types
    for v in [
        V::Float(0.25),
        V::Int(-3),
        V::Bool(true),
        V::Vec3([1.0, -2.0, 0.5]),
        V::String("hi".into()),
        V::Node(Some(NodeId(5))),
    ] {
        let back = V::from_json(v.ty(), &v.to_json());
        assert_eq!(back, Ok(v));
    }
}

#[test]
fn zero_values_have_their_type() {
    for ty in T::ALL {
        if ty != T::Any {
            assert_eq!(V::zero(ty).ty(), ty);
        }
    }
}
