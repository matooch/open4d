// R24.7 raster receivers. The raster key is one-based; zero is clear.
fn receiver_instance_end(instance: u32) -> u32 {
    return bitcast<u32>(path_attributes[instance * 7u + 4u].w);
}

// An invalid key returns (instance_count, 0), which must not index the table.
fn receiver_triangle(key: u32, instance_count: u32) -> vec2<u32> {
    if key == 0u {
        return vec2<u32>(instance_count, 0u);
    }
    let index = key - 1u;
    var lo = 0u;
    var hi = instance_count;
    while lo < hi {
        let mid = lo + (hi - lo) / 2u;
        if receiver_instance_end(mid) <= index {
            lo = mid + 1u;
        } else {
            hi = mid;
        }
    }
    if lo == instance_count {
        return vec2<u32>(instance_count, 0u);
    }
    var start = 0u;
    if lo != 0u {
        start = receiver_instance_end(lo - 1u);
    }
    return vec2<u32>(lo, index - start);
}

fn receiver_geometry(p: vec3<f32>, ids: vec2<u32>) -> Geom {
    let inst = ids.x * 7u;
    let tri = bitcast<u32>(path_attributes[inst].x) + ids.y * 10u;
    let origin = vec3<f32>(
        path_attributes[inst + 1u].w,
        path_attributes[inst + 2u].w,
        path_attributes[inst + 3u].w,
    );
    // Rows of the inverse-transpose are columns of the inverse linear map.
    let to_object = mat3x3<f32>(
        path_attributes[inst + 4u].xyz,
        path_attributes[inst + 5u].xyz,
        path_attributes[inst + 6u].xyz,
    );
    let a = path_attributes[tri].xyz;
    let e1 = path_attributes[tri + 1u].xyz - a;
    let e2 = path_attributes[tri + 2u].xyz - a;
    let q = to_object * (p - origin) - a;
    let aa = dot(e1, e1);
    let ab = dot(e1, e2);
    let bb = dot(e2, e2);
    let qa = dot(q, e1);
    let qb = dot(q, e2);
    let det = aa * bb - ab * ab;
    var bary = vec2<f32>(0.0);
    if det > 0.0 {
        bary = vec2<f32>(bb * qa - ab * qb, aa * qb - ab * qa) / det;
    }
    return instance_geometry(p, ids, bary);
}

// frame = (focal length in pixels, principal x, principal y). Zero focal
// length denotes orthographic, which uses the same view vector at every pixel.
fn receiver_view(pixel: vec2<f32>, jitter: vec2<f32>, frame: vec3<f32>, camera: mat3x3<f32>) -> vec3<f32> {
    if frame.x == 0.0 {
        return -normalize(camera[2]);
    }
    let xy = (pixel + vec2<f32>(0.5) - jitter - frame.yz) / frame.x;
    return -normalize(camera * vec3<f32>(xy.x, -xy.y, 1.0));
}
