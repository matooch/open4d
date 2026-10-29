# Scene, camera, light and render defaults — observation (O29)

Status: not observed · App version: TODO · OS: TODO · Observer: TODO · Date: TODO
Method: scripted read-back, as in [r25-defaults.md](r25-defaults.md): label, our key,
default, min and max (the ±999999999 assignment). This is a **form only**: write no
golden folders for it. Unblocks: U5 (camera, light, material) and U6 (render settings).

Make each object through its menu command in a new document. For the document and render
settings, read the new document's own values.

## 1. Document (project) settings
fps, minimum and maximum time, preview range, level of detail, units shown, and the
default object colour, if any.

| Label | Our key | Default | Min | Max |
|---|---|---|---|---|

## 2. Camera object
Every field of the Object, Physical, Details and Stereoscopic tabs.

| Label | Our key | Default | Min | Max |
|---|---|---|---|---|

## 3. Light object, for each Type option
Make one light per Type option (omni, spot, infinite, area and so on). Fill one table per
type, covering the General, Details, Visibility, Shadow and Noise tabs.

### Type: TODO
| Label | Our key | Default | Min | Max |
|---|---|---|---|---|

## 4. Standard material
Every channel's on/off default, and each enabled channel's fields: Color, Reflectance
(its default layer), Bump, Alpha, Luminance, Transparency.

| Channel | Label | Our key | Default | Min | Max |
|---|---|---|---|---|---|

## 5. Render settings of a new document
Renderer, output size, frame rate, anti-aliasing, the Options tab, and the Save tab
format.

| Label | Our key | Default | Min | Max |
|---|---|---|---|---|

## 6. Default scene behaviour
With no lights in the scene, how is a render lit? A default light, and where? Read it
from the documentation the app shows, or say it can't be read.

TODO

## Notes
TODO
