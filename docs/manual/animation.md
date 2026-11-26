# Animation

## Time

A scene has a frame rate (default 30 fps) and a frame range (default 0 to 90). Frame `n`
is `n / fps` seconds. The timeline at the bottom shows the range and the current frame.

## Keys and tracks

A key stores a value at a time and says how to travel to the next key:

- **Spline** (default): a smooth curve through the keys;
- **Linear**: a straight ramp;
- **Step**: hold the value, then jump.

The interpolation belongs to the segment that *starts* at the key, so one track can mix all
three. A track animates one numeric parameter, or one component (x, y or z) of a vector
parameter such as position or rotation.

## Recording

Press the record button (key `K`) to write a key at the current frame for the selected
objects' position, rotation and scale. With **autokey** on, editing a value at a frame
writes a key automatically. Use the `key.*` and `timeline.*` commands from scripts.

Animated values are shown at the current frame; editing always targets the stored scene,
and dragging starts from the animated value, so objects do not jump.

The diamond beside a keyable Attributes row adds a key at the current frame; when the row
already has a key at this frame, clicking it removes that key (one undo step). Right-click it
to add or update, delete the row's keys at this frame, or jump to its previous or next key. The
timeline's Record button toggles the same way for the selection's position, scale and rotation. With Auto-Key
off, a change to an already animated row on a single selected object is a draft: click the
changed diamond to apply it. Moving to another frame discards the draft.

## Retiming in the Dope Sheet

Select the tracks or keys to edit. Drag to move them in time, and use the scale control to
stretch or compress them around a pivot frame. Positive factors preserve time order;
negative factors reverse it. The CLI equivalents are `key.move` and `key.scale`.

## Shaping a curve

In the Value Graph, select a spline key to show its handles. Break the tangents to edit
the incoming and outgoing sides independently. Weighted handles let you set the time
length as well as the slope. A handle's time offset is in frames and its value offset
uses the parameter's display units (e.g. degrees for an angle).

Switch to **Speed (/s)** to see signed velocity. Drag a spline-side grip vertically to
change the speed without moving the key. A zero outgoing speed eases away from the key;
when the handles are broken, the incoming speed stays unchanged. **Constant speed**
sets the selected segments to Linear.

## Looping after the last key

Set the track's After behaviour to **Repeat** to replay the curve. This repeats that
track's values, independent of whether timeline playback is set to Loop. For example,
for a key at frame 30 on node 1's X Position track:

```sh
forge cmd scene.forge key.set_tangent --save \
  --args '{"node":1,"track":"transform.pos.x","frame":30,"preset":"user","broken":true,"weighted":true,"left":{"dt_frames":-6,"dv":-12},"right":{"dt_frames":9,"dv":30}}'
forge cmd scene.forge key.set_speed --save \
  --args '{"node":1,"track":"transform.pos.x","frame":30,"side":"right","speed":0}'
forge cmd scene.forge track.set_behavior --save \
  --args '{"node":1,"track":"transform.pos.x","after":"repeat"}'
forge eval scene.forge --frame 75 --export frame-75.obj
```

For a complete headless example with scene creation, retiming and numeric checks, see
[`s7_retime_edit_tangents_ease_speed_and_loop`](../../apps/forge/tests/acceptance.rs).
