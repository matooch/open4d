# R25 defaults and ranges — observation (O4)

Status: not observed · App version: TODO · OS: TODO · Observer: TODO · Date: TODO
Method: scripted ([agents/OBSERVER.md](../agents/OBSERVER.md)); objects created **through
their menu command** (O0 finding). Unblocks: brief 07 (switch our defaults to R25).

O1 showed R25 builds the same geometry as R20 but several defaults moved (sphere
segments, cylinder/torus segments and radii, extrude offset, boole type and hide new
edges). This form records every parameter of every feature we already implement, so our
defaults can follow R25 and our tests can keep the R20 defaults for R20 cases.

## How to fill it

For each feature: create the object with its menu command in a new document, read every
parameter we have a key for (the keys are in the feature's existing observation file
`docs/parity/<feature>.md`, "Our key" column), then type a huge and a negative value into
each numeric one and read back what it clamps to. Copy the R20 default from the existing
observation file. **Mark rows where R20 ≠ R25 with `!`** in the first column. Labels: only
where the label changed since R20.

Table format, one per feature (`<feature>` as in `tests/golden/<feature>/`):

| ! | Our key | R20 default | R25 default | R25 min | R25 max | R25 label if changed |
|---|---|---|---|---|---|---|

## Features

Primitives: plane, cube, sphere, cylinder, cone, disc, torus, tube, capsule, pyramid.
Splines: circle, rectangle, helix (docs/parity/splines.md).
Generators: subdivision, boole, symmetry, array, connect, instance, extrude, lathe, loft,
sweep. Deformers: bend, twist, taper, bulge, shear, spherify (and the common box/mode
params from deformer-common.md). Tags: smoothing (angle).

TODO: one table per feature above.

## New-object parameters not in R20

Parameters an R25 object shows that the R20 observation didn't have (label, default,
what it seems to do). We don't implement them yet; list them so a later brief can:
TODO

## Notes
TODO
