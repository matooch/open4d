# Effectors III — observation (O22)

Status: not observed · App version: TODO · OS: TODO · Observer: TODO · Date: TODO
Method: exactly as [effectors-2.md](effectors-2.md) (O14):
- Linear cloner, count 5, P per step (0, 50, 0), with a 20 cm cube child.
- The effector is made through its menu command with the cloner selected.
- Save the clone data and `mesh.obj` to `tests/golden/cloner/<case>/`, with
  `observation = "docs/parity/effectors-3.md"`.

Unblocks: P3 effectors in depth.

## Parameters (fill once per new effector, as in effectors-2.md)

### Random: list the Effector tab's Random Mode options and their settings
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|

### Shader
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|

### Spline
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|

### Push Apart
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|

### Inheritance
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|

### Volume
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|

## Cases

| Case | Effector | Changed from default | Notes |
|---|---|---|---|
| random-gaussian | Random | Random Mode Gaussian (or the second option) | |
| random-noise | Random | Random Mode Noise | |
| random-indexed | Random | Random Mode Indexed (if it exists) | |
| random-scale | Random | position off, scale on, uniform, 1 | |
| shader-noise | Shader | its shader set to a default Noise shader; Position on, P (0, 100, 0) | Record the noise seed and type |
| spline-helix | Spline | spline: a default Helix | |
| spline-helix-segment | Spline | the same, Mode Step (or the second option) | |
| push-apart | Push Apart | the cloner set to Grid Array 3×3×3 with size (30, 30, 30) instead of Linear | |
| push-apart-iterations | Push Apart | the same, iterations 5 | |
| inheritance | Inheritance | a second, Radial cloner (count 5) as the object to inherit from | Record both cloners' setup |
| volume | Volume | a default cube scaled to 100 as the volume object; Position on, P (0, 100, 0) | |
| plain-field-spherical | Plain, P (0, 100, 0) | a Spherical field (default) in its field list, at (0, 100, 0) | |
| plain-field-linear | Plain, P (0, 100, 0) | a Linear field (default, direction Y+) in its field list | |
| plain-absolute | Plain | Transform Mode Absolute, P (0, 100, 0) | |
| plain-remap | Plain | Transform Mode Remap, P (0, 100, 0) | |
| plain-effector-space | Plain | Transform Space Effector, effector rotated H 90°, P (0, 100, 0) | |
| plain-weight | Plain | weight transform 50 %, position off | Record the clone weights |

## Notes
TODO
