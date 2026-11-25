use forge_render_rt::shader::assemble;
use naga::back::hlsl;
use naga::valid::{Capabilities, ValidationFlags, Validator};
#[test]
fn dump() {
    let src = assemble(&[]).unwrap().source;
    let module = naga::front::wgsl::parse_str(&src).unwrap();
    let info = Validator::new(ValidationFlags::all(), Capabilities::all())
        .validate(&module)
        .unwrap();
    let mut out = String::new();
    hlsl::Writer::new(
        &mut out,
        &hlsl::Options::default(),
        &hlsl::PipelineOptions::default(),
    )
    .write(&module, &info, None)
    .unwrap();
    std::fs::write("/tmp/claude-0/rt.hlsl", out).unwrap();
}
