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
