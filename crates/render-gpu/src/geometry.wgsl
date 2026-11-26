// Raw surface geometry shared by R20 transport and R24 receivers.
struct Geom {
    p: vec3<f32>,
    ng: vec3<f32>,
    ns: vec3<f32>,
    uvw: vec3<f32>,
    closed: bool,
    corners: array<vec3<f32>, 3>,
    corner_uvw: array<vec3<f32>, 3>,
};
