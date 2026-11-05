# Deformers and fields

## Deformers

A deformer bends the geometry of the object it belongs to. Available deformers:

| Type id | Effect |
|---|---|
| `forge.bend` | Bends along the box's height |
| `forge.twist` | Rotates slices progressively |
| `forge.taper` | Scales slices progressively |
| `forge.bulge` | Swells or pinches |
| `forge.shear` | Slides slices sideways |
| `forge.spherify` | Blends toward a sphere |
| `forge.squash_stretch` | Squashes and stretches |
| `forge.wrap` | Wraps a flat object around a shape |

### Where a deformer applies

Put a deformer **under** the object. It acts on that object's mesh after the object has
built its geometry. Under an object with no geometry (a null) it acts on the null's mesh
children. A disabled deformer does nothing. Several deformers under one object apply in
tree order, top first.

### The box and the mode

Each deformer has a box (`size`) centred on the deformer's own origin. Move, rotate or scale
the deformer to move its effect. The *mode* decides what happens outside the box along its
height:

- **Limited** (default): below the box nothing changes, inside it follows the deformation,
  above it the points travel rigidly with the end of the deformation.
- **Within box**: only points inside the box on all three axes are affected.
- **Unlimited**: the same rate of deformation continues below and above the box.

## Fields

A field is an object that gives a value between 0 and 1 (or beyond, after remapping) at
every point in space. Deformers and the cloner use fields to decide how strongly to act at
each point.

| Type id | Shape |
|---|---|
| `forge.field.linear` | Gradient along a direction |
| `forge.field.spherical` | Sphere |
| `forge.field.box` | Box |
| further field shapes | cylinder, cone, torus, capsule, radial, random |

Every field has a **remapping** section: strength, invert, inner offset, min and max,
clamping, a multiplier and a contour mode (none, quadratic, step, quantize, curve). A
*field list* combines several fields, each with its own layer strength.
