mod common;
use common::*;
use forge_image::*;

fn layer(name: Option<&str>, w: usize, h: usize, chans: &[&str], seed: u64) -> ExrLayer {
    let mut rng = Lcg(seed);
    ExrLayer {
        name: name.map(String::from),
        width: w,
        height: h,
        channels: chans
            .iter()
            .map(|c| ExrChannel {
                name: c.to_string(),
                samples: (0..w * h).map(|_| rng.next_f32() * 10.0).collect(),
            })
            .collect(),
    }
}

fn find<'a>(ls: &'a [ExrLayer], name: &str) -> &'a ExrLayer {
    ls.iter()
        .find(|l| l.name.as_deref() == Some(name))
        .unwrap_or_else(|| panic!("no layer {name}"))
}

fn chan<'a>(l: &'a ExrLayer, name: &str) -> &'a [f32] {
    &l.channels.iter().find(|c| c.name == name).unwrap().samples
}

#[test]
fn multipart_write_then_multi_layer_read_f32_exact() {
    let beauty = layer(Some("beauty"), 6, 4, &["R", "G", "B", "A"], 1);
    let depth = layer(Some("depth"), 6, 4, &["Z"], 2);
    let diffuse = layer(Some("diffuse"), 3, 2, &["R", "G", "B"], 3);
    let bytes = encode_exr_layers(
        &[beauty.clone(), depth.clone(), diffuse.clone()],
        ExrPrecision::F32,
    )
    .unwrap();
    let got = decode_exr_layers(&bytes, &Limits::default()).unwrap();
    assert_eq!(got.len(), 3);
    for want in [&beauty, &depth, &diffuse] {
        let l = find(&got, want.name.as_deref().unwrap());
        assert_eq!((l.width, l.height), (want.width, want.height));
        assert_eq!(l.channels.len(), want.channels.len());
        for c in &want.channels {
            assert_eq!(
                chan(l, &c.name),
                &c.samples[..],
                "{:?}/{}",
                want.name,
                c.name
            );
        }
    }
    // to_image: RGBA from beauty, grey from depth, RGB (alpha 1) from diffuse
    let b = find(&got, "beauty").to_image().unwrap();
    assert_eq!(b.alpha, AlphaMode::Premultiplied);
    assert_eq!(b.pixel(2, 1)[3], chan(find(&got, "beauty"), "A")[6 + 2]);
    let d = find(&got, "depth").to_image().unwrap();
    let z = d.pixel(1, 1);
    assert_eq!(z[0], z[1]);
    assert_eq!(z[0], chan(find(&got, "depth"), "Z")[6 + 1]);
    assert_eq!(z[3], 1.0);
    let df = find(&got, "diffuse").to_image().unwrap();
    assert_eq!((df.width, df.height), (3, 2));
    assert_eq!(df.pixel(0, 0)[3], 1.0);
    // decode() picks the beauty layer
    let dec = decode(&bytes, &Limits::default()).unwrap();
    assert_eq!((dec.image.width, dec.image.height), (6, 4));
    assert_eq!(dec.info.layers, 3);
    assert_eq!(dec.image.data, b.data);
}

#[test]
fn f16_precision() {
    let l = layer(Some("a"), 5, 5, &["R", "G", "B"], 4);
    let bytes = encode_exr_layers(std::slice::from_ref(&l), ExrPrecision::F16).unwrap();
    let got = decode_exr_layers(&bytes, &Limits::default()).unwrap();
    for c in &l.channels {
        for (a, b) in c.samples.iter().zip(chan(&got[0], &c.name)) {
            assert!((a - b).abs() <= a.abs() * 1.0 / 1024.0 + 1e-6, "{a} {b}");
        }
    }
}

#[test]
fn legacy_dotted_channels_split_into_layers() {
    let mut l = layer(None, 4, 3, &["R", "G", "B"], 5);
    let spec = layer(None, 4, 3, &["spec.R", "spec.G", "spec.B"], 6);
    let ao = layer(None, 4, 3, &["ao.Y"], 7);
    l.channels.extend(spec.channels.clone());
    l.channels.extend(ao.channels.clone());
    let bytes = encode_exr_layers(&[l.clone()], ExrPrecision::F32).unwrap();
    let got = decode_exr_layers(&bytes, &Limits::default()).unwrap();
    assert_eq!(got.len(), 3);
    assert!(got[0].name.is_none());
    assert_eq!(got[0].channels.len(), 3);
    let s = find(&got, "spec");
    assert_eq!(chan(s, "R"), &spec.channels[0].samples[..]);
    let a = find(&got, "ao");
    assert_eq!(a.channels[0].name, "Y");
    assert_eq!(
        a.to_image().unwrap().pixel(0, 0)[0],
        ao.channels[0].samples[0]
    );
}

#[test]
fn writer_rejects_bad_input() {
    let a = layer(Some("x"), 2, 2, &["R"], 1);
    let b = layer(Some("x"), 2, 2, &["R"], 2);
    assert!(matches!(
        encode_exr_layers(&[a.clone(), b], ExrPrecision::F32),
        Err(ImageError::InvalidInput(_))
    ));
    let unnamed = layer(None, 2, 2, &["R"], 1);
    assert!(encode_exr_layers(&[a.clone(), unnamed], ExrPrecision::F32).is_err());
    assert!(encode_exr_layers(&[], ExrPrecision::F32).is_err());
    let mut bad = a.clone();
    bad.channels[0].samples.pop();
    assert!(encode_exr_layers(&[bad], ExrPrecision::F32).is_err());
    let mut empty = a.clone();
    empty.channels.clear();
    assert!(encode_exr_layers(&[empty], ExrPrecision::F32).is_err());
    // a single unnamed layer is fine
    assert!(encode_exr_layers(&[layer(None, 2, 2, &["R"], 1)], ExrPrecision::F32).is_ok());
}

#[test]
fn layer_without_colour_channels_has_no_image() {
    let l = ExrLayer {
        name: Some("m".into()),
        width: 1,
        height: 1,
        channels: vec![],
    };
    assert!(l.to_image().is_none());
}
