# Attribute manager tabs and groups — observation (O64)

Status: not observed · App version: TODO · OS: TODO · Observer: TODO · Date: TODO

Why: owner note (2026-11-26, GUI-SESSIONS.md): "not every setting should be at the same level;
the reference app uses tabs to divide groups of settings." Brief 231 assigns our parameters to
tabs and groups from this table.

## How to observe (scripted, no GUI needed)
For each object type below, create one with defaults, then walk its parameter description
through the scripting API, in the order the app returns it. For every parameter, record the
**tab** it sits on (the top-level group shown as a tab), the **group** inside the tab (if any,
including nested groups as `Group > Subgroup`), its **label**, and its **position** (order within
the group). Also record parameters that are hidden by default, as `hidden`. Record labels as the
app shows them. Write one table per type. Use our key where it is obvious from the existing forms
(for example `cube.md`), otherwise `?`.

Never read resource, description or layout files from disk: only what the running app returns
through its scripting API (OBSERVER.md §0). If the API doesn't expose the tab of a parameter,
write "not observed: <reason>" for that type and move on.

## Types
Cube, Sphere, Cylinder, Plane, Text (spline), Circle (spline), Extrude, Sweep, Lofts, Subdivision
Surface, Symmetry, Boole, Null, Instance, Connect, Bend, Twist, Taper, FFD, Displacer, Cloner,
Matrix, Fracture, Voronoi Fracture, Plain effector, Random effector, Shader effector, Step
effector, Linear field, Spherical field, Random field, Camera, Light (omni and area), Volume
Builder, Volume Mesher, Joint, Skin, Rigid Body tag, Phong tag, Texture tag, Standard material.

## Per type

### <Type>

| Tab | Group | # | Label | Our key | Notes |
|---|---|---|---|---|---|
| TODO | | | | | |

## Behaviour notes
How many tabs a simple object shows; whether some tabs are shared by every object (transform,
display, basic); whether a tab appears only with an option on. TODO
