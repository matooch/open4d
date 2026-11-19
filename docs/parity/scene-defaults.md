# Scene, camera, light and render defaults — observation (O29)

Status: partial (safe scripted readback collected; unsupported controls and scene-light position outstanding) · App version: R25 (25010) · OS: macOS (Darwin 25.6.0) · Observer: Hermes · Date: 2026-11-18
Method: scripted read-back, as in [r25-defaults.md](r25-defaults.md): label, our key,
default, min and max (the ±999999999 assignment). This is a **form only**: write no
golden folders for it. Unblocks: U5 (camera, light, material) and U6 (render settings).

Make each object through its menu command in a new document. For the document and render
settings, read the new document's own values.

## 1. Document (project) settings
fps, minimum and maximum time, preview range, level of detail, units shown, and the
default object colour, if any.

| Tab / Group | Label | Our key | Default | Min | Max | Units / visibility / options |
|---|---|---|---|---|---|---|
| Project | Project Scale | project_scale | not observed: non-scalar custom control | not applicable | not applicable |  |
| Project / Time | FPS | fps | 30 | 1 | 2500 |  |
| Project / Time | Min Time | min_time | {"frames": 0, "seconds": 0.0} | {"frames": -18000, "seconds": -600.0} | {"frames": 3240000, "seconds": 108000.0} | frames and seconds |
| Project / Time | Max Time | max_time | {"frames": 90, "seconds": 3.0} | {"frames": -18000, "seconds": -600.0} | {"frames": 3240000, "seconds": 108000.0} | frames and seconds |
| Project / Time | Preview Min | preview_min | {"frames": 0, "seconds": 0.0} | {"frames": -18000, "seconds": -600.0} | {"frames": 3240000, "seconds": 108000.0} | frames and seconds |
| Project / Time | Preview Max | preview_max | {"frames": 90, "seconds": 3.0} | {"frames": 3240000, "seconds": 108000.0} | {"frames": 3240000, "seconds": 108000.0} | frames and seconds |
| Project / Display | Color | color | [0.8, 0.8, 0.8] | [0.8, 0.8, 0.8] | [0.8, 0.8, 0.8] |  |
| Project / Display | Level of Detail | level_of_detail | 100.0 | 0.0 | 100.0 | percent |

Project Scale readback: {"scale": 1.0, "unit": "cm"}. Project Scale read from new document; separate GUI unit-display preference not observed (preferences excluded).

## 2. Camera object
Every field of the Object, Physical, Details and Stereoscopic tabs.

| Tab / Group | Label | Our key | Default | Min | Max | Units / visibility / options |
|---|---|---|---|---|---|---|
| Object Properties | Projection | projection | Perspective | not applicable | not applicable | options: Perspective, Parallel, Left, Right, Front, Back, Top, Bottom, Military, Frog, Bird, Gentleman, Isometric, Dimetric |
| Object Properties | Sensor Size (Film Gate) | sensor_size_film_gate | 36.0 | 1.0 | 2000.0 |  |
| Object Properties | Field of View (Horizontal) | field_of_view_horizontal | 53.13010235415599 | 0.2 | 174.0 | degrees |
| Object Properties | Field of View (Vertical) | field_of_view_vertical | 31.41727565803149 | 0.20000000000000007 | 174.0 | degrees |
| Object Properties | Zoom | zoom | 1.0 | 0.0001 | 1000000.0 |  |
| Object Properties | Film Offset X | film_offset_x | 0.0 | -99999999900.0 | 99999999900.0 | percent |
| Object Properties | Film Offset Y | film_offset_y | 0.0 | -99999999900.0 | 99999999900.0 | percent |
| Object Properties | Focus Distance | focus_distance | 2000.0 | 0.0 | 999999999.0 | centimeters |
| Object Properties | Use Target Object | use_target_object | 0 | not applicable | not applicable |  |
| Object Properties | Focus Object | focus_object | none | not applicable | not applicable |  |
| Object Properties | White Balance (K) | white_balance_k | 6500.0 | 1000.0 | 10000.0 |  |
| Object Properties | Affect Lights Only | affect_lights_only | 0 | not applicable | not applicable |  |
| Object Properties | Export to Compositing | export_to_compositing | 1 | not applicable | not applicable |  |
| Physical Renderer | Movie Camera | movie_camera | 0 | not applicable | not applicable |  |
| Physical Renderer | F-Stop (f/#) | f_stop_f | 8.0 | 0.01 | 256.0 |  |
| Physical Renderer | Exposure | exposure | 0 | not applicable | not applicable |  |
| Physical Renderer | ISO | iso | 200.0 | 10.0 | 10000.0 |  |
| Physical Renderer | Gain (dB) | gain_db | 0.0 | -100.0 | 100.0 |  |
| Physical Renderer | Shutter Speed (s) | shutter_speed_s | 0.03333333333333333 | 0.0001 | 100.0 |  |
| Physical Renderer | Shutter Angle | shutter_angle | 180.0 | 10.0 | 1080.0 | degrees |
| Physical Renderer | Shutter Offset | shutter_offset | 0.0 | 0.0 | 360.0 | degrees |
| Physical Renderer | Shutter Efficiency | shutter_efficiency | 70.0 | 0.0 | 100.0 | percent |
| Physical Renderer | Lens Distortion - Quadratic | lens_distortion_quadratic | 0.0 | -100.0 | 100.0 | percent |
| Physical Renderer | Lens Distortion - Cubic | lens_distortion_cubic | 0.0 | -100.0 | 100.0 | percent |
| Physical Renderer | Vignetting Intensity | vignetting_intensity | 0.0 | 0.0 | 500.0 | percent |
| Physical Renderer | Vignetting Offset | vignetting_offset | 0.0 | 0.0 | 100.0 | percent |
| Physical Renderer | Chromatic Aberration | chromatic_aberration | 0.0 | -1000.0 | 1000.0 | percent |
| Physical Renderer | Diaphragm Shape | diaphragm_shape | 0 | not applicable | not applicable |  |
| Physical Renderer | Blades | blades | 6 | 3 | 32 |  |
| Physical Renderer | Angle | angle | 0.0 | 0.0 | 3600.0 | degrees |
| Physical Renderer | Bias | bias | 0.0 | -1.0 | 1.0 |  |
| Physical Renderer | Anisotropy | anisotropy | 0.0 | -100.0 | 100.0 | percent |
| Physical Renderer | Shader | shader | none | not applicable | not applicable |  |
| Details | Enable Near Clipping | enable_near_clipping | 1 | not applicable | not applicable |  |
| Details | Near Clipping | near_clipping | 0.0 | 0.0 | 999999999.0 | centimeters |
| Details | Enable Far Clipping | enable_far_clipping | 0 | not applicable | not applicable |  |
| Details | Far Clipping | far_clipping | 10000.0 | 0.0 | 999999999.0 | centimeters |
| Details | Projection Render Filename | projection_render_filename |  | not applicable | not applicable | hidden control |
| Details | Show Cone | show_cone | 1 | not applicable | not applicable |  |
| Details | DOF Map Front Blur | dof_map_front_blur | 0 | not applicable | not applicable |  |
| Details | Front Blur Start | front_blur_start | 0.0 | 0.0 | 999999999.0 | centimeters |
| Details | Front Blur End | front_blur_end | 1000.0 | 0.0 | 999999999.0 | centimeters |
| Details | DOF Map Rear Blur | dof_map_rear_blur | 0 | not applicable | not applicable |  |
| Details | Rear Blur Start | rear_blur_start | 0.0 | 0.0 | 999999999.0 | centimeters |
| Details | Rear Blur End | rear_blur_end | 1000.0 | 0.0 | 999999999.0 | centimeters |
| Stereoscopic | Mode | mode | Mono | not applicable | not applicable | options: Mono, Symmetrical, Left, Right |
| Stereoscopic | Eye Separation | eye_separation | 6.5 | 0.0 | 999999999.0 | centimeters |
| Stereoscopic | Placement | placement | Off Axis | not applicable | not applicable | options: Parallel, Off Axis, On Axis, Radial |
| Stereoscopic | Show All Cameras | show_all_cameras | 0 | not applicable | not applicable |  |
| Stereoscopic | Zero Parallax | zero_parallax | 1000.0 | 0.0 | 999999999.0 | centimeters |
| Stereoscopic | Auto Planes | auto_planes | Manual | not applicable | not applicable | options: Manual, 70, 90 |
| Stereoscopic | Near Plane | near_plane | 800.0 | 0.0 | 999999999.0 | centimeters |
| Stereoscopic | Far Plane | far_plane | 1200.0 | 0.0 | 999999999.0 | centimeters |
| Stereoscopic | Show Floating Frame | show_floating_frame | 0 | not applicable | not applicable |  |
| Stereoscopic | Stereo Mode | stereo_mode | Mono | not applicable | not applicable | options: Mono, Parallel, Toe-In |
| Stereoscopic | Stereo Layout | stereo_layout | Above-Under | not applicable | not applicable | options: Above-Under, Side-By-Side, Left Only, Right Only |
| Stereoscopic | Eye Separation | eye_separation_2 | 6.5 | 0.0 | 999999999.0 | centimeters |
| Stereoscopic | Eye to Neck Distance | eye_to_neck_distance | 6.5 | 0.0 | 999999999.0 | centimeters |
| Stereoscopic | Focal Distance | focal_distance | 100.0 | 0.01 | 999999999.0 | centimeters |
| Stereoscopic | Top Pole Smoothing | top_pole_smoothing | None | not applicable | not applicable | options: None, Linear, Exponential |
| Stereoscopic | Top Smoothing Angle | top_smoothing_angle | -90.0 | -90.0 | 0.0 | degrees |
| Stereoscopic | Top Pole Exp | top_pole_exp | 0.0 | 0.0 | 10.0 |  |
| Stereoscopic | Bottom Pole Smoothing | bottom_pole_smoothing | None | not applicable | not applicable | options: None, Linear, Exponential |
| Stereoscopic | Bottom Smoothing Angle | bottom_smoothing_angle | 90.0 | 0.0 | 90.0 | degrees |
| Stereoscopic | Bottom Pole Exp | bottom_pole_exp | 0.0 | 0.0 | 10.0 |  |
| Composition Helpers | Enable Composition Helpers | enable_composition_helpers | 1 | not applicable | not applicable |  |
| Composition Helpers | Draw Grid | draw_grid | 0 | not applicable | not applicable |  |
| Composition Helpers | Draw Diagonal | draw_diagonal | 0 | not applicable | not applicable |  |
| Composition Helpers | Draw Triangles | draw_triangles | 0 | not applicable | not applicable |  |
| Composition Helpers | Draw Golden Section | draw_golden_section | 0 | not applicable | not applicable |  |
| Composition Helpers | Draw Golden Spiral | draw_golden_spiral | 0 | not applicable | not applicable |  |
| Composition Helpers | Draw Crosshair | draw_crosshair | 0 | not applicable | not applicable |  |
| Composition Helpers / Grid | Grid Cells | grid_cells | 3 | 2 | 20 |  |
| Composition Helpers / Grid | Color | color | [0.77, 0.87, 0.81] | [0.0, 0.0, 0.0] | [1.0, 1.0, 1.0] |  |
| Composition Helpers / Diagonal | Mirror Diagonal | mirror_diagonal | 0 | not applicable | not applicable |  |
| Composition Helpers / Diagonal | Color | color_2 | [0.77, 0.87, 0.81] | [0.0, 0.0, 0.0] | [1.0, 1.0, 1.0] |  |
| Composition Helpers / Triangles | Triangles Mode | triangles_mode | Mode 1 | not applicable | not applicable | options: Mode 1, Mode 2 |
| Composition Helpers / Triangles | Mirror Triangles | mirror_triangles | 0 | not applicable | not applicable |  |
| Composition Helpers / Triangles | Flip Triangles | flip_triangles | 0 | not applicable | not applicable |  |
| Composition Helpers / Triangles | Color | color_3 | [0.77, 0.87, 0.81] | [0.0, 0.0, 0.0] | [1.0, 1.0, 1.0] |  |
| Composition Helpers / Golden Section | Golden Section Iterations | golden_section_iterations | 1 | 1 | 5 |  |
| Composition Helpers / Golden Section | Golden Section Top | golden_section_top | 1 | not applicable | not applicable |  |
| Composition Helpers / Golden Section | Golden Section Right | golden_section_right | 1 | not applicable | not applicable |  |
| Composition Helpers / Golden Section | Golden Section Bottom | golden_section_bottom | 1 | not applicable | not applicable |  |
| Composition Helpers / Golden Section | Golden Section Left | golden_section_left | 1 | not applicable | not applicable |  |
| Composition Helpers / Golden Section | Color | color_4 | [0.77, 0.87, 0.81] | [0.0, 0.0, 0.0] | [1.0, 1.0, 1.0] |  |
| Composition Helpers / Golden Spiral | Mirror H Golden Spiral | mirror_h_golden_spiral | 0 | not applicable | not applicable |  |
| Composition Helpers / Golden Spiral | Mirror V Golden Spiral | mirror_v_golden_spiral | 0 | not applicable | not applicable |  |
| Composition Helpers / Golden Spiral | Flip Golden Spiral | flip_golden_spiral | 0 | not applicable | not applicable |  |
| Composition Helpers / Golden Spiral | Align V Golden Spiral | align_v_golden_spiral | Center | not applicable | not applicable | options: Top, Center, Bottom |
| Composition Helpers / Golden Spiral | Align H Golden Spiral | align_h_golden_spiral | Center | not applicable | not applicable | hidden control; options: Left, Center, Right |
| Composition Helpers / Golden Spiral | Color | color_5 | [0.77, 0.87, 0.81] | [0.0, 0.0, 0.0] | [1.0, 1.0, 1.0] |  |
| Composition Helpers / Crosshair | Scale | scale | 100.0 | 0.0 | 99999999900.0 | percent |
| Composition Helpers / Crosshair | Color | color_6 | [0.77, 0.87, 0.81] | [0.0, 0.0, 0.0] | [1.0, 1.0, 1.0] |  |
| Spherical Camera | Enable | enable | 0 | not applicable | not applicable |  |
| Spherical Camera | FOV Helper | fov_helper | Equirectangular | not applicable | not applicable | options: Equirectangular, Dome |
| Spherical Camera | Mapping | mapping | Lat-Long | not applicable | not applicable | options: Lat-Long, Cube Map (cross), Cube Map (3x2), Cube Map (string) |
| Spherical Camera | Fit Frame | fit_frame | 0 | not applicable | not applicable |  |
| Spherical Camera | Use Full Range | use_full_range | 0 | not applicable | not applicable |  |
| Spherical Camera | Long Min | long_min | -180.0 | -180.0 | 180.0 | degrees |
| Spherical Camera | Long Max | long_max | 180.0 | -180.0 | 180.0 | degrees |
| Spherical Camera | Lat Min | lat_min | -90.0 | -90.0 | 90.0 | degrees |
| Spherical Camera | Lat Max | lat_max | 90.0 | -90.0 | 90.0 | degrees |
| Spherical Camera | Latitude | latitude | 0.0 | -57295779455.786545 | 57295779455.786545 | degrees |


## 3. Light object, for each Type option
Make one light per Type option (omni, spot, infinite, area and so on). Fill one table per
type, covering the General, Details, Visibility, Shadow and Noise tabs.

### Type: Omni

| Tab / Group | Label | Our key | Default | Min | Max | Units / visibility / options |
|---|---|---|---|---|---|---|
| General | Color | color | [1.0, 1.0, 1.0] | [0.0, 0.0, 0.0] | [1.0, 1.0, 1.0] |  |
| General | Use Temperature | use_temperature | 0 | not applicable | not applicable |  |
| General | Color Temperature | color_temperature | 6500.0 | 1000.0 | 10000.0 |  |
| General | Intensity | intensity | 100.0 | -10000000.0 | 10000000.0 | percent |
| General | Type | type | Omni | not applicable | not applicable | options: Omni, Spot, Infinite, Area, Square Spot, Parallel, Parallel Spot, Square Parallel Spot, IES |
| General | Shadow | shadow | None | not applicable | not applicable | options: None, Shadow Maps (Soft), Raytraced (Hard), Area |
| General | Visible Light | visible_light | None | not applicable | not applicable | options: None, Visible, Volumetric, Inverse Volumetric |
| General | No Illumination | no_illumination | 0 | not applicable | not applicable |  |
| General | Show Illumination | show_illumination | 1 | not applicable | not applicable |  |
| General | Ambient Illumination | ambient_illumination | 0 | not applicable | not applicable |  |
| General | Show Visible Light | show_visible_light | 1 | not applicable | not applicable |  |
| General | Diffuse | diffuse | 1 | not applicable | not applicable |  |
| General | Show Clipping | show_clipping | 1 | not applicable | not applicable |  |
| General | Specular | specular | 1 | not applicable | not applicable |  |
| General | Separate Pass | separate_pass | 0 | not applicable | not applicable |  |
| General | GI Illumination | gi_illumination | 1 | not applicable | not applicable |  |
| General | Export to Compositing | export_to_compositing | 1 | not applicable | not applicable |  |
| Details | Use Inner Value | use_inner_value | 1 | not applicable | not applicable |  |
| Details | Inner Angle | inner_angle | 0.0 | 0.0 | 175.0 | degrees |
| Details | Inner Radius | inner_radius | 0.0 | 0.0 | 999999999.0 | centimeters; hidden control |
| Details | Outer Angle | outer_angle | 29.999999999999996 | 0.0 | 175.0 | degrees |
| Details | Outer Radius | outer_radius | 100.0 | 0.0 | 999999999.0 | centimeters; hidden control |
| Details | Aspect Ratio | aspect_ratio | 1.0 | 0.01 | 100.0 |  |
| Details | Contrast | contrast | 0.0 | -1000.0 | 1000.0 | percent |
| Details | Shadow Caster | shadow_caster | 0 | not applicable | not applicable |  |
| Details | Area Shape | area_shape | Rectangle | not applicable | not applicable | options: Disc, Rectangle, Line, Sphere, Cylinder, Cylinder (Perpendicular), Cube, Hemisphere, Object / Spline |
| Details | Object | object | none | not applicable | not applicable |  |
| Details | Size X | size_x | 200.0 | 2e-05 | 999999999.0 | centimeters |
| Details | Size Y | size_y | 200.0 | 0.0 | 999999999.0 | centimeters |
| Details | Size Z | size_z | 200.0 | 0.0 | 999999999.0 | centimeters |
| Details | Falloff Angle | falloff_angle | 180.0 | 0.0 | 180.0 | degrees |
| Details | Infinite Angle | infinite_angle | 0.5 | 0.0 | 90.0 | degrees; hidden control |
| Details | Samples | samples | 40 | 16 | 1000 |  |
| Details | Add Grain (Slow) | add_grain_slow | 0 | not applicable | not applicable |  |
| Details | Show in Render | show_in_render | 0 | not applicable | not applicable |  |
| Details | Show as Solid in Viewport | show_as_solid_in_viewport | 0 | not applicable | not applicable |  |
| Details | Show in Specular | show_in_specular | 1 | not applicable | not applicable |  |
| Details | Show in Reflection | show_in_reflection | 0 | not applicable | not applicable |  |
| Details | Visibility Multiplier | visibility_multiplier | 100.0 | 0.0 | 10000000.0 | percent |
| Details | Falloff | falloff | None | not applicable | not applicable | options: None, Inverse Square (Physically Accurate), Linear, Step, Inverse Square Clamped |
| Details | Falloff Inner Radius | falloff_inner_radius | 0.0 | 0.0 | 999999999.0 | centimeters |
| Details | Falloff Radius / Decay | falloff_radius_decay | 500.0 | 0.0 | 999999999.0 | centimeters |
| Details | Colored Edge Falloff | colored_edge_falloff | 0 | not applicable | not applicable |  |
| Details | Z Direction Only | z_direction_only | 0 | not applicable | not applicable |  |
| Details | Use Gradient | use_gradient | 0 | not applicable | not applicable |  |
| Details | Color | color_2 | not observed: non-scalar custom control | not applicable | not applicable |  |
| Details | Use Near Clipping | use_near_clipping | 0 | not applicable | not applicable |  |
| Details | Near Clipping from | near_clipping_from | 0.0 | 0.0 | 999999999.0 | centimeters |
| Details | Near Clipping to | near_clipping_to | 10.0 | 0.0 | 999999999.0 | centimeters |
| Details | Use Far Clipping | use_far_clipping | 0 | not applicable | not applicable |  |
| Details | Far Clipping from | far_clipping_from | 90.0 | 0.0 | 999999999.0 | centimeters |
| Details | Far Clipping to | far_clipping_to | 100.0 | 0.0 | 999999999.0 | centimeters |
| Visibility | VL Use Falloff | vl_use_falloff | 1 | not applicable | not applicable |  |
| Visibility | VL Falloff | vl_falloff | 100.0 | 0.0 | 100.0 | percent |
| Visibility | VL Use Edge Falloff | vl_use_edge_falloff | 1 | not applicable | not applicable |  |
| Visibility | VL Edge Falloff | vl_edge_falloff | 50.0 | 0.0 | 100.0 | percent |
| Visibility | VL Colored Edge Falloff | vl_colored_edge_falloff | 0 | not applicable | not applicable |  |
| Visibility | VL Inner Distance | vl_inner_distance | 0.0 | 0.0 | 999999999.0 | centimeters |
| Visibility | VL Outer Distance | vl_outer_distance | 500.0 | 0.0 | 999999999.0 | centimeters |
| Visibility | VL Relative Scale | vl_relative_scale | [100.0, 100.0, 100.0] | [0.0, 0.0, 0.0] | [10000.0, 10000.0, 10000.0] | percent |
| Visibility | VL Sample Distance | vl_sample_distance | 25.0 | 0.0 | 999999999.0 | centimeters |
| Visibility | VL Brightness | vl_brightness | 100.0 | 0.0 | 10000.0 | percent |
| Visibility | VL Dust | vl_dust | 0.0 | 0.0 | 100.0 | percent |
| Visibility | VL Dithering | vl_dithering | 0.0 | 0.0 | 100.0 | percent |
| Visibility | VL Use Gradient | vl_use_gradient | 0 | not applicable | not applicable |  |
| Visibility | VL Color | vl_color | not observed: non-scalar custom control | not applicable | not applicable |  |
| Visibility | VL Additive | vl_additive | 0 | not applicable | not applicable |  |
| Visibility | VL Adapt Brightness | vl_adapt_brightness | 1 | not applicable | not applicable |  |
| Shadow | Shadow | shadow_2 | None | not applicable | not applicable | options: None, Shadow Maps (Soft), Raytraced (Hard), Area |
| Shadow | Shadow Density | shadow_density | 100.0 | 0.0 | 10000.0 | percent |
| Shadow | Shadow Color | shadow_color | [0.0, 0.0, 0.0] | [0.0, 0.0, 0.0] | [1.0, 1.0, 1.0] |  |
| Shadow | Shadow Transparency | shadow_transparency | 1 | not applicable | not applicable |  |
| Shadow | Shadow Clipping Influence | shadow_clipping_influence | 0 | not applicable | not applicable |  |
| Shadow | Shadow Map | shadow_map | 250x250 | not applicable | not applicable | hidden control; options: 250x250, 500x500, 750x750, 1000x1000, 1250x1250, 1500x1500, 1750x1750, 2000x2000, Custom |
| Shadow | Shadow Map Size X | shadow_map_size_x | 250 | 40 | 10000 | hidden control |
| Shadow | Shadow Map Size Y | shadow_map_size_y | 250 | 40 | 10000 | hidden control |
| Shadow | Shadow Sample Radius | shadow_sample_radius | 3 | 1 | 20 | hidden control |
| Shadow | Sampling Radius Boost | sampling_radius_boost | None | not applicable | not applicable | hidden control; options: None, Low (2x), Medium (4x), High (8x) |
| Shadow | Shadow Absolute Bias | shadow_absolute_bias | 1 | not applicable | not applicable | hidden control |
| Shadow | Shadow Bias (Rel) | shadow_bias_rel | 1.0 | 0.0 | 100.0 | percent; hidden control |
| Shadow | Shadow Bias (Abs) | shadow_bias_abs | 2.0 | 0.0 | 999999999.0 | centimeters; hidden control |
| Shadow | Shadow Parallel Width | shadow_parallel_width | 500.0 | -999999999.0 | 999999999.0 | centimeters; hidden control |
| Shadow | Outline Shadow | outline_shadow | 0 | not applicable | not applicable | hidden control |
| Shadow | High Quality | high_quality | 0 | not applicable | not applicable | hidden control |
| Shadow | Shadow Cone | shadow_cone | 0 | not applicable | not applicable | hidden control |
| Shadow | Shadow Cone Angle | shadow_cone_angle | 90.0 | 0.0 | 170.0 | degrees; hidden control |
| Shadow | Shadow Soft Cone | shadow_soft_cone | 1 | not applicable | not applicable | hidden control |
| Shadow | Area Shadow Width | area_shadow_width | 100.0 | -999999999.0 | 999999999.0 | centimeters; hidden control |
| Shadow | Accuracy | accuracy | 75.0 | 0.0 | 100.0 | percent; hidden control |
| Shadow | Minimum Samples | minimum_samples | 8 | 2 | 10000 | hidden control |
| Shadow | Maximum Samples | maximum_samples | 100 | 2 | 10000 | hidden control |
| Photometric | Photometric Intensity | photometric_intensity | 0 | not applicable | not applicable |  |
| Photometric | Intensity | intensity_2 | 1000.0 | 0.0 | 10000000.0 |  |
| Photometric | Units | units | Candela (cd) | not applicable | not applicable | options: Lumen (lm), Candela (cd) |
| Photometric | Photometric Data | photometric_data | 1 | not applicable | not applicable |  |
| Photometric | Filename | filename |  | not applicable | not applicable |  |
| Photometric | Photometric Size | photometric_size | 0 | not applicable | not applicable |  |
| Caustics | Surface Caustics | surface_caustics | 0 | not applicable | not applicable |  |
| Caustics | Surface Energy | surface_energy | 100.0 | 0.0 | 99999999900.0 | percent |
| Caustics | Surface Photons | surface_photons | 10000 | 10 | 10000000 |  |
| Caustics | Volume Caustics | volume_caustics | 0 | not applicable | not applicable |  |
| Caustics | Volume Energy | volume_energy | 100.0 | 0.0 | 99999999900.0 | percent |
| Caustics | Volume Photons | volume_photons | 10000 | 10 | 2000000 |  |
| Caustics | Caustics Falloff | caustics_falloff | None | not applicable | not applicable | options: None, Linear, Inverse, Inverse Square, Inverse Cubic, Step |
| Caustics | Caustics Inner Distance | caustics_inner_distance | 0.0 | 0.0 | 999999999.0 | centimeters |
| Caustics | Caustics Outer Distance | caustics_outer_distance | 1000.0 | 0.0 | 999999999.0 | centimeters |
| Noise | Noise | noise | None | not applicable | not applicable | options: None, Illumination, Visibility, Both |
| Noise | Noise Type | noise_type | Soft Turbulence | not applicable | not applicable | options: Noise, Soft Turbulence, Hard Turbulence, Wavy Turbulence |
| Noise | Noise Octaves | noise_octaves | 4 | 1 | 8 |  |
| Noise | Noise Velocity | noise_velocity | 25.0 | 0.0 | 1000.0 | percent |
| Noise | Noise Brightness | noise_brightness | 0.0 | -1000.0 | 1000.0 | percent |
| Noise | Noise Contrast | noise_contrast | 100.0 | -1000.0 | 1000.0 | percent |
| Noise | Noise Local | noise_local | 1 | not applicable | not applicable |  |
| Noise | Noise Visibility Scale | noise_visibility_scale | [100.0, 100.0, 100.0] | [0.0, 0.0, 0.0] | [999999999.0, 999999999.0, 999999999.0] | centimeters |
| Noise | Noise Illumination Scale | noise_illumination_scale | 4.0 | 0.0 | 999999999.0 |  |
| Noise | Noise Wind | noise_wind | [1.0, 0.0, 0.0] | [-999999999.0, -999999999.0, -999999999.0] | [999999999.0, 999999999.0, 999999999.0] | centimeters |
| Noise | Noise Wind Velocity | noise_wind_velocity | 0.0 | 0.0 | 999999999.0 | centimeters |
| Lens Effects | FX Glow | fx_glow | Inactive | not applicable | not applicable | options: Inactive, Custom, Default, Cinema 4D R4, Wideangle, Zoom, Hi-8, Camcorder, Searchlight, Artifact, Star 1, Star 2, Star 3, Purple, Flashlight, Sun 1, Sun 2, Grey, Blue 1, Blue 2, Red, Yellow-Green 1, Yellow-Green 2, Candle |
| Lens Effects | FX Glow Brightness | fx_glow_brightness | 100.0 | 0.0 | 1000.0 | percent |
| Lens Effects | FX Glow Aspect Ratio | fx_glow_aspect_ratio | 1.0 | 0.1 | 10.0 |  |
| Lens Effects | FX Glow Settings | fx_glow_settings | not observed: non-scalar custom control | not applicable | not applicable |  |
| Lens Effects | FX Reflexes | fx_reflexes | Inactive | not applicable | not applicable | options: Inactive, Custom, Default, Cinema 4D R4, Wideangle, Zoom, Hi-8, Camcorder, Searchlight, Artifact, Star 1, Star 2, Star 3, Purple, Flashlight 1, Flashlight 2, Flashlight 3 |
| Lens Effects | FX Reflection Brightness | fx_reflection_brightness | 100.0 | 0.0 | 1000.0 | percent |
| Lens Effects | FX Reflection Aspect Ratio | fx_reflection_aspect_ratio | 1.0 | 0.1 | 10.0 |  |
| Lens Effects | FX Scale | fx_scale | 100.0 | 0.0 | 1000.0 | percent |
| Lens Effects | FX Rotation | fx_rotation | 0.0 | -57295779455.786545 | 57295779455.786545 | degrees |
| Lens Effects | FX Reference Size | fx_reference_size | 1000.0 | 0.0 | 999999999.0 | centimeters |
| Lens Effects | FX Use Light Parameters | fx_use_light_parameters | 1 | not applicable | not applicable |  |
| Lens Effects | FX Fade if Near Border | fx_fade_if_near_border | 0 | not applicable | not applicable |  |
| Lens Effects | FX Fade if Behind Object | fx_fade_if_behind_object | 1 | not applicable | not applicable |  |
| Lens Effects | FX Fade if Approaching Object | fx_fade_if_approaching_object | 0 | not applicable | not applicable |  |
| Lens Effects | FX Glow Distance Scale | fx_glow_distance_scale | 0 | not applicable | not applicable |  |
| Lens Effects | FX Reflex Distance Scale | fx_reflex_distance_scale | 0 | not applicable | not applicable |  |
| Project | Mode | mode | Exclude | not applicable | not applicable | options: Include, Exclude |
| Project | Objects | objects | not observed: non-scalar custom control | not applicable | not applicable |  |
| Project | PyroCluster Illumination | pyrocluster_illumination | 1 | not applicable | not applicable |  |
| Project | PyroCluster Shadow Casting | pyrocluster_shadow_casting | 1 | not applicable | not applicable |  |

### Type: Spot

| Tab / Group | Label | Our key | Default | Min | Max | Units / visibility / options |
|---|---|---|---|---|---|---|
| General | Color | color | [1.0, 1.0, 1.0] | [0.0, 0.0, 0.0] | [1.0, 1.0, 1.0] |  |
| General | Use Temperature | use_temperature | 0 | not applicable | not applicable |  |
| General | Color Temperature | color_temperature | 6500.0 | 1000.0 | 10000.0 |  |
| General | Intensity | intensity | 100.0 | -10000000.0 | 10000000.0 | percent |
| General | Type | type | Spot | not applicable | not applicable | options: Omni, Spot, Infinite, Area, Square Spot, Parallel, Parallel Spot, Square Parallel Spot, IES |
| General | Shadow | shadow | None | not applicable | not applicable | options: None, Shadow Maps (Soft), Raytraced (Hard), Area |
| General | Visible Light | visible_light | None | not applicable | not applicable | options: None, Visible, Volumetric, Inverse Volumetric |
| General | No Illumination | no_illumination | 0 | not applicable | not applicable |  |
| General | Show Illumination | show_illumination | 1 | not applicable | not applicable |  |
| General | Ambient Illumination | ambient_illumination | 0 | not applicable | not applicable |  |
| General | Show Visible Light | show_visible_light | 1 | not applicable | not applicable |  |
| General | Diffuse | diffuse | 1 | not applicable | not applicable |  |
| General | Show Clipping | show_clipping | 1 | not applicable | not applicable |  |
| General | Specular | specular | 1 | not applicable | not applicable |  |
| General | Separate Pass | separate_pass | 0 | not applicable | not applicable |  |
| General | GI Illumination | gi_illumination | 1 | not applicable | not applicable |  |
| General | Export to Compositing | export_to_compositing | 1 | not applicable | not applicable |  |
| Details | Use Inner Value | use_inner_value | 1 | not applicable | not applicable |  |
| Details | Inner Angle | inner_angle | 0.0 | 0.0 | 175.0 | degrees |
| Details | Inner Radius | inner_radius | 0.0 | 0.0 | 999999999.0 | centimeters; hidden control |
| Details | Outer Angle | outer_angle | 29.999999999999996 | 0.0 | 175.0 | degrees |
| Details | Outer Radius | outer_radius | 100.0 | 0.0 | 999999999.0 | centimeters; hidden control |
| Details | Aspect Ratio | aspect_ratio | 1.0 | 0.01 | 100.0 |  |
| Details | Contrast | contrast | 0.0 | -1000.0 | 1000.0 | percent |
| Details | Shadow Caster | shadow_caster | 0 | not applicable | not applicable |  |
| Details | Area Shape | area_shape | Rectangle | not applicable | not applicable | options: Disc, Rectangle, Line, Sphere, Cylinder, Cylinder (Perpendicular), Cube, Hemisphere, Object / Spline |
| Details | Object | object | none | not applicable | not applicable |  |
| Details | Size X | size_x | 200.0 | 2e-05 | 999999999.0 | centimeters |
| Details | Size Y | size_y | 200.0 | 0.0 | 999999999.0 | centimeters |
| Details | Size Z | size_z | 200.0 | 0.0 | 999999999.0 | centimeters |
| Details | Falloff Angle | falloff_angle | 180.0 | 0.0 | 180.0 | degrees |
| Details | Infinite Angle | infinite_angle | 0.5 | 0.0 | 90.0 | degrees; hidden control |
| Details | Samples | samples | 40 | 16 | 1000 |  |
| Details | Add Grain (Slow) | add_grain_slow | 0 | not applicable | not applicable |  |
| Details | Show in Render | show_in_render | 0 | not applicable | not applicable |  |
| Details | Show as Solid in Viewport | show_as_solid_in_viewport | 0 | not applicable | not applicable |  |
| Details | Show in Specular | show_in_specular | 1 | not applicable | not applicable |  |
| Details | Show in Reflection | show_in_reflection | 0 | not applicable | not applicable |  |
| Details | Visibility Multiplier | visibility_multiplier | 100.0 | 0.0 | 10000000.0 | percent |
| Details | Falloff | falloff | None | not applicable | not applicable | options: None, Inverse Square (Physically Accurate), Linear, Step, Inverse Square Clamped |
| Details | Falloff Inner Radius | falloff_inner_radius | 0.0 | 0.0 | 999999999.0 | centimeters |
| Details | Falloff Radius / Decay | falloff_radius_decay | 500.0 | 0.0 | 999999999.0 | centimeters |
| Details | Colored Edge Falloff | colored_edge_falloff | 0 | not applicable | not applicable |  |
| Details | Z Direction Only | z_direction_only | 0 | not applicable | not applicable |  |
| Details | Use Gradient | use_gradient | 0 | not applicable | not applicable |  |
| Details | Color | color_2 | not observed: non-scalar custom control | not applicable | not applicable |  |
| Details | Use Near Clipping | use_near_clipping | 0 | not applicable | not applicable |  |
| Details | Near Clipping from | near_clipping_from | 0.0 | 0.0 | 999999999.0 | centimeters |
| Details | Near Clipping to | near_clipping_to | 10.0 | 0.0 | 999999999.0 | centimeters |
| Details | Use Far Clipping | use_far_clipping | 0 | not applicable | not applicable |  |
| Details | Far Clipping from | far_clipping_from | 90.0 | 0.0 | 999999999.0 | centimeters |
| Details | Far Clipping to | far_clipping_to | 100.0 | 0.0 | 999999999.0 | centimeters |
| Visibility | VL Use Falloff | vl_use_falloff | 1 | not applicable | not applicable |  |
| Visibility | VL Falloff | vl_falloff | 100.0 | 0.0 | 100.0 | percent |
| Visibility | VL Use Edge Falloff | vl_use_edge_falloff | 1 | not applicable | not applicable |  |
| Visibility | VL Edge Falloff | vl_edge_falloff | 50.0 | 0.0 | 100.0 | percent |
| Visibility | VL Colored Edge Falloff | vl_colored_edge_falloff | 0 | not applicable | not applicable |  |
| Visibility | VL Inner Distance | vl_inner_distance | 0.0 | 0.0 | 999999999.0 | centimeters |
| Visibility | VL Outer Distance | vl_outer_distance | 500.0 | 0.0 | 999999999.0 | centimeters |
| Visibility | VL Relative Scale | vl_relative_scale | [100.0, 100.0, 100.0] | [0.0, 0.0, 0.0] | [10000.0, 10000.0, 10000.0] | percent |
| Visibility | VL Sample Distance | vl_sample_distance | 25.0 | 0.0 | 999999999.0 | centimeters |
| Visibility | VL Brightness | vl_brightness | 100.0 | 0.0 | 10000.0 | percent |
| Visibility | VL Dust | vl_dust | 0.0 | 0.0 | 100.0 | percent |
| Visibility | VL Dithering | vl_dithering | 0.0 | 0.0 | 100.0 | percent |
| Visibility | VL Use Gradient | vl_use_gradient | 0 | not applicable | not applicable |  |
| Visibility | VL Color | vl_color | not observed: non-scalar custom control | not applicable | not applicable |  |
| Visibility | VL Additive | vl_additive | 0 | not applicable | not applicable |  |
| Visibility | VL Adapt Brightness | vl_adapt_brightness | 1 | not applicable | not applicable |  |
| Shadow | Shadow | shadow_2 | None | not applicable | not applicable | options: None, Shadow Maps (Soft), Raytraced (Hard), Area |
| Shadow | Shadow Density | shadow_density | 100.0 | 0.0 | 10000.0 | percent |
| Shadow | Shadow Color | shadow_color | [0.0, 0.0, 0.0] | [0.0, 0.0, 0.0] | [1.0, 1.0, 1.0] |  |
| Shadow | Shadow Transparency | shadow_transparency | 1 | not applicable | not applicable |  |
| Shadow | Shadow Clipping Influence | shadow_clipping_influence | 0 | not applicable | not applicable |  |
| Shadow | Shadow Map | shadow_map | 250x250 | not applicable | not applicable | hidden control; options: 250x250, 500x500, 750x750, 1000x1000, 1250x1250, 1500x1500, 1750x1750, 2000x2000, Custom |
| Shadow | Shadow Map Size X | shadow_map_size_x | 250 | 40 | 10000 | hidden control |
| Shadow | Shadow Map Size Y | shadow_map_size_y | 250 | 40 | 10000 | hidden control |
| Shadow | Shadow Sample Radius | shadow_sample_radius | 3 | 1 | 20 | hidden control |
| Shadow | Sampling Radius Boost | sampling_radius_boost | None | not applicable | not applicable | hidden control; options: None, Low (2x), Medium (4x), High (8x) |
| Shadow | Shadow Absolute Bias | shadow_absolute_bias | 1 | not applicable | not applicable | hidden control |
| Shadow | Shadow Bias (Rel) | shadow_bias_rel | 1.0 | 0.0 | 100.0 | percent; hidden control |
| Shadow | Shadow Bias (Abs) | shadow_bias_abs | 2.0 | 0.0 | 999999999.0 | centimeters; hidden control |
| Shadow | Shadow Parallel Width | shadow_parallel_width | 500.0 | -999999999.0 | 999999999.0 | centimeters; hidden control |
| Shadow | Outline Shadow | outline_shadow | 0 | not applicable | not applicable | hidden control |
| Shadow | High Quality | high_quality | 0 | not applicable | not applicable | hidden control |
| Shadow | Shadow Cone | shadow_cone | 0 | not applicable | not applicable | hidden control |
| Shadow | Shadow Cone Angle | shadow_cone_angle | 90.0 | 0.0 | 170.0 | degrees; hidden control |
| Shadow | Shadow Soft Cone | shadow_soft_cone | 1 | not applicable | not applicable | hidden control |
| Shadow | Area Shadow Width | area_shadow_width | 100.0 | -999999999.0 | 999999999.0 | centimeters; hidden control |
| Shadow | Accuracy | accuracy | 75.0 | 0.0 | 100.0 | percent; hidden control |
| Shadow | Minimum Samples | minimum_samples | 8 | 2 | 10000 | hidden control |
| Shadow | Maximum Samples | maximum_samples | 100 | 2 | 10000 | hidden control |
| Photometric | Photometric Intensity | photometric_intensity | 0 | not applicable | not applicable |  |
| Photometric | Intensity | intensity_2 | 1000.0 | 0.0 | 10000000.0 |  |
| Photometric | Units | units | Candela (cd) | not applicable | not applicable | options: Lumen (lm), Candela (cd) |
| Photometric | Photometric Data | photometric_data | 1 | not applicable | not applicable |  |
| Photometric | Filename | filename |  | not applicable | not applicable |  |
| Photometric | Photometric Size | photometric_size | 0 | not applicable | not applicable |  |
| Caustics | Surface Caustics | surface_caustics | 0 | not applicable | not applicable |  |
| Caustics | Surface Energy | surface_energy | 100.0 | 0.0 | 99999999900.0 | percent |
| Caustics | Surface Photons | surface_photons | 10000 | 10 | 10000000 |  |
| Caustics | Volume Caustics | volume_caustics | 0 | not applicable | not applicable |  |
| Caustics | Volume Energy | volume_energy | 100.0 | 0.0 | 99999999900.0 | percent |
| Caustics | Volume Photons | volume_photons | 10000 | 10 | 2000000 |  |
| Caustics | Caustics Falloff | caustics_falloff | None | not applicable | not applicable | options: None, Linear, Inverse, Inverse Square, Inverse Cubic, Step |
| Caustics | Caustics Inner Distance | caustics_inner_distance | 0.0 | 0.0 | 999999999.0 | centimeters |
| Caustics | Caustics Outer Distance | caustics_outer_distance | 1000.0 | 0.0 | 999999999.0 | centimeters |
| Noise | Noise | noise | None | not applicable | not applicable | options: None, Illumination, Visibility, Both |
| Noise | Noise Type | noise_type | Soft Turbulence | not applicable | not applicable | options: Noise, Soft Turbulence, Hard Turbulence, Wavy Turbulence |
| Noise | Noise Octaves | noise_octaves | 4 | 1 | 8 |  |
| Noise | Noise Velocity | noise_velocity | 25.0 | 0.0 | 1000.0 | percent |
| Noise | Noise Brightness | noise_brightness | 0.0 | -1000.0 | 1000.0 | percent |
| Noise | Noise Contrast | noise_contrast | 100.0 | -1000.0 | 1000.0 | percent |
| Noise | Noise Local | noise_local | 1 | not applicable | not applicable |  |
| Noise | Noise Visibility Scale | noise_visibility_scale | [100.0, 100.0, 100.0] | [0.0, 0.0, 0.0] | [999999999.0, 999999999.0, 999999999.0] | centimeters |
| Noise | Noise Illumination Scale | noise_illumination_scale | 4.0 | 0.0 | 999999999.0 |  |
| Noise | Noise Wind | noise_wind | [1.0, 0.0, 0.0] | [-999999999.0, -999999999.0, -999999999.0] | [999999999.0, 999999999.0, 999999999.0] | centimeters |
| Noise | Noise Wind Velocity | noise_wind_velocity | 0.0 | 0.0 | 999999999.0 | centimeters |
| Lens Effects | FX Glow | fx_glow | Inactive | not applicable | not applicable | options: Inactive, Custom, Default, Cinema 4D R4, Wideangle, Zoom, Hi-8, Camcorder, Searchlight, Artifact, Star 1, Star 2, Star 3, Purple, Flashlight, Sun 1, Sun 2, Grey, Blue 1, Blue 2, Red, Yellow-Green 1, Yellow-Green 2, Candle |
| Lens Effects | FX Glow Brightness | fx_glow_brightness | 100.0 | 0.0 | 1000.0 | percent |
| Lens Effects | FX Glow Aspect Ratio | fx_glow_aspect_ratio | 1.0 | 0.1 | 10.0 |  |
| Lens Effects | FX Glow Settings | fx_glow_settings | not observed: non-scalar custom control | not applicable | not applicable |  |
| Lens Effects | FX Reflexes | fx_reflexes | Inactive | not applicable | not applicable | options: Inactive, Custom, Default, Cinema 4D R4, Wideangle, Zoom, Hi-8, Camcorder, Searchlight, Artifact, Star 1, Star 2, Star 3, Purple, Flashlight 1, Flashlight 2, Flashlight 3 |
| Lens Effects | FX Reflection Brightness | fx_reflection_brightness | 100.0 | 0.0 | 1000.0 | percent |
| Lens Effects | FX Reflection Aspect Ratio | fx_reflection_aspect_ratio | 1.0 | 0.1 | 10.0 |  |
| Lens Effects | FX Scale | fx_scale | 100.0 | 0.0 | 1000.0 | percent |
| Lens Effects | FX Rotation | fx_rotation | 0.0 | -57295779455.786545 | 57295779455.786545 | degrees |
| Lens Effects | FX Reference Size | fx_reference_size | 1000.0 | 0.0 | 999999999.0 | centimeters |
| Lens Effects | FX Use Light Parameters | fx_use_light_parameters | 1 | not applicable | not applicable |  |
| Lens Effects | FX Fade if Near Border | fx_fade_if_near_border | 0 | not applicable | not applicable |  |
| Lens Effects | FX Fade if Behind Object | fx_fade_if_behind_object | 1 | not applicable | not applicable |  |
| Lens Effects | FX Fade if Approaching Object | fx_fade_if_approaching_object | 0 | not applicable | not applicable |  |
| Lens Effects | FX Glow Distance Scale | fx_glow_distance_scale | 0 | not applicable | not applicable |  |
| Lens Effects | FX Reflex Distance Scale | fx_reflex_distance_scale | 0 | not applicable | not applicable |  |
| Project | Mode | mode | Exclude | not applicable | not applicable | options: Include, Exclude |
| Project | Objects | objects | not observed: non-scalar custom control | not applicable | not applicable |  |
| Project | PyroCluster Illumination | pyrocluster_illumination | 1 | not applicable | not applicable |  |
| Project | PyroCluster Shadow Casting | pyrocluster_shadow_casting | 1 | not applicable | not applicable |  |

### Type: Infinite

| Tab / Group | Label | Our key | Default | Min | Max | Units / visibility / options |
|---|---|---|---|---|---|---|
| General | Color | color | [1.0, 1.0, 1.0] | [0.0, 0.0, 0.0] | [1.0, 1.0, 1.0] |  |
| General | Use Temperature | use_temperature | 0 | not applicable | not applicable |  |
| General | Color Temperature | color_temperature | 6500.0 | 1000.0 | 10000.0 |  |
| General | Intensity | intensity | 100.0 | -10000000.0 | 10000000.0 | percent |
| General | Type | type | Infinite | not applicable | not applicable | options: Omni, Spot, Infinite, Area, Square Spot, Parallel, Parallel Spot, Square Parallel Spot, IES |
| General | Shadow | shadow | None | not applicable | not applicable | options: None, Shadow Maps (Soft), Raytraced (Hard), Area |
| General | Visible Light | visible_light | None | not applicable | not applicable | options: None, Visible, Volumetric, Inverse Volumetric |
| General | No Illumination | no_illumination | 0 | not applicable | not applicable |  |
| General | Show Illumination | show_illumination | 1 | not applicable | not applicable |  |
| General | Ambient Illumination | ambient_illumination | 0 | not applicable | not applicable |  |
| General | Show Visible Light | show_visible_light | 1 | not applicable | not applicable |  |
| General | Diffuse | diffuse | 1 | not applicable | not applicable |  |
| General | Show Clipping | show_clipping | 1 | not applicable | not applicable |  |
| General | Specular | specular | 1 | not applicable | not applicable |  |
| General | Separate Pass | separate_pass | 0 | not applicable | not applicable |  |
| General | GI Illumination | gi_illumination | 1 | not applicable | not applicable |  |
| General | Export to Compositing | export_to_compositing | 1 | not applicable | not applicable |  |
| Details | Use Inner Value | use_inner_value | 1 | not applicable | not applicable |  |
| Details | Inner Angle | inner_angle | 0.0 | 0.0 | 175.0 | degrees |
| Details | Inner Radius | inner_radius | 0.0 | 0.0 | 999999999.0 | centimeters; hidden control |
| Details | Outer Angle | outer_angle | 29.999999999999996 | 0.0 | 175.0 | degrees |
| Details | Outer Radius | outer_radius | 100.0 | 0.0 | 999999999.0 | centimeters; hidden control |
| Details | Aspect Ratio | aspect_ratio | 1.0 | 0.01 | 100.0 |  |
| Details | Contrast | contrast | 0.0 | -1000.0 | 1000.0 | percent |
| Details | Shadow Caster | shadow_caster | 0 | not applicable | not applicable |  |
| Details | Area Shape | area_shape | Rectangle | not applicable | not applicable | hidden control; options: Disc, Rectangle, Line, Sphere, Cylinder, Cylinder (Perpendicular), Cube, Hemisphere, Object / Spline |
| Details | Object | object | none | not applicable | not applicable | hidden control |
| Details | Size X | size_x | 200.0 | 2e-05 | 999999999.0 | centimeters; hidden control |
| Details | Size Y | size_y | 200.0 | 0.0 | 999999999.0 | centimeters; hidden control |
| Details | Size Z | size_z | 200.0 | 0.0 | 999999999.0 | centimeters; hidden control |
| Details | Falloff Angle | falloff_angle | 180.0 | 0.0 | 180.0 | degrees |
| Details | Infinite Angle | infinite_angle | 0.5 | 0.0 | 90.0 | degrees; hidden control |
| Details | Samples | samples | 40 | 16 | 1000 |  |
| Details | Add Grain (Slow) | add_grain_slow | 0 | not applicable | not applicable |  |
| Details | Show in Render | show_in_render | 0 | not applicable | not applicable |  |
| Details | Show as Solid in Viewport | show_as_solid_in_viewport | 0 | not applicable | not applicable |  |
| Details | Show in Specular | show_in_specular | 1 | not applicable | not applicable |  |
| Details | Show in Reflection | show_in_reflection | 0 | not applicable | not applicable |  |
| Details | Visibility Multiplier | visibility_multiplier | 100.0 | 0.0 | 10000000.0 | percent |
| Details | Falloff | falloff | None | not applicable | not applicable | options: None, Inverse Square (Physically Accurate), Linear, Step, Inverse Square Clamped |
| Details | Falloff Inner Radius | falloff_inner_radius | 0.0 | 0.0 | 999999999.0 | centimeters |
| Details | Falloff Radius / Decay | falloff_radius_decay | 500.0 | 0.0 | 999999999.0 | centimeters |
| Details | Colored Edge Falloff | colored_edge_falloff | 0 | not applicable | not applicable |  |
| Details | Z Direction Only | z_direction_only | 0 | not applicable | not applicable |  |
| Details | Use Gradient | use_gradient | 0 | not applicable | not applicable |  |
| Details | Color | color_2 | not observed: non-scalar custom control | not applicable | not applicable |  |
| Details | Use Near Clipping | use_near_clipping | 0 | not applicable | not applicable | hidden control |
| Details | Near Clipping from | near_clipping_from | 0.0 | 0.0 | 999999999.0 | centimeters; hidden control |
| Details | Near Clipping to | near_clipping_to | 10.0 | 0.0 | 999999999.0 | centimeters; hidden control |
| Details | Use Far Clipping | use_far_clipping | 0 | not applicable | not applicable | hidden control |
| Details | Far Clipping from | far_clipping_from | 90.0 | 0.0 | 999999999.0 | centimeters; hidden control |
| Details | Far Clipping to | far_clipping_to | 100.0 | 0.0 | 999999999.0 | centimeters; hidden control |
| Visibility | VL Use Falloff | vl_use_falloff | 1 | not applicable | not applicable |  |
| Visibility | VL Falloff | vl_falloff | 100.0 | 0.0 | 100.0 | percent |
| Visibility | VL Use Edge Falloff | vl_use_edge_falloff | 1 | not applicable | not applicable |  |
| Visibility | VL Edge Falloff | vl_edge_falloff | 50.0 | 0.0 | 100.0 | percent |
| Visibility | VL Colored Edge Falloff | vl_colored_edge_falloff | 0 | not applicable | not applicable |  |
| Visibility | VL Inner Distance | vl_inner_distance | 0.0 | 0.0 | 999999999.0 | centimeters |
| Visibility | VL Outer Distance | vl_outer_distance | 500.0 | 0.0 | 999999999.0 | centimeters |
| Visibility | VL Relative Scale | vl_relative_scale | [100.0, 100.0, 100.0] | [0.0, 0.0, 0.0] | [10000.0, 10000.0, 10000.0] | percent |
| Visibility | VL Sample Distance | vl_sample_distance | 25.0 | 0.0 | 999999999.0 | centimeters |
| Visibility | VL Brightness | vl_brightness | 100.0 | 0.0 | 10000.0 | percent |
| Visibility | VL Dust | vl_dust | 0.0 | 0.0 | 100.0 | percent |
| Visibility | VL Dithering | vl_dithering | 0.0 | 0.0 | 100.0 | percent |
| Visibility | VL Use Gradient | vl_use_gradient | 0 | not applicable | not applicable |  |
| Visibility | VL Color | vl_color | not observed: non-scalar custom control | not applicable | not applicable |  |
| Visibility | VL Additive | vl_additive | 0 | not applicable | not applicable |  |
| Visibility | VL Adapt Brightness | vl_adapt_brightness | 1 | not applicable | not applicable |  |
| Shadow | Shadow | shadow_2 | None | not applicable | not applicable | options: None, Shadow Maps (Soft), Raytraced (Hard), Area |
| Shadow | Shadow Density | shadow_density | 100.0 | 0.0 | 10000.0 | percent |
| Shadow | Shadow Color | shadow_color | [0.0, 0.0, 0.0] | [0.0, 0.0, 0.0] | [1.0, 1.0, 1.0] |  |
| Shadow | Shadow Transparency | shadow_transparency | 1 | not applicable | not applicable |  |
| Shadow | Shadow Clipping Influence | shadow_clipping_influence | 0 | not applicable | not applicable |  |
| Shadow | Shadow Map | shadow_map | 250x250 | not applicable | not applicable | hidden control; options: 250x250, 500x500, 750x750, 1000x1000, 1250x1250, 1500x1500, 1750x1750, 2000x2000, Custom |
| Shadow | Shadow Map Size X | shadow_map_size_x | 250 | 40 | 10000 | hidden control |
| Shadow | Shadow Map Size Y | shadow_map_size_y | 250 | 40 | 10000 | hidden control |
| Shadow | Shadow Sample Radius | shadow_sample_radius | 3 | 1 | 20 | hidden control |
| Shadow | Sampling Radius Boost | sampling_radius_boost | None | not applicable | not applicable | hidden control; options: None, Low (2x), Medium (4x), High (8x) |
| Shadow | Shadow Absolute Bias | shadow_absolute_bias | 1 | not applicable | not applicable | hidden control |
| Shadow | Shadow Bias (Rel) | shadow_bias_rel | 1.0 | 0.0 | 100.0 | percent; hidden control |
| Shadow | Shadow Bias (Abs) | shadow_bias_abs | 2.0 | 0.0 | 999999999.0 | centimeters; hidden control |
| Shadow | Shadow Parallel Width | shadow_parallel_width | 500.0 | -999999999.0 | 999999999.0 | centimeters; hidden control |
| Shadow | Outline Shadow | outline_shadow | 0 | not applicable | not applicable | hidden control |
| Shadow | High Quality | high_quality | 0 | not applicable | not applicable | hidden control |
| Shadow | Shadow Cone | shadow_cone | 0 | not applicable | not applicable | hidden control |
| Shadow | Shadow Cone Angle | shadow_cone_angle | 90.0 | 0.0 | 170.0 | degrees; hidden control |
| Shadow | Shadow Soft Cone | shadow_soft_cone | 1 | not applicable | not applicable | hidden control |
| Shadow | Area Shadow Width | area_shadow_width | 100.0 | -999999999.0 | 999999999.0 | centimeters; hidden control |
| Shadow | Accuracy | accuracy | 75.0 | 0.0 | 100.0 | percent; hidden control |
| Shadow | Minimum Samples | minimum_samples | 8 | 2 | 10000 | hidden control |
| Shadow | Maximum Samples | maximum_samples | 100 | 2 | 10000 | hidden control |
| Photometric | Photometric Intensity | photometric_intensity | 0 | not applicable | not applicable |  |
| Photometric | Intensity | intensity_2 | 1000.0 | 0.0 | 10000000.0 |  |
| Photometric | Units | units | Candela (cd) | not applicable | not applicable | options: Lumen (lm), Candela (cd) |
| Photometric | Photometric Data | photometric_data | 1 | not applicable | not applicable |  |
| Photometric | Filename | filename |  | not applicable | not applicable |  |
| Photometric | Photometric Size | photometric_size | 0 | not applicable | not applicable |  |
| Caustics | Surface Caustics | surface_caustics | 0 | not applicable | not applicable |  |
| Caustics | Surface Energy | surface_energy | 100.0 | 0.0 | 99999999900.0 | percent |
| Caustics | Surface Photons | surface_photons | 10000 | 10 | 10000000 |  |
| Caustics | Volume Caustics | volume_caustics | 0 | not applicable | not applicable |  |
| Caustics | Volume Energy | volume_energy | 100.0 | 0.0 | 99999999900.0 | percent |
| Caustics | Volume Photons | volume_photons | 10000 | 10 | 2000000 |  |
| Caustics | Caustics Falloff | caustics_falloff | None | not applicable | not applicable | options: None, Linear, Inverse, Inverse Square, Inverse Cubic, Step |
| Caustics | Caustics Inner Distance | caustics_inner_distance | 0.0 | 0.0 | 999999999.0 | centimeters |
| Caustics | Caustics Outer Distance | caustics_outer_distance | 1000.0 | 0.0 | 999999999.0 | centimeters |
| Noise | Noise | noise | None | not applicable | not applicable | options: None, Illumination, Visibility, Both |
| Noise | Noise Type | noise_type | Soft Turbulence | not applicable | not applicable | options: Noise, Soft Turbulence, Hard Turbulence, Wavy Turbulence |
| Noise | Noise Octaves | noise_octaves | 4 | 1 | 8 |  |
| Noise | Noise Velocity | noise_velocity | 25.0 | 0.0 | 1000.0 | percent |
| Noise | Noise Brightness | noise_brightness | 0.0 | -1000.0 | 1000.0 | percent |
| Noise | Noise Contrast | noise_contrast | 100.0 | -1000.0 | 1000.0 | percent |
| Noise | Noise Local | noise_local | 1 | not applicable | not applicable |  |
| Noise | Noise Visibility Scale | noise_visibility_scale | [100.0, 100.0, 100.0] | [0.0, 0.0, 0.0] | [999999999.0, 999999999.0, 999999999.0] | centimeters |
| Noise | Noise Illumination Scale | noise_illumination_scale | 4.0 | 0.0 | 999999999.0 |  |
| Noise | Noise Wind | noise_wind | [1.0, 0.0, 0.0] | [-999999999.0, -999999999.0, -999999999.0] | [999999999.0, 999999999.0, 999999999.0] | centimeters |
| Noise | Noise Wind Velocity | noise_wind_velocity | 0.0 | 0.0 | 999999999.0 | centimeters |
| Lens Effects | FX Glow | fx_glow | Inactive | not applicable | not applicable | options: Inactive, Custom, Default, Cinema 4D R4, Wideangle, Zoom, Hi-8, Camcorder, Searchlight, Artifact, Star 1, Star 2, Star 3, Purple, Flashlight, Sun 1, Sun 2, Grey, Blue 1, Blue 2, Red, Yellow-Green 1, Yellow-Green 2, Candle |
| Lens Effects | FX Glow Brightness | fx_glow_brightness | 100.0 | 0.0 | 1000.0 | percent |
| Lens Effects | FX Glow Aspect Ratio | fx_glow_aspect_ratio | 1.0 | 0.1 | 10.0 |  |
| Lens Effects | FX Glow Settings | fx_glow_settings | not observed: non-scalar custom control | not applicable | not applicable |  |
| Lens Effects | FX Reflexes | fx_reflexes | Inactive | not applicable | not applicable | options: Inactive, Custom, Default, Cinema 4D R4, Wideangle, Zoom, Hi-8, Camcorder, Searchlight, Artifact, Star 1, Star 2, Star 3, Purple, Flashlight 1, Flashlight 2, Flashlight 3 |
| Lens Effects | FX Reflection Brightness | fx_reflection_brightness | 100.0 | 0.0 | 1000.0 | percent |
| Lens Effects | FX Reflection Aspect Ratio | fx_reflection_aspect_ratio | 1.0 | 0.1 | 10.0 |  |
| Lens Effects | FX Scale | fx_scale | 100.0 | 0.0 | 1000.0 | percent |
| Lens Effects | FX Rotation | fx_rotation | 0.0 | -57295779455.786545 | 57295779455.786545 | degrees |
| Lens Effects | FX Reference Size | fx_reference_size | 1000.0 | 0.0 | 999999999.0 | centimeters |
| Lens Effects | FX Use Light Parameters | fx_use_light_parameters | 1 | not applicable | not applicable |  |
| Lens Effects | FX Fade if Near Border | fx_fade_if_near_border | 0 | not applicable | not applicable |  |
| Lens Effects | FX Fade if Behind Object | fx_fade_if_behind_object | 1 | not applicable | not applicable |  |
| Lens Effects | FX Fade if Approaching Object | fx_fade_if_approaching_object | 0 | not applicable | not applicable |  |
| Lens Effects | FX Glow Distance Scale | fx_glow_distance_scale | 0 | not applicable | not applicable |  |
| Lens Effects | FX Reflex Distance Scale | fx_reflex_distance_scale | 0 | not applicable | not applicable |  |
| Project | Mode | mode | Exclude | not applicable | not applicable | options: Include, Exclude |
| Project | Objects | objects | not observed: non-scalar custom control | not applicable | not applicable |  |
| Project | PyroCluster Illumination | pyrocluster_illumination | 1 | not applicable | not applicable |  |
| Project | PyroCluster Shadow Casting | pyrocluster_shadow_casting | 1 | not applicable | not applicable |  |

### Type: Area

| Tab / Group | Label | Our key | Default | Min | Max | Units / visibility / options |
|---|---|---|---|---|---|---|
| General | Color | color | [1.0, 1.0, 1.0] | [0.0, 0.0, 0.0] | [1.0, 1.0, 1.0] |  |
| General | Use Temperature | use_temperature | 0 | not applicable | not applicable |  |
| General | Color Temperature | color_temperature | 6500.0 | 1000.0 | 10000.0 |  |
| General | Intensity | intensity | 100.0 | -10000000.0 | 10000000.0 | percent |
| General | Type | type | Area | not applicable | not applicable | options: Omni, Spot, Infinite, Area, Square Spot, Parallel, Parallel Spot, Square Parallel Spot, IES |
| General | Shadow | shadow | None | not applicable | not applicable | options: None, Shadow Maps (Soft), Raytraced (Hard), Area |
| General | Visible Light | visible_light | None | not applicable | not applicable | options: None, Visible, Volumetric, Inverse Volumetric |
| General | No Illumination | no_illumination | 0 | not applicable | not applicable |  |
| General | Show Illumination | show_illumination | 1 | not applicable | not applicable |  |
| General | Ambient Illumination | ambient_illumination | 0 | not applicable | not applicable |  |
| General | Show Visible Light | show_visible_light | 1 | not applicable | not applicable |  |
| General | Diffuse | diffuse | 1 | not applicable | not applicable |  |
| General | Show Clipping | show_clipping | 1 | not applicable | not applicable |  |
| General | Specular | specular | 1 | not applicable | not applicable |  |
| General | Separate Pass | separate_pass | 0 | not applicable | not applicable |  |
| General | GI Illumination | gi_illumination | 1 | not applicable | not applicable |  |
| General | Export to Compositing | export_to_compositing | 1 | not applicable | not applicable |  |
| Details | Use Inner Value | use_inner_value | 1 | not applicable | not applicable |  |
| Details | Inner Angle | inner_angle | 0.0 | 0.0 | 175.0 | degrees; hidden control |
| Details | Inner Radius | inner_radius | 0.0 | 0.0 | 999999999.0 | centimeters; hidden control |
| Details | Outer Angle | outer_angle | 29.999999999999996 | 0.0 | 175.0 | degrees; hidden control |
| Details | Outer Radius | outer_radius | 100.0 | 0.0 | 999999999.0 | centimeters |
| Details | Aspect Ratio | aspect_ratio | 1.0 | 0.01 | 100.0 |  |
| Details | Contrast | contrast | 0.0 | -1000.0 | 1000.0 | percent |
| Details | Shadow Caster | shadow_caster | 0 | not applicable | not applicable |  |
| Details | Area Shape | area_shape | Rectangle | not applicable | not applicable | options: Disc, Rectangle, Line, Sphere, Cylinder, Cylinder (Perpendicular), Cube, Hemisphere, Object / Spline |
| Details | Object | object | none | not applicable | not applicable |  |
| Details | Size X | size_x | 200.0 | 2e-05 | 999999999.0 | centimeters |
| Details | Size Y | size_y | 200.0 | 0.0 | 999999999.0 | centimeters |
| Details | Size Z | size_z | 200.0 | 0.0 | 999999999.0 | centimeters |
| Details | Falloff Angle | falloff_angle | 180.0 | 0.0 | 180.0 | degrees |
| Details | Infinite Angle | infinite_angle | 0.5 | 0.0 | 90.0 | degrees; hidden control |
| Details | Samples | samples | 40 | 16 | 1000 |  |
| Details | Add Grain (Slow) | add_grain_slow | 0 | not applicable | not applicable |  |
| Details | Show in Render | show_in_render | 0 | not applicable | not applicable |  |
| Details | Show as Solid in Viewport | show_as_solid_in_viewport | 0 | not applicable | not applicable |  |
| Details | Show in Specular | show_in_specular | 1 | not applicable | not applicable |  |
| Details | Show in Reflection | show_in_reflection | 0 | not applicable | not applicable |  |
| Details | Visibility Multiplier | visibility_multiplier | 100.0 | 0.0 | 10000000.0 | percent |
| Details | Falloff | falloff | None | not applicable | not applicable | options: None, Inverse Square (Physically Accurate), Linear, Step, Inverse Square Clamped |
| Details | Falloff Inner Radius | falloff_inner_radius | 0.0 | 0.0 | 999999999.0 | centimeters |
| Details | Falloff Radius / Decay | falloff_radius_decay | 500.0 | 0.0 | 999999999.0 | centimeters |
| Details | Colored Edge Falloff | colored_edge_falloff | 0 | not applicable | not applicable |  |
| Details | Z Direction Only | z_direction_only | 0 | not applicable | not applicable |  |
| Details | Use Gradient | use_gradient | 0 | not applicable | not applicable |  |
| Details | Color | color_2 | not observed: non-scalar custom control | not applicable | not applicable |  |
| Details | Use Near Clipping | use_near_clipping | 0 | not applicable | not applicable | hidden control |
| Details | Near Clipping from | near_clipping_from | 0.0 | 0.0 | 999999999.0 | centimeters; hidden control |
| Details | Near Clipping to | near_clipping_to | 10.0 | 0.0 | 999999999.0 | centimeters; hidden control |
| Details | Use Far Clipping | use_far_clipping | 0 | not applicable | not applicable | hidden control |
| Details | Far Clipping from | far_clipping_from | 90.0 | 0.0 | 999999999.0 | centimeters; hidden control |
| Details | Far Clipping to | far_clipping_to | 100.0 | 0.0 | 999999999.0 | centimeters; hidden control |
| Visibility | VL Use Falloff | vl_use_falloff | 1 | not applicable | not applicable |  |
| Visibility | VL Falloff | vl_falloff | 100.0 | 0.0 | 100.0 | percent |
| Visibility | VL Use Edge Falloff | vl_use_edge_falloff | 1 | not applicable | not applicable |  |
| Visibility | VL Edge Falloff | vl_edge_falloff | 50.0 | 0.0 | 100.0 | percent |
| Visibility | VL Colored Edge Falloff | vl_colored_edge_falloff | 0 | not applicable | not applicable |  |
| Visibility | VL Inner Distance | vl_inner_distance | 0.0 | 0.0 | 999999999.0 | centimeters |
| Visibility | VL Outer Distance | vl_outer_distance | 500.0 | 0.0 | 999999999.0 | centimeters |
| Visibility | VL Relative Scale | vl_relative_scale | [100.0, 100.0, 100.0] | [0.0, 0.0, 0.0] | [10000.0, 10000.0, 10000.0] | percent |
| Visibility | VL Sample Distance | vl_sample_distance | 25.0 | 0.0 | 999999999.0 | centimeters |
| Visibility | VL Brightness | vl_brightness | 100.0 | 0.0 | 10000.0 | percent |
| Visibility | VL Dust | vl_dust | 0.0 | 0.0 | 100.0 | percent |
| Visibility | VL Dithering | vl_dithering | 0.0 | 0.0 | 100.0 | percent |
| Visibility | VL Use Gradient | vl_use_gradient | 0 | not applicable | not applicable |  |
| Visibility | VL Color | vl_color | not observed: non-scalar custom control | not applicable | not applicable |  |
| Visibility | VL Additive | vl_additive | 0 | not applicable | not applicable |  |
| Visibility | VL Adapt Brightness | vl_adapt_brightness | 1 | not applicable | not applicable |  |
| Shadow | Shadow | shadow_2 | None | not applicable | not applicable | options: None, Shadow Maps (Soft), Raytraced (Hard), Area |
| Shadow | Shadow Density | shadow_density | 100.0 | 0.0 | 10000.0 | percent |
| Shadow | Shadow Color | shadow_color | [0.0, 0.0, 0.0] | [0.0, 0.0, 0.0] | [1.0, 1.0, 1.0] |  |
| Shadow | Shadow Transparency | shadow_transparency | 1 | not applicable | not applicable |  |
| Shadow | Shadow Clipping Influence | shadow_clipping_influence | 0 | not applicable | not applicable |  |
| Shadow | Shadow Map | shadow_map | 250x250 | not applicable | not applicable | hidden control; options: 250x250, 500x500, 750x750, 1000x1000, 1250x1250, 1500x1500, 1750x1750, 2000x2000, Custom |
| Shadow | Shadow Map Size X | shadow_map_size_x | 250 | 40 | 10000 | hidden control |
| Shadow | Shadow Map Size Y | shadow_map_size_y | 250 | 40 | 10000 | hidden control |
| Shadow | Shadow Sample Radius | shadow_sample_radius | 3 | 1 | 20 | hidden control |
| Shadow | Sampling Radius Boost | sampling_radius_boost | None | not applicable | not applicable | hidden control; options: None, Low (2x), Medium (4x), High (8x) |
| Shadow | Shadow Absolute Bias | shadow_absolute_bias | 1 | not applicable | not applicable | hidden control |
| Shadow | Shadow Bias (Rel) | shadow_bias_rel | 1.0 | 0.0 | 100.0 | percent; hidden control |
| Shadow | Shadow Bias (Abs) | shadow_bias_abs | 2.0 | 0.0 | 999999999.0 | centimeters; hidden control |
| Shadow | Shadow Parallel Width | shadow_parallel_width | 500.0 | -999999999.0 | 999999999.0 | centimeters; hidden control |
| Shadow | Outline Shadow | outline_shadow | 0 | not applicable | not applicable | hidden control |
| Shadow | High Quality | high_quality | 0 | not applicable | not applicable | hidden control |
| Shadow | Shadow Cone | shadow_cone | 0 | not applicable | not applicable | hidden control |
| Shadow | Shadow Cone Angle | shadow_cone_angle | 90.0 | 0.0 | 170.0 | degrees; hidden control |
| Shadow | Shadow Soft Cone | shadow_soft_cone | 1 | not applicable | not applicable | hidden control |
| Shadow | Area Shadow Width | area_shadow_width | 100.0 | -999999999.0 | 999999999.0 | centimeters; hidden control |
| Shadow | Accuracy | accuracy | 75.0 | 0.0 | 100.0 | percent; hidden control |
| Shadow | Minimum Samples | minimum_samples | 8 | 2 | 10000 | hidden control |
| Shadow | Maximum Samples | maximum_samples | 100 | 2 | 10000 | hidden control |
| Photometric | Photometric Intensity | photometric_intensity | 0 | not applicable | not applicable |  |
| Photometric | Intensity | intensity_2 | 1000.0 | 0.0 | 10000000.0 |  |
| Photometric | Units | units | Candela (cd) | not applicable | not applicable | options: Lumen (lm), Candela (cd), Lux (lx) |
| Photometric | Photometric Data | photometric_data | 1 | not applicable | not applicable |  |
| Photometric | Filename | filename |  | not applicable | not applicable |  |
| Photometric | Photometric Size | photometric_size | 0 | not applicable | not applicable |  |
| Caustics | Surface Caustics | surface_caustics | 0 | not applicable | not applicable |  |
| Caustics | Surface Energy | surface_energy | 100.0 | 0.0 | 99999999900.0 | percent |
| Caustics | Surface Photons | surface_photons | 10000 | 10 | 10000000 |  |
| Caustics | Volume Caustics | volume_caustics | 0 | not applicable | not applicable |  |
| Caustics | Volume Energy | volume_energy | 100.0 | 0.0 | 99999999900.0 | percent |
| Caustics | Volume Photons | volume_photons | 10000 | 10 | 2000000 |  |
| Caustics | Caustics Falloff | caustics_falloff | None | not applicable | not applicable | options: None, Linear, Inverse, Inverse Square, Inverse Cubic, Step |
| Caustics | Caustics Inner Distance | caustics_inner_distance | 0.0 | 0.0 | 999999999.0 | centimeters |
| Caustics | Caustics Outer Distance | caustics_outer_distance | 1000.0 | 0.0 | 999999999.0 | centimeters |
| Noise | Noise | noise | None | not applicable | not applicable | options: None, Illumination, Visibility, Both |
| Noise | Noise Type | noise_type | Soft Turbulence | not applicable | not applicable | options: Noise, Soft Turbulence, Hard Turbulence, Wavy Turbulence |
| Noise | Noise Octaves | noise_octaves | 4 | 1 | 8 |  |
| Noise | Noise Velocity | noise_velocity | 25.0 | 0.0 | 1000.0 | percent |
| Noise | Noise Brightness | noise_brightness | 0.0 | -1000.0 | 1000.0 | percent |
| Noise | Noise Contrast | noise_contrast | 100.0 | -1000.0 | 1000.0 | percent |
| Noise | Noise Local | noise_local | 1 | not applicable | not applicable |  |
| Noise | Noise Visibility Scale | noise_visibility_scale | [100.0, 100.0, 100.0] | [0.0, 0.0, 0.0] | [999999999.0, 999999999.0, 999999999.0] | centimeters |
| Noise | Noise Illumination Scale | noise_illumination_scale | 4.0 | 0.0 | 999999999.0 |  |
| Noise | Noise Wind | noise_wind | [1.0, 0.0, 0.0] | [-999999999.0, -999999999.0, -999999999.0] | [999999999.0, 999999999.0, 999999999.0] | centimeters |
| Noise | Noise Wind Velocity | noise_wind_velocity | 0.0 | 0.0 | 999999999.0 | centimeters |
| Lens Effects | FX Glow | fx_glow | Inactive | not applicable | not applicable | options: Inactive, Custom, Default, Cinema 4D R4, Wideangle, Zoom, Hi-8, Camcorder, Searchlight, Artifact, Star 1, Star 2, Star 3, Purple, Flashlight, Sun 1, Sun 2, Grey, Blue 1, Blue 2, Red, Yellow-Green 1, Yellow-Green 2, Candle |
| Lens Effects | FX Glow Brightness | fx_glow_brightness | 100.0 | 0.0 | 1000.0 | percent |
| Lens Effects | FX Glow Aspect Ratio | fx_glow_aspect_ratio | 1.0 | 0.1 | 10.0 |  |
| Lens Effects | FX Glow Settings | fx_glow_settings | not observed: non-scalar custom control | not applicable | not applicable |  |
| Lens Effects | FX Reflexes | fx_reflexes | Inactive | not applicable | not applicable | options: Inactive, Custom, Default, Cinema 4D R4, Wideangle, Zoom, Hi-8, Camcorder, Searchlight, Artifact, Star 1, Star 2, Star 3, Purple, Flashlight 1, Flashlight 2, Flashlight 3 |
| Lens Effects | FX Reflection Brightness | fx_reflection_brightness | 100.0 | 0.0 | 1000.0 | percent |
| Lens Effects | FX Reflection Aspect Ratio | fx_reflection_aspect_ratio | 1.0 | 0.1 | 10.0 |  |
| Lens Effects | FX Scale | fx_scale | 100.0 | 0.0 | 1000.0 | percent |
| Lens Effects | FX Rotation | fx_rotation | 0.0 | -57295779455.786545 | 57295779455.786545 | degrees |
| Lens Effects | FX Reference Size | fx_reference_size | 1000.0 | 0.0 | 999999999.0 | centimeters |
| Lens Effects | FX Use Light Parameters | fx_use_light_parameters | 1 | not applicable | not applicable |  |
| Lens Effects | FX Fade if Near Border | fx_fade_if_near_border | 0 | not applicable | not applicable |  |
| Lens Effects | FX Fade if Behind Object | fx_fade_if_behind_object | 1 | not applicable | not applicable |  |
| Lens Effects | FX Fade if Approaching Object | fx_fade_if_approaching_object | 0 | not applicable | not applicable |  |
| Lens Effects | FX Glow Distance Scale | fx_glow_distance_scale | 0 | not applicable | not applicable |  |
| Lens Effects | FX Reflex Distance Scale | fx_reflex_distance_scale | 0 | not applicable | not applicable |  |
| Project | Mode | mode | Exclude | not applicable | not applicable | options: Include, Exclude |
| Project | Objects | objects | not observed: non-scalar custom control | not applicable | not applicable |  |
| Project | PyroCluster Illumination | pyrocluster_illumination | 1 | not applicable | not applicable |  |
| Project | PyroCluster Shadow Casting | pyrocluster_shadow_casting | 1 | not applicable | not applicable |  |

### Type: Square Spot

| Tab / Group | Label | Our key | Default | Min | Max | Units / visibility / options |
|---|---|---|---|---|---|---|
| General | Color | color | [1.0, 1.0, 1.0] | [0.0, 0.0, 0.0] | [1.0, 1.0, 1.0] |  |
| General | Use Temperature | use_temperature | 0 | not applicable | not applicable |  |
| General | Color Temperature | color_temperature | 6500.0 | 1000.0 | 10000.0 |  |
| General | Intensity | intensity | 100.0 | -10000000.0 | 10000000.0 | percent |
| General | Type | type | Square Spot | not applicable | not applicable | options: Omni, Spot, Infinite, Area, Square Spot, Parallel, Parallel Spot, Square Parallel Spot, IES |
| General | Shadow | shadow | None | not applicable | not applicable | options: None, Shadow Maps (Soft), Raytraced (Hard), Area |
| General | Visible Light | visible_light | None | not applicable | not applicable | options: None, Visible, Volumetric, Inverse Volumetric |
| General | No Illumination | no_illumination | 0 | not applicable | not applicable |  |
| General | Show Illumination | show_illumination | 1 | not applicable | not applicable |  |
| General | Ambient Illumination | ambient_illumination | 0 | not applicable | not applicable |  |
| General | Show Visible Light | show_visible_light | 1 | not applicable | not applicable |  |
| General | Diffuse | diffuse | 1 | not applicable | not applicable |  |
| General | Show Clipping | show_clipping | 1 | not applicable | not applicable |  |
| General | Specular | specular | 1 | not applicable | not applicable |  |
| General | Separate Pass | separate_pass | 0 | not applicable | not applicable |  |
| General | GI Illumination | gi_illumination | 1 | not applicable | not applicable |  |
| General | Export to Compositing | export_to_compositing | 1 | not applicable | not applicable |  |
| Details | Use Inner Value | use_inner_value | 1 | not applicable | not applicable |  |
| Details | Inner Angle | inner_angle | 0.0 | 0.0 | 175.0 | degrees |
| Details | Inner Radius | inner_radius | 0.0 | 0.0 | 999999999.0 | centimeters; hidden control |
| Details | Outer Angle | outer_angle | 29.999999999999996 | 0.0 | 175.0 | degrees |
| Details | Outer Radius | outer_radius | 100.0 | 0.0 | 999999999.0 | centimeters; hidden control |
| Details | Aspect Ratio | aspect_ratio | 1.0 | 0.01 | 100.0 |  |
| Details | Contrast | contrast | 0.0 | -1000.0 | 1000.0 | percent |
| Details | Shadow Caster | shadow_caster | 0 | not applicable | not applicable |  |
| Details | Area Shape | area_shape | Rectangle | not applicable | not applicable | options: Disc, Rectangle, Line, Sphere, Cylinder, Cylinder (Perpendicular), Cube, Hemisphere, Object / Spline |
| Details | Object | object | none | not applicable | not applicable |  |
| Details | Size X | size_x | 200.0 | 2e-05 | 999999999.0 | centimeters |
| Details | Size Y | size_y | 200.0 | 0.0 | 999999999.0 | centimeters |
| Details | Size Z | size_z | 200.0 | 0.0 | 999999999.0 | centimeters |
| Details | Falloff Angle | falloff_angle | 180.0 | 0.0 | 180.0 | degrees |
| Details | Infinite Angle | infinite_angle | 0.5 | 0.0 | 90.0 | degrees; hidden control |
| Details | Samples | samples | 40 | 16 | 1000 |  |
| Details | Add Grain (Slow) | add_grain_slow | 0 | not applicable | not applicable |  |
| Details | Show in Render | show_in_render | 0 | not applicable | not applicable |  |
| Details | Show as Solid in Viewport | show_as_solid_in_viewport | 0 | not applicable | not applicable |  |
| Details | Show in Specular | show_in_specular | 1 | not applicable | not applicable |  |
| Details | Show in Reflection | show_in_reflection | 0 | not applicable | not applicable |  |
| Details | Visibility Multiplier | visibility_multiplier | 100.0 | 0.0 | 10000000.0 | percent |
| Details | Falloff | falloff | None | not applicable | not applicable | options: None, Inverse Square (Physically Accurate), Linear, Step, Inverse Square Clamped |
| Details | Falloff Inner Radius | falloff_inner_radius | 0.0 | 0.0 | 999999999.0 | centimeters |
| Details | Falloff Radius / Decay | falloff_radius_decay | 500.0 | 0.0 | 999999999.0 | centimeters |
| Details | Colored Edge Falloff | colored_edge_falloff | 0 | not applicable | not applicable |  |
| Details | Z Direction Only | z_direction_only | 0 | not applicable | not applicable |  |
| Details | Use Gradient | use_gradient | 0 | not applicable | not applicable |  |
| Details | Color | color_2 | not observed: non-scalar custom control | not applicable | not applicable |  |
| Details | Use Near Clipping | use_near_clipping | 0 | not applicable | not applicable |  |
| Details | Near Clipping from | near_clipping_from | 0.0 | 0.0 | 999999999.0 | centimeters |
| Details | Near Clipping to | near_clipping_to | 10.0 | 0.0 | 999999999.0 | centimeters |
| Details | Use Far Clipping | use_far_clipping | 0 | not applicable | not applicable |  |
| Details | Far Clipping from | far_clipping_from | 90.0 | 0.0 | 999999999.0 | centimeters |
| Details | Far Clipping to | far_clipping_to | 100.0 | 0.0 | 999999999.0 | centimeters |
| Visibility | VL Use Falloff | vl_use_falloff | 1 | not applicable | not applicable |  |
| Visibility | VL Falloff | vl_falloff | 100.0 | 0.0 | 100.0 | percent |
| Visibility | VL Use Edge Falloff | vl_use_edge_falloff | 1 | not applicable | not applicable |  |
| Visibility | VL Edge Falloff | vl_edge_falloff | 50.0 | 0.0 | 100.0 | percent |
| Visibility | VL Colored Edge Falloff | vl_colored_edge_falloff | 0 | not applicable | not applicable |  |
| Visibility | VL Inner Distance | vl_inner_distance | 0.0 | 0.0 | 999999999.0 | centimeters |
| Visibility | VL Outer Distance | vl_outer_distance | 500.0 | 0.0 | 999999999.0 | centimeters |
| Visibility | VL Relative Scale | vl_relative_scale | [100.0, 100.0, 100.0] | [0.0, 0.0, 0.0] | [10000.0, 10000.0, 10000.0] | percent |
| Visibility | VL Sample Distance | vl_sample_distance | 25.0 | 0.0 | 999999999.0 | centimeters |
| Visibility | VL Brightness | vl_brightness | 100.0 | 0.0 | 10000.0 | percent |
| Visibility | VL Dust | vl_dust | 0.0 | 0.0 | 100.0 | percent |
| Visibility | VL Dithering | vl_dithering | 0.0 | 0.0 | 100.0 | percent |
| Visibility | VL Use Gradient | vl_use_gradient | 0 | not applicable | not applicable |  |
| Visibility | VL Color | vl_color | not observed: non-scalar custom control | not applicable | not applicable |  |
| Visibility | VL Additive | vl_additive | 0 | not applicable | not applicable |  |
| Visibility | VL Adapt Brightness | vl_adapt_brightness | 1 | not applicable | not applicable |  |
| Shadow | Shadow | shadow_2 | None | not applicable | not applicable | options: None, Shadow Maps (Soft), Raytraced (Hard), Area |
| Shadow | Shadow Density | shadow_density | 100.0 | 0.0 | 10000.0 | percent |
| Shadow | Shadow Color | shadow_color | [0.0, 0.0, 0.0] | [0.0, 0.0, 0.0] | [1.0, 1.0, 1.0] |  |
| Shadow | Shadow Transparency | shadow_transparency | 1 | not applicable | not applicable |  |
| Shadow | Shadow Clipping Influence | shadow_clipping_influence | 0 | not applicable | not applicable |  |
| Shadow | Shadow Map | shadow_map | 250x250 | not applicable | not applicable | hidden control; options: 250x250, 500x500, 750x750, 1000x1000, 1250x1250, 1500x1500, 1750x1750, 2000x2000, Custom |
| Shadow | Shadow Map Size X | shadow_map_size_x | 250 | 40 | 10000 | hidden control |
| Shadow | Shadow Map Size Y | shadow_map_size_y | 250 | 40 | 10000 | hidden control |
| Shadow | Shadow Sample Radius | shadow_sample_radius | 3 | 1 | 20 | hidden control |
| Shadow | Sampling Radius Boost | sampling_radius_boost | None | not applicable | not applicable | hidden control; options: None, Low (2x), Medium (4x), High (8x) |
| Shadow | Shadow Absolute Bias | shadow_absolute_bias | 1 | not applicable | not applicable | hidden control |
| Shadow | Shadow Bias (Rel) | shadow_bias_rel | 1.0 | 0.0 | 100.0 | percent; hidden control |
| Shadow | Shadow Bias (Abs) | shadow_bias_abs | 2.0 | 0.0 | 999999999.0 | centimeters; hidden control |
| Shadow | Shadow Parallel Width | shadow_parallel_width | 500.0 | -999999999.0 | 999999999.0 | centimeters; hidden control |
| Shadow | Outline Shadow | outline_shadow | 0 | not applicable | not applicable | hidden control |
| Shadow | High Quality | high_quality | 0 | not applicable | not applicable | hidden control |
| Shadow | Shadow Cone | shadow_cone | 0 | not applicable | not applicable | hidden control |
| Shadow | Shadow Cone Angle | shadow_cone_angle | 90.0 | 0.0 | 170.0 | degrees; hidden control |
| Shadow | Shadow Soft Cone | shadow_soft_cone | 1 | not applicable | not applicable | hidden control |
| Shadow | Area Shadow Width | area_shadow_width | 100.0 | -999999999.0 | 999999999.0 | centimeters; hidden control |
| Shadow | Accuracy | accuracy | 75.0 | 0.0 | 100.0 | percent; hidden control |
| Shadow | Minimum Samples | minimum_samples | 8 | 2 | 10000 | hidden control |
| Shadow | Maximum Samples | maximum_samples | 100 | 2 | 10000 | hidden control |
| Photometric | Photometric Intensity | photometric_intensity | 0 | not applicable | not applicable |  |
| Photometric | Intensity | intensity_2 | 1000.0 | 0.0 | 10000000.0 |  |
| Photometric | Units | units | Candela (cd) | not applicable | not applicable | options: Lumen (lm), Candela (cd) |
| Photometric | Photometric Data | photometric_data | 1 | not applicable | not applicable |  |
| Photometric | Filename | filename |  | not applicable | not applicable |  |
| Photometric | Photometric Size | photometric_size | 0 | not applicable | not applicable |  |
| Caustics | Surface Caustics | surface_caustics | 0 | not applicable | not applicable |  |
| Caustics | Surface Energy | surface_energy | 100.0 | 0.0 | 99999999900.0 | percent |
| Caustics | Surface Photons | surface_photons | 10000 | 10 | 10000000 |  |
| Caustics | Volume Caustics | volume_caustics | 0 | not applicable | not applicable |  |
| Caustics | Volume Energy | volume_energy | 100.0 | 0.0 | 99999999900.0 | percent |
| Caustics | Volume Photons | volume_photons | 10000 | 10 | 2000000 |  |
| Caustics | Caustics Falloff | caustics_falloff | None | not applicable | not applicable | options: None, Linear, Inverse, Inverse Square, Inverse Cubic, Step |
| Caustics | Caustics Inner Distance | caustics_inner_distance | 0.0 | 0.0 | 999999999.0 | centimeters |
| Caustics | Caustics Outer Distance | caustics_outer_distance | 1000.0 | 0.0 | 999999999.0 | centimeters |
| Noise | Noise | noise | None | not applicable | not applicable | options: None, Illumination, Visibility, Both |
| Noise | Noise Type | noise_type | Soft Turbulence | not applicable | not applicable | options: Noise, Soft Turbulence, Hard Turbulence, Wavy Turbulence |
| Noise | Noise Octaves | noise_octaves | 4 | 1 | 8 |  |
| Noise | Noise Velocity | noise_velocity | 25.0 | 0.0 | 1000.0 | percent |
| Noise | Noise Brightness | noise_brightness | 0.0 | -1000.0 | 1000.0 | percent |
| Noise | Noise Contrast | noise_contrast | 100.0 | -1000.0 | 1000.0 | percent |
| Noise | Noise Local | noise_local | 1 | not applicable | not applicable |  |
| Noise | Noise Visibility Scale | noise_visibility_scale | [100.0, 100.0, 100.0] | [0.0, 0.0, 0.0] | [999999999.0, 999999999.0, 999999999.0] | centimeters |
| Noise | Noise Illumination Scale | noise_illumination_scale | 4.0 | 0.0 | 999999999.0 |  |
| Noise | Noise Wind | noise_wind | [1.0, 0.0, 0.0] | [-999999999.0, -999999999.0, -999999999.0] | [999999999.0, 999999999.0, 999999999.0] | centimeters |
| Noise | Noise Wind Velocity | noise_wind_velocity | 0.0 | 0.0 | 999999999.0 | centimeters |
| Lens Effects | FX Glow | fx_glow | Inactive | not applicable | not applicable | options: Inactive, Custom, Default, Cinema 4D R4, Wideangle, Zoom, Hi-8, Camcorder, Searchlight, Artifact, Star 1, Star 2, Star 3, Purple, Flashlight, Sun 1, Sun 2, Grey, Blue 1, Blue 2, Red, Yellow-Green 1, Yellow-Green 2, Candle |
| Lens Effects | FX Glow Brightness | fx_glow_brightness | 100.0 | 0.0 | 1000.0 | percent |
| Lens Effects | FX Glow Aspect Ratio | fx_glow_aspect_ratio | 1.0 | 0.1 | 10.0 |  |
| Lens Effects | FX Glow Settings | fx_glow_settings | not observed: non-scalar custom control | not applicable | not applicable |  |
| Lens Effects | FX Reflexes | fx_reflexes | Inactive | not applicable | not applicable | options: Inactive, Custom, Default, Cinema 4D R4, Wideangle, Zoom, Hi-8, Camcorder, Searchlight, Artifact, Star 1, Star 2, Star 3, Purple, Flashlight 1, Flashlight 2, Flashlight 3 |
| Lens Effects | FX Reflection Brightness | fx_reflection_brightness | 100.0 | 0.0 | 1000.0 | percent |
| Lens Effects | FX Reflection Aspect Ratio | fx_reflection_aspect_ratio | 1.0 | 0.1 | 10.0 |  |
| Lens Effects | FX Scale | fx_scale | 100.0 | 0.0 | 1000.0 | percent |
| Lens Effects | FX Rotation | fx_rotation | 0.0 | -57295779455.786545 | 57295779455.786545 | degrees |
| Lens Effects | FX Reference Size | fx_reference_size | 1000.0 | 0.0 | 999999999.0 | centimeters |
| Lens Effects | FX Use Light Parameters | fx_use_light_parameters | 1 | not applicable | not applicable |  |
| Lens Effects | FX Fade if Near Border | fx_fade_if_near_border | 0 | not applicable | not applicable |  |
| Lens Effects | FX Fade if Behind Object | fx_fade_if_behind_object | 1 | not applicable | not applicable |  |
| Lens Effects | FX Fade if Approaching Object | fx_fade_if_approaching_object | 0 | not applicable | not applicable |  |
| Lens Effects | FX Glow Distance Scale | fx_glow_distance_scale | 0 | not applicable | not applicable |  |
| Lens Effects | FX Reflex Distance Scale | fx_reflex_distance_scale | 0 | not applicable | not applicable |  |
| Project | Mode | mode | Exclude | not applicable | not applicable | options: Include, Exclude |
| Project | Objects | objects | not observed: non-scalar custom control | not applicable | not applicable |  |
| Project | PyroCluster Illumination | pyrocluster_illumination | 1 | not applicable | not applicable |  |
| Project | PyroCluster Shadow Casting | pyrocluster_shadow_casting | 1 | not applicable | not applicable |  |

### Type: Parallel

| Tab / Group | Label | Our key | Default | Min | Max | Units / visibility / options |
|---|---|---|---|---|---|---|
| General | Color | color | [1.0, 1.0, 1.0] | [0.0, 0.0, 0.0] | [1.0, 1.0, 1.0] |  |
| General | Use Temperature | use_temperature | 0 | not applicable | not applicable |  |
| General | Color Temperature | color_temperature | 6500.0 | 1000.0 | 10000.0 |  |
| General | Intensity | intensity | 100.0 | -10000000.0 | 10000000.0 | percent |
| General | Type | type | Parallel | not applicable | not applicable | options: Omni, Spot, Infinite, Area, Square Spot, Parallel, Parallel Spot, Square Parallel Spot, IES |
| General | Shadow | shadow | None | not applicable | not applicable | options: None, Shadow Maps (Soft), Raytraced (Hard), Area |
| General | Visible Light | visible_light | None | not applicable | not applicable | options: None, Visible, Volumetric, Inverse Volumetric |
| General | No Illumination | no_illumination | 0 | not applicable | not applicable |  |
| General | Show Illumination | show_illumination | 1 | not applicable | not applicable |  |
| General | Ambient Illumination | ambient_illumination | 0 | not applicable | not applicable |  |
| General | Show Visible Light | show_visible_light | 1 | not applicable | not applicable |  |
| General | Diffuse | diffuse | 1 | not applicable | not applicable |  |
| General | Show Clipping | show_clipping | 1 | not applicable | not applicable |  |
| General | Specular | specular | 1 | not applicable | not applicable |  |
| General | Separate Pass | separate_pass | 0 | not applicable | not applicable |  |
| General | GI Illumination | gi_illumination | 1 | not applicable | not applicable |  |
| General | Export to Compositing | export_to_compositing | 1 | not applicable | not applicable |  |
| Details | Use Inner Value | use_inner_value | 1 | not applicable | not applicable |  |
| Details | Inner Angle | inner_angle | 0.0 | 0.0 | 175.0 | degrees |
| Details | Inner Radius | inner_radius | 0.0 | 0.0 | 999999999.0 | centimeters; hidden control |
| Details | Outer Angle | outer_angle | 29.999999999999996 | 0.0 | 175.0 | degrees |
| Details | Outer Radius | outer_radius | 100.0 | 0.0 | 999999999.0 | centimeters; hidden control |
| Details | Aspect Ratio | aspect_ratio | 1.0 | 0.01 | 100.0 |  |
| Details | Contrast | contrast | 0.0 | -1000.0 | 1000.0 | percent |
| Details | Shadow Caster | shadow_caster | 0 | not applicable | not applicable |  |
| Details | Area Shape | area_shape | Rectangle | not applicable | not applicable | options: Disc, Rectangle, Line, Sphere, Cylinder, Cylinder (Perpendicular), Cube, Hemisphere, Object / Spline |
| Details | Object | object | none | not applicable | not applicable |  |
| Details | Size X | size_x | 200.0 | 2e-05 | 999999999.0 | centimeters |
| Details | Size Y | size_y | 200.0 | 0.0 | 999999999.0 | centimeters |
| Details | Size Z | size_z | 200.0 | 0.0 | 999999999.0 | centimeters |
| Details | Falloff Angle | falloff_angle | 180.0 | 0.0 | 180.0 | degrees |
| Details | Infinite Angle | infinite_angle | 0.5 | 0.0 | 90.0 | degrees; hidden control |
| Details | Samples | samples | 40 | 16 | 1000 |  |
| Details | Add Grain (Slow) | add_grain_slow | 0 | not applicable | not applicable |  |
| Details | Show in Render | show_in_render | 0 | not applicable | not applicable |  |
| Details | Show as Solid in Viewport | show_as_solid_in_viewport | 0 | not applicable | not applicable |  |
| Details | Show in Specular | show_in_specular | 1 | not applicable | not applicable |  |
| Details | Show in Reflection | show_in_reflection | 0 | not applicable | not applicable |  |
| Details | Visibility Multiplier | visibility_multiplier | 100.0 | 0.0 | 10000000.0 | percent |
| Details | Falloff | falloff | None | not applicable | not applicable | options: None, Inverse Square (Physically Accurate), Linear, Step, Inverse Square Clamped |
| Details | Falloff Inner Radius | falloff_inner_radius | 0.0 | 0.0 | 999999999.0 | centimeters |
| Details | Falloff Radius / Decay | falloff_radius_decay | 500.0 | 0.0 | 999999999.0 | centimeters |
| Details | Colored Edge Falloff | colored_edge_falloff | 0 | not applicable | not applicable |  |
| Details | Z Direction Only | z_direction_only | 0 | not applicable | not applicable |  |
| Details | Use Gradient | use_gradient | 0 | not applicable | not applicable |  |
| Details | Color | color_2 | not observed: non-scalar custom control | not applicable | not applicable |  |
| Details | Use Near Clipping | use_near_clipping | 0 | not applicable | not applicable |  |
| Details | Near Clipping from | near_clipping_from | 0.0 | 0.0 | 999999999.0 | centimeters |
| Details | Near Clipping to | near_clipping_to | 10.0 | 0.0 | 999999999.0 | centimeters |
| Details | Use Far Clipping | use_far_clipping | 0 | not applicable | not applicable |  |
| Details | Far Clipping from | far_clipping_from | 90.0 | 0.0 | 999999999.0 | centimeters |
| Details | Far Clipping to | far_clipping_to | 100.0 | 0.0 | 999999999.0 | centimeters |
| Visibility | VL Use Falloff | vl_use_falloff | 1 | not applicable | not applicable |  |
| Visibility | VL Falloff | vl_falloff | 100.0 | 0.0 | 100.0 | percent |
| Visibility | VL Use Edge Falloff | vl_use_edge_falloff | 1 | not applicable | not applicable |  |
| Visibility | VL Edge Falloff | vl_edge_falloff | 50.0 | 0.0 | 100.0 | percent |
| Visibility | VL Colored Edge Falloff | vl_colored_edge_falloff | 0 | not applicable | not applicable |  |
| Visibility | VL Inner Distance | vl_inner_distance | 0.0 | 0.0 | 999999999.0 | centimeters |
| Visibility | VL Outer Distance | vl_outer_distance | 500.0 | 0.0 | 999999999.0 | centimeters |
| Visibility | VL Relative Scale | vl_relative_scale | [100.0, 100.0, 100.0] | [0.0, 0.0, 0.0] | [10000.0, 10000.0, 10000.0] | percent |
| Visibility | VL Sample Distance | vl_sample_distance | 25.0 | 0.0 | 999999999.0 | centimeters |
| Visibility | VL Brightness | vl_brightness | 100.0 | 0.0 | 10000.0 | percent |
| Visibility | VL Dust | vl_dust | 0.0 | 0.0 | 100.0 | percent |
| Visibility | VL Dithering | vl_dithering | 0.0 | 0.0 | 100.0 | percent |
| Visibility | VL Use Gradient | vl_use_gradient | 0 | not applicable | not applicable |  |
| Visibility | VL Color | vl_color | not observed: non-scalar custom control | not applicable | not applicable |  |
| Visibility | VL Additive | vl_additive | 0 | not applicable | not applicable |  |
| Visibility | VL Adapt Brightness | vl_adapt_brightness | 1 | not applicable | not applicable |  |
| Shadow | Shadow | shadow_2 | None | not applicable | not applicable | options: None, Shadow Maps (Soft), Raytraced (Hard), Area |
| Shadow | Shadow Density | shadow_density | 100.0 | 0.0 | 10000.0 | percent |
| Shadow | Shadow Color | shadow_color | [0.0, 0.0, 0.0] | [0.0, 0.0, 0.0] | [1.0, 1.0, 1.0] |  |
| Shadow | Shadow Transparency | shadow_transparency | 1 | not applicable | not applicable |  |
| Shadow | Shadow Clipping Influence | shadow_clipping_influence | 0 | not applicable | not applicable |  |
| Shadow | Shadow Map | shadow_map | 250x250 | not applicable | not applicable | hidden control; options: 250x250, 500x500, 750x750, 1000x1000, 1250x1250, 1500x1500, 1750x1750, 2000x2000, Custom |
| Shadow | Shadow Map Size X | shadow_map_size_x | 250 | 40 | 10000 | hidden control |
| Shadow | Shadow Map Size Y | shadow_map_size_y | 250 | 40 | 10000 | hidden control |
| Shadow | Shadow Sample Radius | shadow_sample_radius | 3 | 1 | 20 | hidden control |
| Shadow | Sampling Radius Boost | sampling_radius_boost | None | not applicable | not applicable | hidden control; options: None, Low (2x), Medium (4x), High (8x) |
| Shadow | Shadow Absolute Bias | shadow_absolute_bias | 1 | not applicable | not applicable | hidden control |
| Shadow | Shadow Bias (Rel) | shadow_bias_rel | 1.0 | 0.0 | 100.0 | percent; hidden control |
| Shadow | Shadow Bias (Abs) | shadow_bias_abs | 2.0 | 0.0 | 999999999.0 | centimeters; hidden control |
| Shadow | Shadow Parallel Width | shadow_parallel_width | 500.0 | -999999999.0 | 999999999.0 | centimeters; hidden control |
| Shadow | Outline Shadow | outline_shadow | 0 | not applicable | not applicable | hidden control |
| Shadow | High Quality | high_quality | 0 | not applicable | not applicable | hidden control |
| Shadow | Shadow Cone | shadow_cone | 0 | not applicable | not applicable | hidden control |
| Shadow | Shadow Cone Angle | shadow_cone_angle | 90.0 | 0.0 | 170.0 | degrees; hidden control |
| Shadow | Shadow Soft Cone | shadow_soft_cone | 1 | not applicable | not applicable | hidden control |
| Shadow | Area Shadow Width | area_shadow_width | 100.0 | -999999999.0 | 999999999.0 | centimeters; hidden control |
| Shadow | Accuracy | accuracy | 75.0 | 0.0 | 100.0 | percent; hidden control |
| Shadow | Minimum Samples | minimum_samples | 8 | 2 | 10000 | hidden control |
| Shadow | Maximum Samples | maximum_samples | 100 | 2 | 10000 | hidden control |
| Photometric | Photometric Intensity | photometric_intensity | 0 | not applicable | not applicable |  |
| Photometric | Intensity | intensity_2 | 1000.0 | 0.0 | 10000000.0 |  |
| Photometric | Units | units | Candela (cd) | not applicable | not applicable | options: Lumen (lm), Candela (cd) |
| Photometric | Photometric Data | photometric_data | 1 | not applicable | not applicable |  |
| Photometric | Filename | filename |  | not applicable | not applicable |  |
| Photometric | Photometric Size | photometric_size | 0 | not applicable | not applicable |  |
| Caustics | Surface Caustics | surface_caustics | 0 | not applicable | not applicable |  |
| Caustics | Surface Energy | surface_energy | 100.0 | 0.0 | 99999999900.0 | percent |
| Caustics | Surface Photons | surface_photons | 10000 | 10 | 10000000 |  |
| Caustics | Volume Caustics | volume_caustics | 0 | not applicable | not applicable |  |
| Caustics | Volume Energy | volume_energy | 100.0 | 0.0 | 99999999900.0 | percent |
| Caustics | Volume Photons | volume_photons | 10000 | 10 | 2000000 |  |
| Caustics | Caustics Falloff | caustics_falloff | None | not applicable | not applicable | options: None, Linear, Inverse, Inverse Square, Inverse Cubic, Step |
| Caustics | Caustics Inner Distance | caustics_inner_distance | 0.0 | 0.0 | 999999999.0 | centimeters |
| Caustics | Caustics Outer Distance | caustics_outer_distance | 1000.0 | 0.0 | 999999999.0 | centimeters |
| Noise | Noise | noise | None | not applicable | not applicable | options: None, Illumination, Visibility, Both |
| Noise | Noise Type | noise_type | Soft Turbulence | not applicable | not applicable | options: Noise, Soft Turbulence, Hard Turbulence, Wavy Turbulence |
| Noise | Noise Octaves | noise_octaves | 4 | 1 | 8 |  |
| Noise | Noise Velocity | noise_velocity | 25.0 | 0.0 | 1000.0 | percent |
| Noise | Noise Brightness | noise_brightness | 0.0 | -1000.0 | 1000.0 | percent |
| Noise | Noise Contrast | noise_contrast | 100.0 | -1000.0 | 1000.0 | percent |
| Noise | Noise Local | noise_local | 1 | not applicable | not applicable |  |
| Noise | Noise Visibility Scale | noise_visibility_scale | [100.0, 100.0, 100.0] | [0.0, 0.0, 0.0] | [999999999.0, 999999999.0, 999999999.0] | centimeters |
| Noise | Noise Illumination Scale | noise_illumination_scale | 4.0 | 0.0 | 999999999.0 |  |
| Noise | Noise Wind | noise_wind | [1.0, 0.0, 0.0] | [-999999999.0, -999999999.0, -999999999.0] | [999999999.0, 999999999.0, 999999999.0] | centimeters |
| Noise | Noise Wind Velocity | noise_wind_velocity | 0.0 | 0.0 | 999999999.0 | centimeters |
| Lens Effects | FX Glow | fx_glow | Inactive | not applicable | not applicable | options: Inactive, Custom, Default, Cinema 4D R4, Wideangle, Zoom, Hi-8, Camcorder, Searchlight, Artifact, Star 1, Star 2, Star 3, Purple, Flashlight, Sun 1, Sun 2, Grey, Blue 1, Blue 2, Red, Yellow-Green 1, Yellow-Green 2, Candle |
| Lens Effects | FX Glow Brightness | fx_glow_brightness | 100.0 | 0.0 | 1000.0 | percent |
| Lens Effects | FX Glow Aspect Ratio | fx_glow_aspect_ratio | 1.0 | 0.1 | 10.0 |  |
| Lens Effects | FX Glow Settings | fx_glow_settings | not observed: non-scalar custom control | not applicable | not applicable |  |
| Lens Effects | FX Reflexes | fx_reflexes | Inactive | not applicable | not applicable | options: Inactive, Custom, Default, Cinema 4D R4, Wideangle, Zoom, Hi-8, Camcorder, Searchlight, Artifact, Star 1, Star 2, Star 3, Purple, Flashlight 1, Flashlight 2, Flashlight 3 |
| Lens Effects | FX Reflection Brightness | fx_reflection_brightness | 100.0 | 0.0 | 1000.0 | percent |
| Lens Effects | FX Reflection Aspect Ratio | fx_reflection_aspect_ratio | 1.0 | 0.1 | 10.0 |  |
| Lens Effects | FX Scale | fx_scale | 100.0 | 0.0 | 1000.0 | percent |
| Lens Effects | FX Rotation | fx_rotation | 0.0 | -57295779455.786545 | 57295779455.786545 | degrees |
| Lens Effects | FX Reference Size | fx_reference_size | 1000.0 | 0.0 | 999999999.0 | centimeters |
| Lens Effects | FX Use Light Parameters | fx_use_light_parameters | 1 | not applicable | not applicable |  |
| Lens Effects | FX Fade if Near Border | fx_fade_if_near_border | 0 | not applicable | not applicable |  |
| Lens Effects | FX Fade if Behind Object | fx_fade_if_behind_object | 1 | not applicable | not applicable |  |
| Lens Effects | FX Fade if Approaching Object | fx_fade_if_approaching_object | 0 | not applicable | not applicable |  |
| Lens Effects | FX Glow Distance Scale | fx_glow_distance_scale | 0 | not applicable | not applicable |  |
| Lens Effects | FX Reflex Distance Scale | fx_reflex_distance_scale | 0 | not applicable | not applicable |  |
| Project | Mode | mode | Exclude | not applicable | not applicable | options: Include, Exclude |
| Project | Objects | objects | not observed: non-scalar custom control | not applicable | not applicable |  |
| Project | PyroCluster Illumination | pyrocluster_illumination | 1 | not applicable | not applicable |  |
| Project | PyroCluster Shadow Casting | pyrocluster_shadow_casting | 1 | not applicable | not applicable |  |

### Type: Parallel Spot

| Tab / Group | Label | Our key | Default | Min | Max | Units / visibility / options |
|---|---|---|---|---|---|---|
| General | Color | color | [1.0, 1.0, 1.0] | [0.0, 0.0, 0.0] | [1.0, 1.0, 1.0] |  |
| General | Use Temperature | use_temperature | 0 | not applicable | not applicable |  |
| General | Color Temperature | color_temperature | 6500.0 | 1000.0 | 10000.0 |  |
| General | Intensity | intensity | 100.0 | -10000000.0 | 10000000.0 | percent |
| General | Type | type | Parallel Spot | not applicable | not applicable | options: Omni, Spot, Infinite, Area, Square Spot, Parallel, Parallel Spot, Square Parallel Spot, IES |
| General | Shadow | shadow | None | not applicable | not applicable | options: None, Shadow Maps (Soft), Raytraced (Hard), Area |
| General | Visible Light | visible_light | None | not applicable | not applicable | options: None, Visible, Volumetric, Inverse Volumetric |
| General | No Illumination | no_illumination | 0 | not applicable | not applicable |  |
| General | Show Illumination | show_illumination | 1 | not applicable | not applicable |  |
| General | Ambient Illumination | ambient_illumination | 0 | not applicable | not applicable |  |
| General | Show Visible Light | show_visible_light | 1 | not applicable | not applicable |  |
| General | Diffuse | diffuse | 1 | not applicable | not applicable |  |
| General | Show Clipping | show_clipping | 1 | not applicable | not applicable |  |
| General | Specular | specular | 1 | not applicable | not applicable |  |
| General | Separate Pass | separate_pass | 0 | not applicable | not applicable |  |
| General | GI Illumination | gi_illumination | 1 | not applicable | not applicable |  |
| General | Export to Compositing | export_to_compositing | 1 | not applicable | not applicable |  |
| Details | Use Inner Value | use_inner_value | 1 | not applicable | not applicable |  |
| Details | Inner Angle | inner_angle | 0.0 | 0.0 | 175.0 | degrees; hidden control |
| Details | Inner Radius | inner_radius | 0.0 | 0.0 | 999999999.0 | centimeters |
| Details | Outer Angle | outer_angle | 29.999999999999996 | 0.0 | 175.0 | degrees; hidden control |
| Details | Outer Radius | outer_radius | 100.0 | 0.0 | 999999999.0 | centimeters |
| Details | Aspect Ratio | aspect_ratio | 1.0 | 0.01 | 100.0 |  |
| Details | Contrast | contrast | 0.0 | -1000.0 | 1000.0 | percent |
| Details | Shadow Caster | shadow_caster | 0 | not applicable | not applicable |  |
| Details | Area Shape | area_shape | Rectangle | not applicable | not applicable | options: Disc, Rectangle, Line, Sphere, Cylinder, Cylinder (Perpendicular), Cube, Hemisphere, Object / Spline |
| Details | Object | object | none | not applicable | not applicable |  |
| Details | Size X | size_x | 200.0 | 2e-05 | 999999999.0 | centimeters |
| Details | Size Y | size_y | 200.0 | 0.0 | 999999999.0 | centimeters |
| Details | Size Z | size_z | 200.0 | 0.0 | 999999999.0 | centimeters |
| Details | Falloff Angle | falloff_angle | 180.0 | 0.0 | 180.0 | degrees |
| Details | Infinite Angle | infinite_angle | 0.5 | 0.0 | 90.0 | degrees; hidden control |
| Details | Samples | samples | 40 | 16 | 1000 |  |
| Details | Add Grain (Slow) | add_grain_slow | 0 | not applicable | not applicable |  |
| Details | Show in Render | show_in_render | 0 | not applicable | not applicable |  |
| Details | Show as Solid in Viewport | show_as_solid_in_viewport | 0 | not applicable | not applicable |  |
| Details | Show in Specular | show_in_specular | 1 | not applicable | not applicable |  |
| Details | Show in Reflection | show_in_reflection | 0 | not applicable | not applicable |  |
| Details | Visibility Multiplier | visibility_multiplier | 100.0 | 0.0 | 10000000.0 | percent |
| Details | Falloff | falloff | None | not applicable | not applicable | options: None, Inverse Square (Physically Accurate), Linear, Step, Inverse Square Clamped |
| Details | Falloff Inner Radius | falloff_inner_radius | 0.0 | 0.0 | 999999999.0 | centimeters |
| Details | Falloff Radius / Decay | falloff_radius_decay | 500.0 | 0.0 | 999999999.0 | centimeters |
| Details | Colored Edge Falloff | colored_edge_falloff | 0 | not applicable | not applicable |  |
| Details | Z Direction Only | z_direction_only | 0 | not applicable | not applicable |  |
| Details | Use Gradient | use_gradient | 0 | not applicable | not applicable |  |
| Details | Color | color_2 | not observed: non-scalar custom control | not applicable | not applicable |  |
| Details | Use Near Clipping | use_near_clipping | 0 | not applicable | not applicable |  |
| Details | Near Clipping from | near_clipping_from | 0.0 | 0.0 | 999999999.0 | centimeters |
| Details | Near Clipping to | near_clipping_to | 10.0 | 0.0 | 999999999.0 | centimeters |
| Details | Use Far Clipping | use_far_clipping | 0 | not applicable | not applicable |  |
| Details | Far Clipping from | far_clipping_from | 90.0 | 0.0 | 999999999.0 | centimeters |
| Details | Far Clipping to | far_clipping_to | 100.0 | 0.0 | 999999999.0 | centimeters |
| Visibility | VL Use Falloff | vl_use_falloff | 1 | not applicable | not applicable |  |
| Visibility | VL Falloff | vl_falloff | 100.0 | 0.0 | 100.0 | percent |
| Visibility | VL Use Edge Falloff | vl_use_edge_falloff | 1 | not applicable | not applicable |  |
| Visibility | VL Edge Falloff | vl_edge_falloff | 50.0 | 0.0 | 100.0 | percent |
| Visibility | VL Colored Edge Falloff | vl_colored_edge_falloff | 0 | not applicable | not applicable |  |
| Visibility | VL Inner Distance | vl_inner_distance | 0.0 | 0.0 | 999999999.0 | centimeters |
| Visibility | VL Outer Distance | vl_outer_distance | 500.0 | 0.0 | 999999999.0 | centimeters |
| Visibility | VL Relative Scale | vl_relative_scale | [100.0, 100.0, 100.0] | [0.0, 0.0, 0.0] | [10000.0, 10000.0, 10000.0] | percent |
| Visibility | VL Sample Distance | vl_sample_distance | 25.0 | 0.0 | 999999999.0 | centimeters |
| Visibility | VL Brightness | vl_brightness | 100.0 | 0.0 | 10000.0 | percent |
| Visibility | VL Dust | vl_dust | 0.0 | 0.0 | 100.0 | percent |
| Visibility | VL Dithering | vl_dithering | 0.0 | 0.0 | 100.0 | percent |
| Visibility | VL Use Gradient | vl_use_gradient | 0 | not applicable | not applicable |  |
| Visibility | VL Color | vl_color | not observed: non-scalar custom control | not applicable | not applicable |  |
| Visibility | VL Additive | vl_additive | 0 | not applicable | not applicable |  |
| Visibility | VL Adapt Brightness | vl_adapt_brightness | 1 | not applicable | not applicable |  |
| Shadow | Shadow | shadow_2 | None | not applicable | not applicable | options: None, Shadow Maps (Soft), Raytraced (Hard), Area |
| Shadow | Shadow Density | shadow_density | 100.0 | 0.0 | 10000.0 | percent |
| Shadow | Shadow Color | shadow_color | [0.0, 0.0, 0.0] | [0.0, 0.0, 0.0] | [1.0, 1.0, 1.0] |  |
| Shadow | Shadow Transparency | shadow_transparency | 1 | not applicable | not applicable |  |
| Shadow | Shadow Clipping Influence | shadow_clipping_influence | 0 | not applicable | not applicable |  |
| Shadow | Shadow Map | shadow_map | 250x250 | not applicable | not applicable | hidden control; options: 250x250, 500x500, 750x750, 1000x1000, 1250x1250, 1500x1500, 1750x1750, 2000x2000, Custom |
| Shadow | Shadow Map Size X | shadow_map_size_x | 250 | 40 | 10000 | hidden control |
| Shadow | Shadow Map Size Y | shadow_map_size_y | 250 | 40 | 10000 | hidden control |
| Shadow | Shadow Sample Radius | shadow_sample_radius | 3 | 1 | 20 | hidden control |
| Shadow | Sampling Radius Boost | sampling_radius_boost | None | not applicable | not applicable | hidden control; options: None, Low (2x), Medium (4x), High (8x) |
| Shadow | Shadow Absolute Bias | shadow_absolute_bias | 1 | not applicable | not applicable | hidden control |
| Shadow | Shadow Bias (Rel) | shadow_bias_rel | 1.0 | 0.0 | 100.0 | percent; hidden control |
| Shadow | Shadow Bias (Abs) | shadow_bias_abs | 2.0 | 0.0 | 999999999.0 | centimeters; hidden control |
| Shadow | Shadow Parallel Width | shadow_parallel_width | 500.0 | -999999999.0 | 999999999.0 | centimeters; hidden control |
| Shadow | Outline Shadow | outline_shadow | 0 | not applicable | not applicable | hidden control |
| Shadow | High Quality | high_quality | 0 | not applicable | not applicable | hidden control |
| Shadow | Shadow Cone | shadow_cone | 0 | not applicable | not applicable | hidden control |
| Shadow | Shadow Cone Angle | shadow_cone_angle | 90.0 | 0.0 | 170.0 | degrees; hidden control |
| Shadow | Shadow Soft Cone | shadow_soft_cone | 1 | not applicable | not applicable | hidden control |
| Shadow | Area Shadow Width | area_shadow_width | 100.0 | -999999999.0 | 999999999.0 | centimeters; hidden control |
| Shadow | Accuracy | accuracy | 75.0 | 0.0 | 100.0 | percent; hidden control |
| Shadow | Minimum Samples | minimum_samples | 8 | 2 | 10000 | hidden control |
| Shadow | Maximum Samples | maximum_samples | 100 | 2 | 10000 | hidden control |
| Photometric | Photometric Intensity | photometric_intensity | 0 | not applicable | not applicable |  |
| Photometric | Intensity | intensity_2 | 1000.0 | 0.0 | 10000000.0 |  |
| Photometric | Units | units | Candela (cd) | not applicable | not applicable | options: Lumen (lm), Candela (cd) |
| Photometric | Photometric Data | photometric_data | 1 | not applicable | not applicable |  |
| Photometric | Filename | filename |  | not applicable | not applicable |  |
| Photometric | Photometric Size | photometric_size | 0 | not applicable | not applicable |  |
| Caustics | Surface Caustics | surface_caustics | 0 | not applicable | not applicable |  |
| Caustics | Surface Energy | surface_energy | 100.0 | 0.0 | 99999999900.0 | percent |
| Caustics | Surface Photons | surface_photons | 10000 | 10 | 10000000 |  |
| Caustics | Volume Caustics | volume_caustics | 0 | not applicable | not applicable |  |
| Caustics | Volume Energy | volume_energy | 100.0 | 0.0 | 99999999900.0 | percent |
| Caustics | Volume Photons | volume_photons | 10000 | 10 | 2000000 |  |
| Caustics | Caustics Falloff | caustics_falloff | None | not applicable | not applicable | options: None, Linear, Inverse, Inverse Square, Inverse Cubic, Step |
| Caustics | Caustics Inner Distance | caustics_inner_distance | 0.0 | 0.0 | 999999999.0 | centimeters |
| Caustics | Caustics Outer Distance | caustics_outer_distance | 1000.0 | 0.0 | 999999999.0 | centimeters |
| Noise | Noise | noise | None | not applicable | not applicable | options: None, Illumination, Visibility, Both |
| Noise | Noise Type | noise_type | Soft Turbulence | not applicable | not applicable | options: Noise, Soft Turbulence, Hard Turbulence, Wavy Turbulence |
| Noise | Noise Octaves | noise_octaves | 4 | 1 | 8 |  |
| Noise | Noise Velocity | noise_velocity | 25.0 | 0.0 | 1000.0 | percent |
| Noise | Noise Brightness | noise_brightness | 0.0 | -1000.0 | 1000.0 | percent |
| Noise | Noise Contrast | noise_contrast | 100.0 | -1000.0 | 1000.0 | percent |
| Noise | Noise Local | noise_local | 1 | not applicable | not applicable |  |
| Noise | Noise Visibility Scale | noise_visibility_scale | [100.0, 100.0, 100.0] | [0.0, 0.0, 0.0] | [999999999.0, 999999999.0, 999999999.0] | centimeters |
| Noise | Noise Illumination Scale | noise_illumination_scale | 4.0 | 0.0 | 999999999.0 |  |
| Noise | Noise Wind | noise_wind | [1.0, 0.0, 0.0] | [-999999999.0, -999999999.0, -999999999.0] | [999999999.0, 999999999.0, 999999999.0] | centimeters |
| Noise | Noise Wind Velocity | noise_wind_velocity | 0.0 | 0.0 | 999999999.0 | centimeters |
| Lens Effects | FX Glow | fx_glow | Inactive | not applicable | not applicable | options: Inactive, Custom, Default, Cinema 4D R4, Wideangle, Zoom, Hi-8, Camcorder, Searchlight, Artifact, Star 1, Star 2, Star 3, Purple, Flashlight, Sun 1, Sun 2, Grey, Blue 1, Blue 2, Red, Yellow-Green 1, Yellow-Green 2, Candle |
| Lens Effects | FX Glow Brightness | fx_glow_brightness | 100.0 | 0.0 | 1000.0 | percent |
| Lens Effects | FX Glow Aspect Ratio | fx_glow_aspect_ratio | 1.0 | 0.1 | 10.0 |  |
| Lens Effects | FX Glow Settings | fx_glow_settings | not observed: non-scalar custom control | not applicable | not applicable |  |
| Lens Effects | FX Reflexes | fx_reflexes | Inactive | not applicable | not applicable | options: Inactive, Custom, Default, Cinema 4D R4, Wideangle, Zoom, Hi-8, Camcorder, Searchlight, Artifact, Star 1, Star 2, Star 3, Purple, Flashlight 1, Flashlight 2, Flashlight 3 |
| Lens Effects | FX Reflection Brightness | fx_reflection_brightness | 100.0 | 0.0 | 1000.0 | percent |
| Lens Effects | FX Reflection Aspect Ratio | fx_reflection_aspect_ratio | 1.0 | 0.1 | 10.0 |  |
| Lens Effects | FX Scale | fx_scale | 100.0 | 0.0 | 1000.0 | percent |
| Lens Effects | FX Rotation | fx_rotation | 0.0 | -57295779455.786545 | 57295779455.786545 | degrees |
| Lens Effects | FX Reference Size | fx_reference_size | 1000.0 | 0.0 | 999999999.0 | centimeters |
| Lens Effects | FX Use Light Parameters | fx_use_light_parameters | 1 | not applicable | not applicable |  |
| Lens Effects | FX Fade if Near Border | fx_fade_if_near_border | 0 | not applicable | not applicable |  |
| Lens Effects | FX Fade if Behind Object | fx_fade_if_behind_object | 1 | not applicable | not applicable |  |
| Lens Effects | FX Fade if Approaching Object | fx_fade_if_approaching_object | 0 | not applicable | not applicable |  |
| Lens Effects | FX Glow Distance Scale | fx_glow_distance_scale | 0 | not applicable | not applicable |  |
| Lens Effects | FX Reflex Distance Scale | fx_reflex_distance_scale | 0 | not applicable | not applicable |  |
| Project | Mode | mode | Exclude | not applicable | not applicable | options: Include, Exclude |
| Project | Objects | objects | not observed: non-scalar custom control | not applicable | not applicable |  |
| Project | PyroCluster Illumination | pyrocluster_illumination | 1 | not applicable | not applicable |  |
| Project | PyroCluster Shadow Casting | pyrocluster_shadow_casting | 1 | not applicable | not applicable |  |

### Type: Square Parallel Spot

| Tab / Group | Label | Our key | Default | Min | Max | Units / visibility / options |
|---|---|---|---|---|---|---|
| General | Color | color | [1.0, 1.0, 1.0] | [0.0, 0.0, 0.0] | [1.0, 1.0, 1.0] |  |
| General | Use Temperature | use_temperature | 0 | not applicable | not applicable |  |
| General | Color Temperature | color_temperature | 6500.0 | 1000.0 | 10000.0 |  |
| General | Intensity | intensity | 100.0 | -10000000.0 | 10000000.0 | percent |
| General | Type | type | Square Parallel Spot | not applicable | not applicable | options: Omni, Spot, Infinite, Area, Square Spot, Parallel, Parallel Spot, Square Parallel Spot, IES |
| General | Shadow | shadow | None | not applicable | not applicable | options: None, Shadow Maps (Soft), Raytraced (Hard), Area |
| General | Visible Light | visible_light | None | not applicable | not applicable | options: None, Visible, Volumetric, Inverse Volumetric |
| General | No Illumination | no_illumination | 0 | not applicable | not applicable |  |
| General | Show Illumination | show_illumination | 1 | not applicable | not applicable |  |
| General | Ambient Illumination | ambient_illumination | 0 | not applicable | not applicable |  |
| General | Show Visible Light | show_visible_light | 1 | not applicable | not applicable |  |
| General | Diffuse | diffuse | 1 | not applicable | not applicable |  |
| General | Show Clipping | show_clipping | 1 | not applicable | not applicable |  |
| General | Specular | specular | 1 | not applicable | not applicable |  |
| General | Separate Pass | separate_pass | 0 | not applicable | not applicable |  |
| General | GI Illumination | gi_illumination | 1 | not applicable | not applicable |  |
| General | Export to Compositing | export_to_compositing | 1 | not applicable | not applicable |  |
| Details | Use Inner Value | use_inner_value | 1 | not applicable | not applicable |  |
| Details | Inner Angle | inner_angle | 0.0 | 0.0 | 175.0 | degrees; hidden control |
| Details | Inner Radius | inner_radius | 0.0 | 0.0 | 999999999.0 | centimeters |
| Details | Outer Angle | outer_angle | 29.999999999999996 | 0.0 | 175.0 | degrees; hidden control |
| Details | Outer Radius | outer_radius | 100.0 | 0.0 | 999999999.0 | centimeters |
| Details | Aspect Ratio | aspect_ratio | 1.0 | 0.01 | 100.0 |  |
| Details | Contrast | contrast | 0.0 | -1000.0 | 1000.0 | percent |
| Details | Shadow Caster | shadow_caster | 0 | not applicable | not applicable |  |
| Details | Area Shape | area_shape | Rectangle | not applicable | not applicable | options: Disc, Rectangle, Line, Sphere, Cylinder, Cylinder (Perpendicular), Cube, Hemisphere, Object / Spline |
| Details | Object | object | none | not applicable | not applicable |  |
| Details | Size X | size_x | 200.0 | 2e-05 | 999999999.0 | centimeters |
| Details | Size Y | size_y | 200.0 | 0.0 | 999999999.0 | centimeters |
| Details | Size Z | size_z | 200.0 | 0.0 | 999999999.0 | centimeters |
| Details | Falloff Angle | falloff_angle | 180.0 | 0.0 | 180.0 | degrees |
| Details | Infinite Angle | infinite_angle | 0.5 | 0.0 | 90.0 | degrees; hidden control |
| Details | Samples | samples | 40 | 16 | 1000 |  |
| Details | Add Grain (Slow) | add_grain_slow | 0 | not applicable | not applicable |  |
| Details | Show in Render | show_in_render | 0 | not applicable | not applicable |  |
| Details | Show as Solid in Viewport | show_as_solid_in_viewport | 0 | not applicable | not applicable |  |
| Details | Show in Specular | show_in_specular | 1 | not applicable | not applicable |  |
| Details | Show in Reflection | show_in_reflection | 0 | not applicable | not applicable |  |
| Details | Visibility Multiplier | visibility_multiplier | 100.0 | 0.0 | 10000000.0 | percent |
| Details | Falloff | falloff | None | not applicable | not applicable | options: None, Inverse Square (Physically Accurate), Linear, Step, Inverse Square Clamped |
| Details | Falloff Inner Radius | falloff_inner_radius | 0.0 | 0.0 | 999999999.0 | centimeters |
| Details | Falloff Radius / Decay | falloff_radius_decay | 500.0 | 0.0 | 999999999.0 | centimeters |
| Details | Colored Edge Falloff | colored_edge_falloff | 0 | not applicable | not applicable |  |
| Details | Z Direction Only | z_direction_only | 0 | not applicable | not applicable |  |
| Details | Use Gradient | use_gradient | 0 | not applicable | not applicable |  |
| Details | Color | color_2 | not observed: non-scalar custom control | not applicable | not applicable |  |
| Details | Use Near Clipping | use_near_clipping | 0 | not applicable | not applicable |  |
| Details | Near Clipping from | near_clipping_from | 0.0 | 0.0 | 999999999.0 | centimeters |
| Details | Near Clipping to | near_clipping_to | 10.0 | 0.0 | 999999999.0 | centimeters |
| Details | Use Far Clipping | use_far_clipping | 0 | not applicable | not applicable |  |
| Details | Far Clipping from | far_clipping_from | 90.0 | 0.0 | 999999999.0 | centimeters |
| Details | Far Clipping to | far_clipping_to | 100.0 | 0.0 | 999999999.0 | centimeters |
| Visibility | VL Use Falloff | vl_use_falloff | 1 | not applicable | not applicable |  |
| Visibility | VL Falloff | vl_falloff | 100.0 | 0.0 | 100.0 | percent |
| Visibility | VL Use Edge Falloff | vl_use_edge_falloff | 1 | not applicable | not applicable |  |
| Visibility | VL Edge Falloff | vl_edge_falloff | 50.0 | 0.0 | 100.0 | percent |
| Visibility | VL Colored Edge Falloff | vl_colored_edge_falloff | 0 | not applicable | not applicable |  |
| Visibility | VL Inner Distance | vl_inner_distance | 0.0 | 0.0 | 999999999.0 | centimeters |
| Visibility | VL Outer Distance | vl_outer_distance | 500.0 | 0.0 | 999999999.0 | centimeters |
| Visibility | VL Relative Scale | vl_relative_scale | [100.0, 100.0, 100.0] | [0.0, 0.0, 0.0] | [10000.0, 10000.0, 10000.0] | percent |
| Visibility | VL Sample Distance | vl_sample_distance | 25.0 | 0.0 | 999999999.0 | centimeters |
| Visibility | VL Brightness | vl_brightness | 100.0 | 0.0 | 10000.0 | percent |
| Visibility | VL Dust | vl_dust | 0.0 | 0.0 | 100.0 | percent |
| Visibility | VL Dithering | vl_dithering | 0.0 | 0.0 | 100.0 | percent |
| Visibility | VL Use Gradient | vl_use_gradient | 0 | not applicable | not applicable |  |
| Visibility | VL Color | vl_color | not observed: non-scalar custom control | not applicable | not applicable |  |
| Visibility | VL Additive | vl_additive | 0 | not applicable | not applicable |  |
| Visibility | VL Adapt Brightness | vl_adapt_brightness | 1 | not applicable | not applicable |  |
| Shadow | Shadow | shadow_2 | None | not applicable | not applicable | options: None, Shadow Maps (Soft), Raytraced (Hard), Area |
| Shadow | Shadow Density | shadow_density | 100.0 | 0.0 | 10000.0 | percent |
| Shadow | Shadow Color | shadow_color | [0.0, 0.0, 0.0] | [0.0, 0.0, 0.0] | [1.0, 1.0, 1.0] |  |
| Shadow | Shadow Transparency | shadow_transparency | 1 | not applicable | not applicable |  |
| Shadow | Shadow Clipping Influence | shadow_clipping_influence | 0 | not applicable | not applicable |  |
| Shadow | Shadow Map | shadow_map | 250x250 | not applicable | not applicable | hidden control; options: 250x250, 500x500, 750x750, 1000x1000, 1250x1250, 1500x1500, 1750x1750, 2000x2000, Custom |
| Shadow | Shadow Map Size X | shadow_map_size_x | 250 | 40 | 10000 | hidden control |
| Shadow | Shadow Map Size Y | shadow_map_size_y | 250 | 40 | 10000 | hidden control |
| Shadow | Shadow Sample Radius | shadow_sample_radius | 3 | 1 | 20 | hidden control |
| Shadow | Sampling Radius Boost | sampling_radius_boost | None | not applicable | not applicable | hidden control; options: None, Low (2x), Medium (4x), High (8x) |
| Shadow | Shadow Absolute Bias | shadow_absolute_bias | 1 | not applicable | not applicable | hidden control |
| Shadow | Shadow Bias (Rel) | shadow_bias_rel | 1.0 | 0.0 | 100.0 | percent; hidden control |
| Shadow | Shadow Bias (Abs) | shadow_bias_abs | 2.0 | 0.0 | 999999999.0 | centimeters; hidden control |
| Shadow | Shadow Parallel Width | shadow_parallel_width | 500.0 | -999999999.0 | 999999999.0 | centimeters; hidden control |
| Shadow | Outline Shadow | outline_shadow | 0 | not applicable | not applicable | hidden control |
| Shadow | High Quality | high_quality | 0 | not applicable | not applicable | hidden control |
| Shadow | Shadow Cone | shadow_cone | 0 | not applicable | not applicable | hidden control |
| Shadow | Shadow Cone Angle | shadow_cone_angle | 90.0 | 0.0 | 170.0 | degrees; hidden control |
| Shadow | Shadow Soft Cone | shadow_soft_cone | 1 | not applicable | not applicable | hidden control |
| Shadow | Area Shadow Width | area_shadow_width | 100.0 | -999999999.0 | 999999999.0 | centimeters; hidden control |
| Shadow | Accuracy | accuracy | 75.0 | 0.0 | 100.0 | percent; hidden control |
| Shadow | Minimum Samples | minimum_samples | 8 | 2 | 10000 | hidden control |
| Shadow | Maximum Samples | maximum_samples | 100 | 2 | 10000 | hidden control |
| Photometric | Photometric Intensity | photometric_intensity | 0 | not applicable | not applicable |  |
| Photometric | Intensity | intensity_2 | 1000.0 | 0.0 | 10000000.0 |  |
| Photometric | Units | units | Candela (cd) | not applicable | not applicable | options: Lumen (lm), Candela (cd) |
| Photometric | Photometric Data | photometric_data | 1 | not applicable | not applicable |  |
| Photometric | Filename | filename |  | not applicable | not applicable |  |
| Photometric | Photometric Size | photometric_size | 0 | not applicable | not applicable |  |
| Caustics | Surface Caustics | surface_caustics | 0 | not applicable | not applicable |  |
| Caustics | Surface Energy | surface_energy | 100.0 | 0.0 | 99999999900.0 | percent |
| Caustics | Surface Photons | surface_photons | 10000 | 10 | 10000000 |  |
| Caustics | Volume Caustics | volume_caustics | 0 | not applicable | not applicable |  |
| Caustics | Volume Energy | volume_energy | 100.0 | 0.0 | 99999999900.0 | percent |
| Caustics | Volume Photons | volume_photons | 10000 | 10 | 2000000 |  |
| Caustics | Caustics Falloff | caustics_falloff | None | not applicable | not applicable | options: None, Linear, Inverse, Inverse Square, Inverse Cubic, Step |
| Caustics | Caustics Inner Distance | caustics_inner_distance | 0.0 | 0.0 | 999999999.0 | centimeters |
| Caustics | Caustics Outer Distance | caustics_outer_distance | 1000.0 | 0.0 | 999999999.0 | centimeters |
| Noise | Noise | noise | None | not applicable | not applicable | options: None, Illumination, Visibility, Both |
| Noise | Noise Type | noise_type | Soft Turbulence | not applicable | not applicable | options: Noise, Soft Turbulence, Hard Turbulence, Wavy Turbulence |
| Noise | Noise Octaves | noise_octaves | 4 | 1 | 8 |  |
| Noise | Noise Velocity | noise_velocity | 25.0 | 0.0 | 1000.0 | percent |
| Noise | Noise Brightness | noise_brightness | 0.0 | -1000.0 | 1000.0 | percent |
| Noise | Noise Contrast | noise_contrast | 100.0 | -1000.0 | 1000.0 | percent |
| Noise | Noise Local | noise_local | 1 | not applicable | not applicable |  |
| Noise | Noise Visibility Scale | noise_visibility_scale | [100.0, 100.0, 100.0] | [0.0, 0.0, 0.0] | [999999999.0, 999999999.0, 999999999.0] | centimeters |
| Noise | Noise Illumination Scale | noise_illumination_scale | 4.0 | 0.0 | 999999999.0 |  |
| Noise | Noise Wind | noise_wind | [1.0, 0.0, 0.0] | [-999999999.0, -999999999.0, -999999999.0] | [999999999.0, 999999999.0, 999999999.0] | centimeters |
| Noise | Noise Wind Velocity | noise_wind_velocity | 0.0 | 0.0 | 999999999.0 | centimeters |
| Lens Effects | FX Glow | fx_glow | Inactive | not applicable | not applicable | options: Inactive, Custom, Default, Cinema 4D R4, Wideangle, Zoom, Hi-8, Camcorder, Searchlight, Artifact, Star 1, Star 2, Star 3, Purple, Flashlight, Sun 1, Sun 2, Grey, Blue 1, Blue 2, Red, Yellow-Green 1, Yellow-Green 2, Candle |
| Lens Effects | FX Glow Brightness | fx_glow_brightness | 100.0 | 0.0 | 1000.0 | percent |
| Lens Effects | FX Glow Aspect Ratio | fx_glow_aspect_ratio | 1.0 | 0.1 | 10.0 |  |
| Lens Effects | FX Glow Settings | fx_glow_settings | not observed: non-scalar custom control | not applicable | not applicable |  |
| Lens Effects | FX Reflexes | fx_reflexes | Inactive | not applicable | not applicable | options: Inactive, Custom, Default, Cinema 4D R4, Wideangle, Zoom, Hi-8, Camcorder, Searchlight, Artifact, Star 1, Star 2, Star 3, Purple, Flashlight 1, Flashlight 2, Flashlight 3 |
| Lens Effects | FX Reflection Brightness | fx_reflection_brightness | 100.0 | 0.0 | 1000.0 | percent |
| Lens Effects | FX Reflection Aspect Ratio | fx_reflection_aspect_ratio | 1.0 | 0.1 | 10.0 |  |
| Lens Effects | FX Scale | fx_scale | 100.0 | 0.0 | 1000.0 | percent |
| Lens Effects | FX Rotation | fx_rotation | 0.0 | -57295779455.786545 | 57295779455.786545 | degrees |
| Lens Effects | FX Reference Size | fx_reference_size | 1000.0 | 0.0 | 999999999.0 | centimeters |
| Lens Effects | FX Use Light Parameters | fx_use_light_parameters | 1 | not applicable | not applicable |  |
| Lens Effects | FX Fade if Near Border | fx_fade_if_near_border | 0 | not applicable | not applicable |  |
| Lens Effects | FX Fade if Behind Object | fx_fade_if_behind_object | 1 | not applicable | not applicable |  |
| Lens Effects | FX Fade if Approaching Object | fx_fade_if_approaching_object | 0 | not applicable | not applicable |  |
| Lens Effects | FX Glow Distance Scale | fx_glow_distance_scale | 0 | not applicable | not applicable |  |
| Lens Effects | FX Reflex Distance Scale | fx_reflex_distance_scale | 0 | not applicable | not applicable |  |
| Project | Mode | mode | Exclude | not applicable | not applicable | options: Include, Exclude |
| Project | Objects | objects | not observed: non-scalar custom control | not applicable | not applicable |  |
| Project | PyroCluster Illumination | pyrocluster_illumination | 1 | not applicable | not applicable |  |
| Project | PyroCluster Shadow Casting | pyrocluster_shadow_casting | 1 | not applicable | not applicable |  |

### Type: IES

| Tab / Group | Label | Our key | Default | Min | Max | Units / visibility / options |
|---|---|---|---|---|---|---|
| General | Color | color | [1.0, 1.0, 1.0] | [0.0, 0.0, 0.0] | [1.0, 1.0, 1.0] |  |
| General | Use Temperature | use_temperature | 0 | not applicable | not applicable |  |
| General | Color Temperature | color_temperature | 6500.0 | 1000.0 | 10000.0 |  |
| General | Intensity | intensity | 100.0 | -10000000.0 | 10000000.0 | percent |
| General | Type | type | IES | not applicable | not applicable | options: Omni, Spot, Infinite, Area, Square Spot, Parallel, Parallel Spot, Square Parallel Spot, IES |
| General | Shadow | shadow | None | not applicable | not applicable | options: None, Shadow Maps (Soft), Raytraced (Hard), Area |
| General | Visible Light | visible_light | None | not applicable | not applicable | options: None, Visible, Volumetric, Inverse Volumetric |
| General | No Illumination | no_illumination | 0 | not applicable | not applicable |  |
| General | Show Illumination | show_illumination | 1 | not applicable | not applicable |  |
| General | Ambient Illumination | ambient_illumination | 0 | not applicable | not applicable |  |
| General | Show Visible Light | show_visible_light | 1 | not applicable | not applicable |  |
| General | Diffuse | diffuse | 1 | not applicable | not applicable |  |
| General | Show Clipping | show_clipping | 1 | not applicable | not applicable |  |
| General | Specular | specular | 1 | not applicable | not applicable |  |
| General | Separate Pass | separate_pass | 0 | not applicable | not applicable |  |
| General | GI Illumination | gi_illumination | 1 | not applicable | not applicable |  |
| General | Export to Compositing | export_to_compositing | 1 | not applicable | not applicable |  |
| Details | Use Inner Value | use_inner_value | 1 | not applicable | not applicable |  |
| Details | Inner Angle | inner_angle | 0.0 | 0.0 | 175.0 | degrees |
| Details | Inner Radius | inner_radius | 0.0 | 0.0 | 999999999.0 | centimeters; hidden control |
| Details | Outer Angle | outer_angle | 29.999999999999996 | 0.0 | 175.0 | degrees |
| Details | Outer Radius | outer_radius | 100.0 | 0.0 | 999999999.0 | centimeters; hidden control |
| Details | Aspect Ratio | aspect_ratio | 1.0 | 0.01 | 100.0 |  |
| Details | Contrast | contrast | 0.0 | -1000.0 | 1000.0 | percent |
| Details | Shadow Caster | shadow_caster | 0 | not applicable | not applicable |  |
| Details | Area Shape | area_shape | Rectangle | not applicable | not applicable | options: Disc, Rectangle, Line, Sphere, Cylinder, Cylinder (Perpendicular), Cube, Hemisphere, Object / Spline |
| Details | Object | object | none | not applicable | not applicable |  |
| Details | Size X | size_x | 200.0 | 2e-05 | 999999999.0 | centimeters |
| Details | Size Y | size_y | 200.0 | 0.0 | 999999999.0 | centimeters |
| Details | Size Z | size_z | 200.0 | 0.0 | 999999999.0 | centimeters |
| Details | Falloff Angle | falloff_angle | 180.0 | 0.0 | 180.0 | degrees |
| Details | Infinite Angle | infinite_angle | 0.5 | 0.0 | 90.0 | degrees; hidden control |
| Details | Samples | samples | 40 | 16 | 1000 |  |
| Details | Add Grain (Slow) | add_grain_slow | 0 | not applicable | not applicable |  |
| Details | Show in Render | show_in_render | 0 | not applicable | not applicable |  |
| Details | Show as Solid in Viewport | show_as_solid_in_viewport | 0 | not applicable | not applicable |  |
| Details | Show in Specular | show_in_specular | 1 | not applicable | not applicable |  |
| Details | Show in Reflection | show_in_reflection | 0 | not applicable | not applicable |  |
| Details | Visibility Multiplier | visibility_multiplier | 100.0 | 0.0 | 10000000.0 | percent |
| Details | Falloff | falloff | None | not applicable | not applicable | options: None, Inverse Square (Physically Accurate), Linear, Step, Inverse Square Clamped |
| Details | Falloff Inner Radius | falloff_inner_radius | 0.0 | 0.0 | 999999999.0 | centimeters |
| Details | Falloff Radius / Decay | falloff_radius_decay | 500.0 | 0.0 | 999999999.0 | centimeters |
| Details | Colored Edge Falloff | colored_edge_falloff | 0 | not applicable | not applicable |  |
| Details | Z Direction Only | z_direction_only | 0 | not applicable | not applicable |  |
| Details | Use Gradient | use_gradient | 0 | not applicable | not applicable |  |
| Details | Color | color_2 | not observed: non-scalar custom control | not applicable | not applicable |  |
| Details | Use Near Clipping | use_near_clipping | 0 | not applicable | not applicable |  |
| Details | Near Clipping from | near_clipping_from | 0.0 | 0.0 | 999999999.0 | centimeters |
| Details | Near Clipping to | near_clipping_to | 10.0 | 0.0 | 999999999.0 | centimeters |
| Details | Use Far Clipping | use_far_clipping | 0 | not applicable | not applicable |  |
| Details | Far Clipping from | far_clipping_from | 90.0 | 0.0 | 999999999.0 | centimeters |
| Details | Far Clipping to | far_clipping_to | 100.0 | 0.0 | 999999999.0 | centimeters |
| Visibility | VL Use Falloff | vl_use_falloff | 1 | not applicable | not applicable |  |
| Visibility | VL Falloff | vl_falloff | 100.0 | 0.0 | 100.0 | percent |
| Visibility | VL Use Edge Falloff | vl_use_edge_falloff | 1 | not applicable | not applicable |  |
| Visibility | VL Edge Falloff | vl_edge_falloff | 50.0 | 0.0 | 100.0 | percent |
| Visibility | VL Colored Edge Falloff | vl_colored_edge_falloff | 0 | not applicable | not applicable |  |
| Visibility | VL Inner Distance | vl_inner_distance | 0.0 | 0.0 | 999999999.0 | centimeters |
| Visibility | VL Outer Distance | vl_outer_distance | 500.0 | 0.0 | 999999999.0 | centimeters |
| Visibility | VL Relative Scale | vl_relative_scale | [100.0, 100.0, 100.0] | [0.0, 0.0, 0.0] | [10000.0, 10000.0, 10000.0] | percent |
| Visibility | VL Sample Distance | vl_sample_distance | 25.0 | 0.0 | 999999999.0 | centimeters |
| Visibility | VL Brightness | vl_brightness | 100.0 | 0.0 | 10000.0 | percent |
| Visibility | VL Dust | vl_dust | 0.0 | 0.0 | 100.0 | percent |
| Visibility | VL Dithering | vl_dithering | 0.0 | 0.0 | 100.0 | percent |
| Visibility | VL Use Gradient | vl_use_gradient | 0 | not applicable | not applicable |  |
| Visibility | VL Color | vl_color | not observed: non-scalar custom control | not applicable | not applicable |  |
| Visibility | VL Additive | vl_additive | 0 | not applicable | not applicable |  |
| Visibility | VL Adapt Brightness | vl_adapt_brightness | 1 | not applicable | not applicable |  |
| Shadow | Shadow | shadow_2 | None | not applicable | not applicable | options: None, Shadow Maps (Soft), Raytraced (Hard), Area |
| Shadow | Shadow Density | shadow_density | 100.0 | 0.0 | 10000.0 | percent |
| Shadow | Shadow Color | shadow_color | [0.0, 0.0, 0.0] | [0.0, 0.0, 0.0] | [1.0, 1.0, 1.0] |  |
| Shadow | Shadow Transparency | shadow_transparency | 1 | not applicable | not applicable |  |
| Shadow | Shadow Clipping Influence | shadow_clipping_influence | 0 | not applicable | not applicable |  |
| Shadow | Shadow Map | shadow_map | 250x250 | not applicable | not applicable | hidden control; options: 250x250, 500x500, 750x750, 1000x1000, 1250x1250, 1500x1500, 1750x1750, 2000x2000, Custom |
| Shadow | Shadow Map Size X | shadow_map_size_x | 250 | 40 | 10000 | hidden control |
| Shadow | Shadow Map Size Y | shadow_map_size_y | 250 | 40 | 10000 | hidden control |
| Shadow | Shadow Sample Radius | shadow_sample_radius | 3 | 1 | 20 | hidden control |
| Shadow | Sampling Radius Boost | sampling_radius_boost | None | not applicable | not applicable | hidden control; options: None, Low (2x), Medium (4x), High (8x) |
| Shadow | Shadow Absolute Bias | shadow_absolute_bias | 1 | not applicable | not applicable | hidden control |
| Shadow | Shadow Bias (Rel) | shadow_bias_rel | 1.0 | 0.0 | 100.0 | percent; hidden control |
| Shadow | Shadow Bias (Abs) | shadow_bias_abs | 2.0 | 0.0 | 999999999.0 | centimeters; hidden control |
| Shadow | Shadow Parallel Width | shadow_parallel_width | 500.0 | -999999999.0 | 999999999.0 | centimeters; hidden control |
| Shadow | Outline Shadow | outline_shadow | 0 | not applicable | not applicable | hidden control |
| Shadow | High Quality | high_quality | 0 | not applicable | not applicable | hidden control |
| Shadow | Shadow Cone | shadow_cone | 0 | not applicable | not applicable | hidden control |
| Shadow | Shadow Cone Angle | shadow_cone_angle | 90.0 | 0.0 | 170.0 | degrees; hidden control |
| Shadow | Shadow Soft Cone | shadow_soft_cone | 1 | not applicable | not applicable | hidden control |
| Shadow | Area Shadow Width | area_shadow_width | 100.0 | -999999999.0 | 999999999.0 | centimeters; hidden control |
| Shadow | Accuracy | accuracy | 75.0 | 0.0 | 100.0 | percent; hidden control |
| Shadow | Minimum Samples | minimum_samples | 8 | 2 | 10000 | hidden control |
| Shadow | Maximum Samples | maximum_samples | 100 | 2 | 10000 | hidden control |
| Photometric | Photometric Intensity | photometric_intensity | 0 | not applicable | not applicable |  |
| Photometric | Intensity | intensity_2 | 1000.0 | 0.0 | 10000000.0 |  |
| Photometric | Units | units | Candela (cd) | not applicable | not applicable | options: Lumen (lm), Candela (cd) |
| Photometric | Photometric Data | photometric_data | 1 | not applicable | not applicable |  |
| Photometric | Filename | filename |  | not applicable | not applicable |  |
| Photometric | Photometric Size | photometric_size | 0 | not applicable | not applicable |  |
| Caustics | Surface Caustics | surface_caustics | 0 | not applicable | not applicable |  |
| Caustics | Surface Energy | surface_energy | 100.0 | 0.0 | 99999999900.0 | percent |
| Caustics | Surface Photons | surface_photons | 10000 | 10 | 10000000 |  |
| Caustics | Volume Caustics | volume_caustics | 0 | not applicable | not applicable |  |
| Caustics | Volume Energy | volume_energy | 100.0 | 0.0 | 99999999900.0 | percent |
| Caustics | Volume Photons | volume_photons | 10000 | 10 | 2000000 |  |
| Caustics | Caustics Falloff | caustics_falloff | None | not applicable | not applicable | options: None, Linear, Inverse, Inverse Square, Inverse Cubic, Step |
| Caustics | Caustics Inner Distance | caustics_inner_distance | 0.0 | 0.0 | 999999999.0 | centimeters |
| Caustics | Caustics Outer Distance | caustics_outer_distance | 1000.0 | 0.0 | 999999999.0 | centimeters |
| Noise | Noise | noise | None | not applicable | not applicable | options: None, Illumination, Visibility, Both |
| Noise | Noise Type | noise_type | Soft Turbulence | not applicable | not applicable | options: Noise, Soft Turbulence, Hard Turbulence, Wavy Turbulence |
| Noise | Noise Octaves | noise_octaves | 4 | 1 | 8 |  |
| Noise | Noise Velocity | noise_velocity | 25.0 | 0.0 | 1000.0 | percent |
| Noise | Noise Brightness | noise_brightness | 0.0 | -1000.0 | 1000.0 | percent |
| Noise | Noise Contrast | noise_contrast | 100.0 | -1000.0 | 1000.0 | percent |
| Noise | Noise Local | noise_local | 1 | not applicable | not applicable |  |
| Noise | Noise Visibility Scale | noise_visibility_scale | [100.0, 100.0, 100.0] | [0.0, 0.0, 0.0] | [999999999.0, 999999999.0, 999999999.0] | centimeters |
| Noise | Noise Illumination Scale | noise_illumination_scale | 4.0 | 0.0 | 999999999.0 |  |
| Noise | Noise Wind | noise_wind | [1.0, 0.0, 0.0] | [-999999999.0, -999999999.0, -999999999.0] | [999999999.0, 999999999.0, 999999999.0] | centimeters |
| Noise | Noise Wind Velocity | noise_wind_velocity | 0.0 | 0.0 | 999999999.0 | centimeters |
| Lens Effects | FX Glow | fx_glow | Inactive | not applicable | not applicable | options: Inactive, Custom, Default, Cinema 4D R4, Wideangle, Zoom, Hi-8, Camcorder, Searchlight, Artifact, Star 1, Star 2, Star 3, Purple, Flashlight, Sun 1, Sun 2, Grey, Blue 1, Blue 2, Red, Yellow-Green 1, Yellow-Green 2, Candle |
| Lens Effects | FX Glow Brightness | fx_glow_brightness | 100.0 | 0.0 | 1000.0 | percent |
| Lens Effects | FX Glow Aspect Ratio | fx_glow_aspect_ratio | 1.0 | 0.1 | 10.0 |  |
| Lens Effects | FX Glow Settings | fx_glow_settings | not observed: non-scalar custom control | not applicable | not applicable |  |
| Lens Effects | FX Reflexes | fx_reflexes | Inactive | not applicable | not applicable | options: Inactive, Custom, Default, Cinema 4D R4, Wideangle, Zoom, Hi-8, Camcorder, Searchlight, Artifact, Star 1, Star 2, Star 3, Purple, Flashlight 1, Flashlight 2, Flashlight 3 |
| Lens Effects | FX Reflection Brightness | fx_reflection_brightness | 100.0 | 0.0 | 1000.0 | percent |
| Lens Effects | FX Reflection Aspect Ratio | fx_reflection_aspect_ratio | 1.0 | 0.1 | 10.0 |  |
| Lens Effects | FX Scale | fx_scale | 100.0 | 0.0 | 1000.0 | percent |
| Lens Effects | FX Rotation | fx_rotation | 0.0 | -57295779455.786545 | 57295779455.786545 | degrees |
| Lens Effects | FX Reference Size | fx_reference_size | 1000.0 | 0.0 | 999999999.0 | centimeters |
| Lens Effects | FX Use Light Parameters | fx_use_light_parameters | 1 | not applicable | not applicable |  |
| Lens Effects | FX Fade if Near Border | fx_fade_if_near_border | 0 | not applicable | not applicable |  |
| Lens Effects | FX Fade if Behind Object | fx_fade_if_behind_object | 1 | not applicable | not applicable |  |
| Lens Effects | FX Fade if Approaching Object | fx_fade_if_approaching_object | 0 | not applicable | not applicable |  |
| Lens Effects | FX Glow Distance Scale | fx_glow_distance_scale | 0 | not applicable | not applicable |  |
| Lens Effects | FX Reflex Distance Scale | fx_reflex_distance_scale | 0 | not applicable | not applicable |  |
| Project | Mode | mode | Exclude | not applicable | not applicable | options: Include, Exclude |
| Project | Objects | objects | not observed: non-scalar custom control | not applicable | not applicable |  |
| Project | PyroCluster Illumination | pyrocluster_illumination | 1 | not applicable | not applicable |  |
| Project | PyroCluster Shadow Casting | pyrocluster_shadow_casting | 1 | not applicable | not applicable |  |

## 4. Standard material
Every channel's on/off default, and each enabled channel's fields: Color, Reflectance
(its default layer), Bump, Alpha, Luminance, Transparency.

| Channel | Label | Our key | Default | Min | Max | Units / visibility / options |
|---|---|---|---|---|---|---|
| Basic Properties / Icon Settings | Color | color | [1.0, 0.9, 0.4] | [0.0, 0.0, 0.0] | [1.0, 1.0, 1.0] |  |
| Basic Properties | Enable Color | enable_color | 1 | not applicable | not applicable |  |
| Basic Properties | Enable Diffusion | enable_diffusion | 0 | not applicable | not applicable |  |
| Basic Properties | Enable Luminance | enable_luminance | 0 | not applicable | not applicable |  |
| Basic Properties | Enable Transparency | enable_transparency | 0 | not applicable | not applicable |  |
| Basic Properties | Enable Reflectance | enable_reflectance | 1 | not applicable | not applicable |  |
| Basic Properties | Enable Environment | enable_environment | 0 | not applicable | not applicable |  |
| Basic Properties | Enable Fog | enable_fog | 0 | not applicable | not applicable |  |
| Basic Properties | Enable Bump | enable_bump | 0 | not applicable | not applicable |  |
| Basic Properties | Enable Normal | enable_normal | 0 | not applicable | not applicable |  |
| Basic Properties | Enable Alpha | enable_alpha | 0 | not applicable | not applicable |  |
| Basic Properties | Enable Glow | enable_glow | 0 | not applicable | not applicable |  |
| Basic Properties | Enable Displacement | enable_displacement | 0 | not applicable | not applicable |  |
| Color | Color | color_2 | [0.8, 0.8, 0.8] | [0.0, 0.0, 0.0] | [1.0, 1.0, 1.0] |  |
| Color | Brightness | brightness | 100.0 | 0.0 | 10000.0 | percent |
| Color | Texture | texture | none | not applicable | not applicable |  |
| Color | Mix Mode | mix_mode | Normal | not applicable | not applicable | options: Normal, Add, Subtract, Multiply |
| Color | Mix Strength | mix_strength | 100.0 | 0.0 | 100.0 | percent |
| Color | Model | model | Lambertian | not applicable | not applicable | options: Lambertian, Oren-Nayar |
| Color | Diffuse Falloff | diffuse_falloff | 0.0 | -1000.0 | 1000.0 | percent |
| Color | Diffuse Level | diffuse_level | 100.0 | 0.0 | 1000.0 | percent |
| Color | Roughness | roughness | 50.0 | 0.0 | 200.0 | percent |
| Diffusion | Diffusion Brightness | diffusion_brightness | 100.0 | 0.0 | 100.0 | percent |
| Diffusion | Diffusion Affect Luminance | diffusion_affect_luminance | 0 | not applicable | not applicable |  |
| Diffusion | Diffusion Affect Specular | diffusion_affect_specular | 1 | not applicable | not applicable |  |
| Diffusion | Diffusion Affect Reflection | diffusion_affect_reflection | 0 | not applicable | not applicable |  |
| Diffusion | Texture | texture_2 | none | not applicable | not applicable |  |
| Diffusion | Mix Mode | mix_mode_2 | Normal | not applicable | not applicable | options: Normal, Add, Subtract, Multiply |
| Diffusion | Mix Strength | mix_strength_2 | 100.0 | 0.0 | 100.0 | percent |
| Luminance | Luminance Color | luminance_color | [1.0, 1.0, 1.0] | [0.0, 0.0, 0.0] | [1.0, 1.0, 1.0] |  |
| Luminance | Luminance Brightness | luminance_brightness | 100.0 | 0.0 | 99999999900.0 | percent |
| Luminance | Texture | texture_3 | none | not applicable | not applicable |  |
| Luminance | Mix Mode | mix_mode_3 | Normal | not applicable | not applicable | options: Normal, Add, Subtract, Multiply |
| Luminance | Mix Strength | mix_strength_3 | 100.0 | 0.0 | 100.0 | percent |
| Transparency | Transparency Color | transparency_color | [1.0, 1.0, 1.0] | [0.0, 0.0, 0.0] | [1.0, 1.0, 1.0] |  |
| Transparency | Transparency Brightness | transparency_brightness | 100.0 | 0.0 | 100.0 | percent |
| Transparency | Transparency Refraction Preset | transparency_refraction_preset | Glass | not applicable | not applicable | options: Custom, Beer, Diamond, Emerald, Ethanol, Glass, Jade, Milk, Oil (Vegetable), Pearl, PET, Plexiglas, Ruby, Sapphire, Water, Water (Ice), Whiskey |
| Transparency | Transparency Refraction | transparency_refraction | 1.517 | 0.25 | 4.0 |  |
| Transparency | Transparency Total Internal Reflection | transparency_total_internal_reflection | 1 | not applicable | not applicable |  |
| Transparency | Exit Reflections | exit_reflections | 1 | not applicable | not applicable |  |
| Transparency | Transparency Fresnel Reflectivity | transparency_fresnel_reflectivity | 100.0 | 0.0 | 100.0 | percent |
| Transparency | Transparency Additive | transparency_additive | 0 | not applicable | not applicable |  |
| Transparency | Texture | texture_4 | none | not applicable | not applicable |  |
| Transparency | Mix Mode | mix_mode_4 | Normal | not applicable | not applicable | options: Normal, Add, Subtract, Multiply |
| Transparency | Mix Strength | mix_strength_4 | 100.0 | 0.0 | 100.0 | percent |
| Transparency | Absorption Color | absorption_color | [1.0, 1.0, 1.0] | [0.0, 0.0, 0.0] | [1.0, 1.0, 1.0] |  |
| Transparency | Absorption Distance | absorption_distance | 100.0 | 0.0 | 999999999.0 | centimeters |
| Transparency | Transparency Blurriness | transparency_blurriness | 0.0 | 0.0 | 100.0 | percent |
| Transparency | Transparency Min Samples | transparency_min_samples | 5 | 1 | 10000 |  |
| Transparency | Transparency Max Samples | transparency_max_samples | 128 | 1 | 10000 |  |
| Transparency | Transparency Accuracy | transparency_accuracy | 50.0 | 0.0 | 100.0 | percent |
| Reflectance | Legacy Reflection Color | legacy_reflection_color | [1.0, 1.0, 1.0] | [0.0, 0.0, 0.0] | [1.0, 1.0, 1.0] | hidden control |
| Reflectance | Legacy Reflection Brightness | legacy_reflection_brightness | 0.0 | 0.0 | 10000.0 | percent; hidden control |
| Reflectance | Legacy Reflection Additive | legacy_reflection_additive | 0 | not applicable | not applicable | hidden control |
| Reflectance | Legacy Reflection Use Bump | legacy_reflection_use_bump | 1 | not applicable | not applicable | hidden control |
| Reflectance | Texture | texture_5 | none | not applicable | not applicable | hidden control |
| Reflectance | Legacy Mix Mode | legacy_mix_mode | Normal | not applicable | not applicable | hidden control; options: Normal, Add, Subtract, Multiply |
| Reflectance | Legacy Mix Strength | legacy_mix_strength | 100.0 | 0.0 | 100.0 | percent; hidden control |
| Reflectance | Legacy Reflection Blurriness | legacy_reflection_blurriness | 0.0 | 0.0 | 100.0 | percent; hidden control |
| Reflectance | Legacy Reflection Min Samples | legacy_reflection_min_samples | 5 | 1 | 10000 | hidden control |
| Reflectance | Legacy Reflection Max Samples | legacy_reflection_max_samples | 128 | 1 | 10000 | hidden control |
| Reflectance | Legacy Reflection Accuracy | legacy_reflection_accuracy | 50.0 | 0.0 | 100.0 | percent; hidden control |
| Reflectance | Layer Setup | layer_setup | not observed: '(32928, 1031184, 5703)' | not observed | not observed |  |
| Reflectance | Global Reflection Brightness | global_reflection_brightness | 100.0 | 0.0 | 1000.0 | percent |
| Reflectance | Global Specular Brightness | global_specular_brightness | 100.0 | 0.0 | 1000.0 | percent |
| Reflectance | Separate Pass | separate_pass | 0 | not applicable | not applicable |  |
| Reflectance | Default Specular.Type | default_specular_type | Specular - Blinn (Legacy) | not applicable | not applicable | options: Beckmann, GGX, Phong, Ward, Anisotropic, Lambertian (Diffuse), Oren-Nayar (Diffuse), Irawan (Woven Cloth), Reflection (Legacy), Specular - Blinn (Legacy), Specular - Phong (Legacy) |
| Reflectance | Default Specular.Attenuation | default_specular_attenuation | Additive | not applicable | not applicable | options: Additive, Metal |
| Reflectance | Default Specular.Width | default_specular_width | 50.0 | 0.0 | 100.0 | percent |
| Reflectance | Default Specular.Width.Texture | default_specular_width_texture | none | not applicable | not applicable |  |
| Reflectance | Default Specular.Falloff | default_specular_falloff | 0.0 | -100.0 | 100.0 | percent |
| Reflectance | Default Specular.Falloff.Texture | default_specular_falloff_texture | none | not applicable | not applicable |  |
| Reflectance | Default Specular.Inner Width | default_specular_inner_width | 0.0 | 0.0 | 100.0 | percent |
| Reflectance | Default Specular.Inner Width.Texture | default_specular_inner_width_texture | none | not applicable | not applicable |  |
| Reflectance | Default Specular.Specular Strength | default_specular_specular_strength | 20.0 | 0.0 | 10000.0 | percent |
| Reflectance | Default Specular.Specular Strength.Texture | default_specular_specular_strength_texture | none | not applicable | not applicable |  |
| Reflectance | Default Specular.Specular Strength.Colored | default_specular_specular_strength_colored | 0 | not applicable | not applicable |  |
| Reflectance | Default Specular.Bump Strength | default_specular_bump_strength | 100.0 | 0.0 | 100.0 | percent |
| Reflectance | Default Specular.Bump Strength.Texture | default_specular_bump_strength_texture | none | not applicable | not applicable |  |
| Reflectance | Default Specular.Bump Strength.Mode | default_specular_bump_strength_mode | Default | not applicable | not applicable | options: Default, Custom Bump Map, Custom Normal Map |
| Reflectance | Default Specular.Separate Pass | default_specular_separate_pass | 0 | not applicable | not applicable |  |
| Reflectance / Layer Color | Default Specular.Layer Color.Color | default_specular_layer_color_color | [1.0, 1.0, 1.0] | [0.0, 0.0, 0.0] | [1.0, 1.0, 1.0] |  |
| Reflectance / Layer Color | Default Specular.Layer Color.Brightness | default_specular_layer_color_brightness | 100.0 | 0.0 | 10000.0 | percent |
| Reflectance / Layer Color | Default Specular.Layer Color.Texture | default_specular_layer_color_texture | none | not applicable | not applicable |  |
| Reflectance / Layer Color | Default Specular.Layer Color.Mix Mode | default_specular_layer_color_mix_mode | Normal | not applicable | not applicable | options: Normal, Add, Subtract, Multiply |
| Reflectance / Layer Color | Default Specular.Layer Color.Mix Strength | default_specular_layer_color_mix_strength | 100.0 | 0.0 | 100.0 | percent |
| Reflectance / Layer Mask | Default Specular.Layer Mask.Amount | default_specular_layer_mask_amount | 100.0 | 0.0 | 100.0 | percent |
| Reflectance / Layer Mask | Default Specular.Layer Mask.Color | default_specular_layer_mask_color | [1.0, 1.0, 1.0] | [0.0, 0.0, 0.0] | [1.0, 1.0, 1.0] |  |
| Reflectance / Layer Mask | Default Specular.Layer Mask.Texture | default_specular_layer_mask_texture | none | not applicable | not applicable |  |
| Reflectance / Layer Mask | Default Specular.Layer Mask.Mix Mode | default_specular_layer_mask_mix_mode | Normal | not applicable | not applicable | options: Normal, Add, Subtract, Multiply |
| Reflectance / Layer Mask | Default Specular.Layer Mask.Mix Strength | default_specular_layer_mask_mix_strength | 100.0 | 0.0 | 100.0 | percent |
| Environment | Environment Color | environment_color | [1.0, 1.0, 1.0] | [0.0, 0.0, 0.0] | [1.0, 1.0, 1.0] |  |
| Environment | Environment Brightness | environment_brightness | 100.0 | 0.0 | 10000.0 | percent |
| Environment | Texture | texture_6 | none | not applicable | not applicable |  |
| Environment | Mix Mode | mix_mode_5 | Multiply | not applicable | not applicable | options: Normal, Add, Subtract, Multiply |
| Environment | Mix Strength | mix_strength_5 | 100.0 | 0.0 | 100.0 | percent |
| Environment | Environment Tiles X | environment_tiles_x | 1.0 | 0.0 | 999999999.0 |  |
| Environment | Environment Tiles Y | environment_tiles_y | 1.0 | 0.0 | 999999999.0 |  |
| Environment | Environment Exclusive | environment_exclusive | 1 | not applicable | not applicable |  |
| Fog | Fog Color | fog_color | [1.0, 1.0, 1.0] | [0.0, 0.0, 0.0] | [1.0, 1.0, 1.0] |  |
| Fog | Fog Brightness | fog_brightness | 100.0 | 0.0 | 100.0 | percent |
| Fog | Fog Distance | fog_distance | 1000.0 | 0.0 | 999999999.0 | centimeters |
| Bump | Bump Strength | bump_strength | 20.0 | -1000000.0 | 1000000.0 | percent |
| Bump | Bump MIP Falloff | bump_mip_falloff | 1 | not applicable | not applicable |  |
| Bump | Texture | texture_7 | none | not applicable | not applicable |  |
| Normal | Strength | strength | 100.0 | 0.0 | 100000.0 | percent |
| Normal | Method | method | Tangent | not applicable | not applicable | options: Tangent, Object, World |
| Normal | Flip X (Red) | flip_x_red | 0 | not applicable | not applicable |  |
| Normal | Flip Y (Green) | flip_y_green | 0 | not applicable | not applicable |  |
| Normal | Flip Z (Blue) | flip_z_blue | 0 | not applicable | not applicable |  |
| Normal | Swap Y & Z (Y up) | swap_y_z_y_up | 0 | not applicable | not applicable |  |
| Normal | Texture | texture_8 | none | not applicable | not applicable |  |
| Alpha | Alpha Color | alpha_color | [0.0, 0.0, 0.0] | [0.0, 0.0, 0.0] | [1.0, 1.0, 1.0] |  |
| Alpha | Alpha Delta | alpha_delta | [0.5, 0.5, 0.5] | [0.0, 0.0, 0.0] | [1.0, 1.0, 1.0] |  |
| Alpha | Alpha Invert | alpha_invert | 0 | not applicable | not applicable |  |
| Alpha | Alpha Soft | alpha_soft | 1 | not applicable | not applicable |  |
| Alpha | Alpha Image Alpha | alpha_image_alpha | 1 | not applicable | not applicable |  |
| Alpha | Alpha Pre-multiplied | alpha_pre_multiplied | 0 | not applicable | not applicable |  |
| Alpha | Texture | texture_9 | none | not applicable | not applicable |  |
| Specular | Legacy Specular Color | legacy_specular_color | [1.0, 1.0, 1.0] | [0.0, 0.0, 0.0] | [1.0, 1.0, 1.0] | hidden control |
| Specular | Legacy Specular Brightness | legacy_specular_brightness | 100.0 | 0.0 | 10000.0 | percent; hidden control |
| Specular | Texture | texture_10 | none | not applicable | not applicable | hidden control |
| Specular | Legacy Mix Mode | legacy_mix_mode_2 | Normal | not applicable | not applicable | hidden control; options: Normal, Add, Subtract, Multiply |
| Specular | Legacy Mix Strength | legacy_mix_strength_2 | 100.0 | 0.0 | 100.0 | percent; hidden control |
| Specular | Legacy Specular Mode | legacy_specular_mode | Plastic | not applicable | not applicable | hidden control; options: Plastic, Metal, Colored |
| Specular | Legacy Specular Width | legacy_specular_width | 50.0 | 0.0 | 100.0 | percent; hidden control |
| Specular | Legacy Specular Height | legacy_specular_height | 20.0 | 0.0 | 1000.0 | percent; hidden control |
| Specular | Legacy Specular Falloff | legacy_specular_falloff | 0.0 | -100.0 | 100.0 | percent; hidden control |
| Specular | Legacy Specular Inner Width | legacy_specular_inner_width | 0.0 | 0.0 | 100.0 | percent; hidden control |
| Glow | Glow Color | glow_color | [1.0, 1.0, 1.0] | [0.0, 0.0, 0.0] | [1.0, 1.0, 1.0] |  |
| Glow | Glow Brightness | glow_brightness | 100.0 | 0.0 | 100.0 | percent |
| Glow | Glow Inner Strength | glow_inner_strength | 100.0 | 0.0 | 10000.0 | percent |
| Glow | Glow Outer Strength | glow_outer_strength | 500.0 | 0.0 | 10000.0 | percent |
| Glow | Glow Radius | glow_radius | 10.0 | 0.0 | 999999999.0 | centimeters |
| Glow | Glow Random | glow_random | 0.0 | 0.0 | 100.0 | percent |
| Glow | Glow Frequency | glow_frequency | 1.0 | 0.0 | 999999999.0 |  |
| Glow | Glow Use Material Color | glow_use_material_color | 1 | not applicable | not applicable |  |
| Displacement | Displacement Strength | displacement_strength | 100.0 | -100.0 | 100.0 | percent |
| Displacement | Displacement Height | displacement_height | 10.0 | -999999999.0 | 999999999.0 | centimeters |
| Displacement | Type | type | Intensity (Centered) | not applicable | not applicable | options: Intensity, Intensity (Centered), Red / Green, RGB (XYZ Tangent), RGB (XYZ Object), RGB (XYZ World) |
| Displacement | Texture | texture_11 | none | not applicable | not applicable |  |
| Displacement / Sub Polygon Displacement | Sub Polygon Displacement | sub_polygon_displacement | 0 | not applicable | not applicable |  |
| Displacement / Sub Polygon Displacement | Subdivision Level | subdivision_level | 4 | 1 | 12 |  |
| Displacement / Sub Polygon Displacement | Round Geometry | round_geometry | 0 | not applicable | not applicable |  |
| Displacement / Sub Polygon Displacement | Round Contour | round_contour | 1 | not applicable | not applicable |  |
| Displacement / Sub Polygon Displacement | Map Rounded Geometry | map_rounded_geometry | 1 | not applicable | not applicable |  |
| Displacement / Sub Polygon Displacement | Map Resulting Geometry | map_resulting_geometry | 0 | not applicable | not applicable |  |
| Displacement / Sub Polygon Displacement | Keep Original Edges | keep_original_edges | 1 | not applicable | not applicable |  |
| Displacement / Sub Polygon Displacement | Best Distribution | best_distribution | 1 | not applicable | not applicable |  |
| Illumination | Polygon Light | polygon_light | 0 | not applicable | not applicable |  |
| Illumination | Portal | portal | 0 | not applicable | not applicable |  |
| Illumination | GI Generate | gi_generate | 1 | not applicable | not applicable |  |
| Illumination | GI Generate Strength | gi_generate_strength | 100.0 | 0.0 | 10000.0 | percent |
| Illumination | GI Generate Saturation | gi_generate_saturation | 100.0 | 0.0 | 1000.0 | percent |
| Illumination | GI Receive | gi_receive | 1 | not applicable | not applicable |  |
| Illumination | GI Receive Strength | gi_receive_strength | 100.0 | 0.0 | 10000.0 | percent |
| Illumination | GI Receive Saturation | gi_receive_saturation | 100.0 | 0.0 | 1000.0 | percent |
| Illumination | Caustics Generate | caustics_generate | 1 | not applicable | not applicable |  |
| Illumination | Caustics Generate Strength | caustics_generate_strength | 100.0 | 0.0 | 10000.0 | percent |
| Illumination | Caustics Receive | caustics_receive | 1 | not applicable | not applicable |  |
| Illumination | Caustics Receive Strength | caustics_receive_strength | 100.0 | 0.0 | 10000.0 | percent |
| Illumination | Caustics Sample Radius | caustics_sample_radius | 10.0 | 0.0 | 999999999.0 | centimeters |
| Illumination | Caustics Samples | caustics_samples | 100.0 | 1.0 | 10000.0 |  |
| Viewport | Animate Preview | animate_preview | 0 | not applicable | not applicable |  |
| Viewport | Texture Preview Size | texture_preview_size | Default | not applicable | not applicable | options: Default, No Scaling, 64x64 (16 KB), 128x128 (64 KB), 256x256 (256 KB), 512x512 (1 MB), 1024x1024 (4 MB), 2048x2048 (16 MB), 4096x4096 (64 MB), 8192x8192 (256 MB), 16384x16384 (1024 MB) |
| Viewport | Viewport Display | viewport_display | Combined | not applicable | not applicable | options: Combined, Color, Diffusion, Luminance, Transparency, Reflectance, Environment, Bump, Alpha, Displacement, Normal |
| Viewport | Display Color | display_color | 1 | not applicable | not applicable |  |
| Viewport | Display Diffusion | display_diffusion | 1 | not applicable | not applicable |  |
| Viewport | Display Luminance | display_luminance | 1 | not applicable | not applicable |  |
| Viewport | Display Transparency | display_transparency | 1 | not applicable | not applicable |  |
| Viewport | Display Environment | display_environment | 1 | not applicable | not applicable |  |
| Viewport | Display Bump | display_bump | 1 | not applicable | not applicable |  |
| Viewport | Display Normal | display_normal | 1 | not applicable | not applicable |  |
| Viewport | Display Alpha | display_alpha | 1 | not applicable | not applicable |  |
| Viewport | Display Reflectance | display_reflectance | 1 | not applicable | not applicable |  |
| Viewport | Display Displacement | display_displacement | 1 | not applicable | not applicable |  |
| Viewport / Reflectance Preview | Environment Override | environment_override | none | not applicable | not applicable |  |
| Viewport / Reflectance Preview | Rotation | rotation | [0.0, 0.0, 0.0] | [-57295779455.786545, -57295779455.786545, -57295779455.786545] | [57295779455.786545, 57295779455.786545, 57295779455.786545] | degrees |
| Viewport / Viewport Tessellation | Mode | mode | None | not applicable | not applicable | options: None, Uniform, Projective |
| Viewport / Viewport Tessellation | Uniform Level | uniform_level | 8.0 | 1.0 | 999999999.0 | hidden control |
| Viewport / Viewport Tessellation | Projective Level | projective_level | 8.0 | 1.0 | 999999999.0 | hidden control |

Reflectance layer count: 1; layers: [{"name": "Default Specular", "fields_note": "Layer fields are included in description table where exposed."}].

## 5. Render settings of a new document
Renderer, output size, frame rate, anti-aliasing, the Options tab, and the Save tab
format.

| Tab / Group | Label | Our key | Default | Min | Max | Units / visibility / options |
|---|---|---|---|---|---|---|
| Basic Properties / Icon Settings | Icon File / ID | icon_file_id |  | not applicable | not applicable |  |
| Basic Properties / Icon Settings | Icon Color | icon_color | 0 | not applicable | not applicable |  |
| Basic Properties / Icon Settings | Color | color | [1.0, 0.9, 0.4] | [0.0, 0.0, 0.0] | [1.0, 1.0, 1.0] |  |
| Basic Properties | Name | name | My Render Setting | not applicable | not applicable |  |
| Basic Properties | Layer | layer | none | not applicable | not applicable | hidden control |
| Basic Properties | Save | save | 1 | not applicable | not applicable |  |
| Basic Properties | Enable Multi-Pass Rendering | enable_multi_pass_rendering | 0 | not applicable | not applicable |  |
| Basic Properties | Enable Stereoscopic Rendering | enable_stereoscopic_rendering | 0 | not applicable | not applicable |  |
| Basic Properties | Material Override | material_override | 0 | not applicable | not applicable |  |
| Basic Properties | Renderer | renderer | Standard | not applicable | not applicable | options: Standard, Physical, Viewport Renderer |
| Output | Width | width | 1280.0 | 0.0 | 128000.0 | hidden control |
| Output | Width | width_2 | 1280.0 | 1280.0 | 1280.0 |  |
| Output | Height | height | 720.0 | 0.0 | 128000.0 | hidden control |
| Output | Height | height_2 | 720.0 | 720.0 | 720.0 |  |
| Output | Lock Ratio | lock_ratio | 0 | not applicable | not applicable |  |
| Output | Adapt Data Rate | adapt_data_rate | 1 | not applicable | not applicable |  |
| Output | Resolution | resolution | 72.0 | 1.0 | 100000.0 | hidden control |
| Output | Resolution | resolution_2 | 72.0 | 72.0 | 72.0 |  |
| Output | Render Region | render_region | 0 | not applicable | not applicable |  |
| Output | Left Border | left_border | 0 | 0 | 128000 |  |
| Output | Top Border | top_border | 0 | 0 | 128000 |  |
| Output | Right Border | right_border | 0 | 0 | 128000 |  |
| Output | Bottom Border | bottom_border | 0 | 0 | 128000 |  |
| Output | Film Aspect | film_aspect | 1.7777777777777777 | 0.001 | 1000.0 |  |
| Output | Pixel Aspect | pixel_aspect | 1.0 | 0.001 | 1000.0 |  |
| Output | Frame Rate | frame_rate | 30.0 | 1.0 | 2500.0 |  |
| Output | Frame Range | frame_range | Current Frame | not applicable | not applicable | options: Manual, Current Frame, All Frames, Preview Range |
| Output | From | from | {"frames": 0, "seconds": 0.0} | {"frames": 0, "seconds": 0.0} | {"frames": 3240000, "seconds": 108000.0} | frames and seconds |
| Output | To | to | {"frames": 0, "seconds": 0.0} | {"frames": 0, "seconds": 0.0} | {"frames": 3240000, "seconds": 108000.0} | frames and seconds |
| Output | Frame Step | frame_step | 1 | 1 | 1000000 |  |
| Output | Fields | fields | None | not applicable | not applicable | options: None, Even First, Odd First |
| Output | Annotations | annotations |  | not applicable | not applicable |  |
| Save / Regular Image | Save | save_2 | 1 | not applicable | not applicable |  |
| Save / Regular Image | File... | file |  | not applicable | not applicable |  |
| Save / Regular Image | Format | format | TIF | not applicable | not applicable | options: BMP, BodyPaint 3D (B3D), DDS, DPX, HDR, IFF, JPG, OpenEXR, PICT, PNG, PSB, PSD, RLA, RPF, TGA, TIF, MOV, MP4 |
| Save / Regular Image | Options... | options |  | not applicable | not applicable |  |
| Save / Regular Image | Depth | depth | 8 Bit/Channel | not applicable | not applicable | options: 8 Bit/Channel, 16 Bit/Channel, 32 Bit/Channel |
| Save / Regular Image | Name | name_2 | Name0000.TIF | not applicable | not applicable | options: Name0000.TIF, Name0000, Name.0000, Name000.TIF, Name000, Name.000, Name.0000.TIF |
| Save / Regular Image | Image Color Profile | image_color_profile | not observed: non-scalar custom control | not applicable | not applicable |  |
| Save / Regular Image | Alpha Channel | alpha_channel | 0 | not applicable | not applicable |  |
| Save / Regular Image | Straight Alpha | straight_alpha | 0 | not applicable | not applicable |  |
| Save / Regular Image | Separate Alpha | separate_alpha | 0 | not applicable | not applicable |  |
| Save / Regular Image | 8 Bit Dithering | 8_bit_dithering | 1 | not applicable | not applicable |  |
| Save / Regular Image | Include Sound | include_sound | 0 | not applicable | not applicable |  |
| Save / QuickTime VR Options | Generate File | generate_file | VR | not applicable | not applicable | options: VR, Intermediate, Both |
| Save / QuickTime VR Options | Horizontal Steps | horizontal_steps | 36 | 1 | 1000 |  |
| Save / QuickTime VR Options | Start Angle | start_angle | 0.0 | -57295779455.786545 | 57295779455.786545 | degrees |
| Save / QuickTime VR Options | End Angle | end_angle | 360.0 | -57295779455.786545 | 57295779455.786545 | degrees |
| Save / QuickTime VR Options | Vertical Steps | vertical_steps | 19 | 1 | 1000 |  |
| Save / QuickTime VR Options | Start Angle | start_angle_2 | 90.0 | -90.0 | 90.0 | degrees |
| Save / QuickTime VR Options | End Angle | end_angle_2 | -90.0 | -90.0 | 90.0 | degrees |
| Save / QuickTime VR Options | Default X Resolution | default_x_resolution | 320 | 2 | 4000 |  |
| Save / QuickTime VR Options | Default Y Resolution | default_y_resolution | 240 | 2 | 4000 |  |
| Save / Multi-Pass Image | Save | save_3 | 1 | not applicable | not applicable |  |
| Save / Multi-Pass Image | File | file_2 |  | not applicable | not applicable |  |
| Save / Multi-Pass Image | Format | format_2 | PSD | not applicable | not applicable | options: BMP, BodyPaint 3D (B3D), DDS, DPX, HDR, IFF, JPG, OpenEXR, PICT, PNG, PSB, PSD, RLA, RPF, TGA, TIF, MOV, MP4 |
| Save / Multi-Pass Image | Options... | options_2 |  | not applicable | not applicable |  |
| Save / Multi-Pass Image | Depth | depth_2 | 16 Bit/Channel | not applicable | not applicable | options: 8 Bit/Channel, 16 Bit/Channel, 32 Bit/Channel |
| Save / Multi-Pass Image | Multi-Layer File | multi_layer_file | 1 | not applicable | not applicable |  |
| Save / Multi-Pass Image | Layer Name as Suffix | layer_name_as_suffix | 1 | not applicable | not applicable |  |
| Save / Multi-Pass Image | User Defined Layer Name | user_defined_layer_name | 0 | not applicable | not applicable |  |
| Save / Multi-Pass Image | Straight Alpha | straight_alpha_2 | 0 | not applicable | not applicable |  |
| Save / Compositing Project File | Save | save_4 | 0 | not applicable | not applicable |  |
| Save / Compositing Project File | Target Application | target_application | After Effects | not applicable | not applicable | options: After Effects, Nuke, Final Cut 7, Motion |
| Save / Compositing Project File | Relative | relative | 0 | not applicable | not applicable |  |
| Save / Compositing Project File | Include Timeline Marker | include_timeline_marker | 0 | not applicable | not applicable |  |
| Save / Compositing Project File | Include 3D Data | include_3d_data | 0 | not applicable | not applicable |  |
| Save / Compositing Project File | Save FBX File | save_fbx_file | 0 | not applicable | not applicable |  |
| Save / Compositing Project File | Save Alembic File | save_alembic_file | 0 | not applicable | not applicable |  |
| Multi-Pass | Separate Lights | separate_lights | None | not applicable | not applicable | options: None, All, Selected |
| Multi-Pass | Mode | mode | 3 Channels: Diffuse,Specular,Shadow | not applicable | not applicable | options: 1 Channel: Diffuse+Specular+Shadow, 2 Channels: Diffuse+Specular,Shadow, 3 Channels: Diffuse,Specular,Shadow |
| Multi-Pass | Shadow Correction | shadow_correction | 0 | not applicable | not applicable |  |
| Anti-Aliasing | Anti-Aliasing | anti_aliasing | Geometry | not applicable | not applicable | options: None, Geometry, Best |
| Anti-Aliasing | Min Level | min_level | 1x1 | not applicable | not applicable | options: 1x1, 2x2, 4x4, 8x8, 16x16 |
| Anti-Aliasing | Max Level | max_level | 4x4 | not applicable | not applicable | options: 1x1, 2x2, 4x4, 8x8, 16x16 |
| Anti-Aliasing | Threshold | threshold | 10.0 | 0.0 | 100.0 | percent |
| Anti-Aliasing | Use Object Properties | use_object_properties | 1 | not applicable | not applicable |  |
| Anti-Aliasing | Consider Multi-Passes | consider_multi_passes | 0 | not applicable | not applicable |  |
| Anti-Aliasing | Filter | filter | Cubic (Still Image) | not applicable | not applicable | options: Cubic (Still Image), Gauss (Animation), Mitchell, Sinc, Box, Triangle, Catmull, PAL/NTSC |
| Anti-Aliasing | Custom Size | custom_size | 0 | not applicable | not applicable |  |
| Anti-Aliasing | Filter Width | filter_width | 2.0 | 2.0 | 2.0 |  |
| Anti-Aliasing | Filter Height | filter_height | 2.0 | 2.0 | 2.0 |  |
| Anti-Aliasing | Clip Negative Component | clip_negative_component | 0 | not applicable | not applicable |  |
| Anti-Aliasing | MIP Scale | mip_scale | 50.0 | 0.0 | 1000.0 | percent |
| Anti-Aliasing | Small Fragments | small_fragments | Hybrid | not applicable | not applicable | options: Hybrid, Scanline only, Raytrace only |
| Material Override | Custom Material | custom_material | none | not applicable | not applicable |  |
| Material Override | Mode | mode_2 | Exclude | not applicable | not applicable | options: Include, Exclude |
| Material Override | Materials | materials | not observed: non-scalar custom control | not applicable | not applicable |  |
| Material Override / Preserve | Diffuse Color | diffuse_color | 0 | not applicable | not applicable |  |
| Material Override / Preserve | Luminance | luminance | 0 | not applicable | not applicable |  |
| Material Override / Preserve | Transparency | transparency | 1 | not applicable | not applicable |  |
| Material Override / Preserve | Reflectance | reflectance | 0 | not applicable | not applicable |  |
| Material Override / Preserve | Bump | bump | 1 | not applicable | not applicable |  |
| Material Override / Preserve | Normal | normal | 1 | not applicable | not applicable |  |
| Material Override / Preserve | Alpha | alpha | 1 | not applicable | not applicable |  |
| Material Override / Preserve | Displacement | displacement | 1 | not applicable | not applicable |  |
| Options | Transparency | transparency_2 | 1 | not applicable | not applicable |  |
| Options | Ray Threshold | ray_threshold | 0.1 | 0.0 | 100.0 | percent |
| Options | Refraction | refraction | 1 | not applicable | not applicable |  |
| Options | Ray Depth | ray_depth | 15 | 0 | 500 |  |
| Options | Reflection | reflection | 1 | not applicable | not applicable |  |
| Options | Reflection Depth | reflection_depth | 5 | 1 | 200 |  |
| Options | Shadow | shadow | 1 | not applicable | not applicable |  |
| Options | Shadow Depth | shadow_depth | 15 | 0 | 500 |  |
| Options | Limit Reflections to Floor/Sky | limit_reflections_to_floor_sky | 0 | not applicable | not applicable |  |
| Options | Level of Detail | level_of_detail | 100.0 | 0.0 | 1000.0 | percent |
| Options | Blurriness | blurriness | 1 | not applicable | not applicable |  |
| Options | Global Brightness | global_brightness | 100.0 | 0.0 | 100000.0 | percent |
| Options | Limit Shadows to Soft | limit_shadows_to_soft | 0 | not applicable | not applicable |  |
| Options | Motion Scale | motion_scale | 128.0 | 1.0 | 65536.0 |  |
| Options | Cache Shadow Maps | cache_shadow_maps | 0 | not applicable | not applicable |  |
| Options | Active Object Only | active_object_only | 0 | not applicable | not applicable |  |
| Options | Default Light | default_light | 1 | not applicable | not applicable |  |
| Options | Textures | textures | 1 | not applicable | not applicable |  |
| Options | Show Texture Errors | show_texture_errors | 1 | not applicable | not applicable |  |
| Options | Volumetric Lighting | volumetric_lighting | 1 | not applicable | not applicable |  |
| Options | Use Display Tag LOD | use_display_tag_lod | 0 | not applicable | not applicable |  |
| Options | Render HUD | render_hud | 0 | not applicable | not applicable |  |
| Options | Render Doodle | render_doodle | 1 | not applicable | not applicable |  |
| Options | Sub Polygon Displacement | sub_polygon_displacement | 1 | not applicable | not applicable |  |
| Options | Post Effects | post_effects | 1 | not applicable | not applicable |  |
| Options | Identical Noise Distribution | identical_noise_distribution | 0 | not applicable | not applicable |  |
| Options | Subsurface Scattering | subsurface_scattering | 1 | not applicable | not applicable |  |
| Options | Bucket Sequence | bucket_sequence | Centered | not applicable | not applicable | options: Centered, Left to Right, Right to Left, Top to Bottom, Bottom to Top |
| Options | Automatic Size | automatic_size | 1 | not applicable | not applicable |  |
| Options | Bucket Width | bucket_width | 64 | 8 | 256 |  |
| Options | Bucket Height | bucket_height | 64 | 8 | 256 |  |
| Stereoscopic | Calculate Stereoscopic Images | calculate_stereoscopic_images | Merged Stereoscopic Image | not applicable | not applicable | options: Individual Channels, Merged Stereoscopic Image, Individual Channels and Merged Image, Single Channel |
| Stereoscopic | Single Channel | single_channel | 1 | 1 | 999999999 |  |
| Stereoscopic | Non-Stereoscopic Image | non_stereoscopic_image | 0 | not applicable | not applicable |  |
| Stereoscopic | Use Folder for Saving | use_folder_for_saving | 0 | not applicable | not applicable |  |
| Stereoscopic | Channels | channels | 2 | 1 | 999999999 |  |
| Stereoscopic | Mode | mode_3 | Anaglyph | not applicable | not applicable | options: Anaglyph, Side-by-Side, Interlaced |
| Stereoscopic | Additional Parallax (pixel) | additional_parallax_pixel | 0 | 0 | 999999999 |  |
| Stereoscopic | Swap Left/Right | swap_left_right | 0 | not applicable | not applicable |  |
| Stereoscopic / Anaglyph | System | system | Red - Cyan | not applicable | not applicable | hidden control; options: Red - Blue, Red - Green, Red - Cyan, Yellow - Blue, Green - Magenta, Custom |
| Stereoscopic / Anaglyph | System | system_2 | Red | not applicable | not applicable | options: Red, Green, Blue, Yellow, Magenta, Custom |
| Stereoscopic / Anaglyph | Left | left | [1.0, 0.0, 0.0] | [0.0, 0.0, 0.0] | [1.0, 1.0, 1.0] |  |
| Stereoscopic / Anaglyph | Right | right | [0.0, 1.0, 0.0] | [0.0, 0.0, 0.0] | [1.0, 1.0, 1.0] |  |
| Stereoscopic / Anaglyph | Method | method | Optimized | not applicable | not applicable | options: Full, Gray, Half Color, Optimized, Color |
| Stereoscopic / Side-by-Side | Alignment | alignment | Horizontal | not applicable | not applicable | options: Horizontal, Vertical |
| Stereoscopic / Side-by-Side | Left Mirror X | left_mirror_x | 0 | not applicable | not applicable |  |
| Stereoscopic / Side-by-Side | Left Mirror Y | left_mirror_y | 0 | not applicable | not applicable |  |
| Stereoscopic / Side-by-Side | Right Mirror X | right_mirror_x | 0 | not applicable | not applicable |  |
| Stereoscopic / Side-by-Side | Right Mirror Y | right_mirror_y | 0 | not applicable | not applicable |  |
| Stereoscopic / Interlaced | Type | type | Horizontal | not applicable | not applicable | options: Horizontal, Vertical, Checkerboard |
| Team Render / Single Image | Distribute Subsurface Caches | distribute_subsurface_caches | 1 | not applicable | not applicable |  |
| Team Render / Single Image | Distribute Ambient Occlusion Caches | distribute_ambient_occlusion_caches | 1 | not applicable | not applicable |  |
| Team Render / Single Image | Distribute Irradiance Caches | distribute_irradiance_caches | 1 | not applicable | not applicable |  |
| Team Render / Single Image | Distribute Light Mapping Caches | distribute_light_mapping_caches | 1 | not applicable | not applicable |  |
| Team Render / Single Image | Distribute Radiosity Map Caches | distribute_radiosity_map_caches | 0 | not applicable | not applicable |  |


## 6. Default scene behaviour
With no lights in the scene, how is a render lit? A default light, and where? Read it
from the documentation the app shows, or say it can't be read.

The new render settings expose **Default Light = on**. Default-light position/direction and the app-shown explanatory documentation: **not observed**; the permitted surface did not yield that documentation. No light position is inferred from a render. The separate unit-display preference is likewise not observed because preferences are excluded.


## Notes

- Form-only observation: no golden folders were created for O29. Each camera, each light Type and the standard material came from its own menu command in a fresh owned document; document and render settings were read before changing any new-document values.
- Default columns are captured together before range probes. Numeric Min / Max columns are **observed public assignment readbacks at −999999999 and +999999999**, not inferred mathematical limits or a guarantee of GUI-editable limits. Values are converted to the displayed unit (degrees, percent, centimeters; times show frames and seconds). Colors/vectors assign each component. Boolean and dropdown ranges are not applicable; dropdown labels are listed. Custom controls that could not be faithfully decoded remain explicitly not observed.
- Each endpoint trial restores the complete original public parameter container before and after the write so coupled time or lens fields cannot contaminate later defaults. An initial exploratory document-time sweep restored only the changed property and moved linked preview/range fields; that sweep was rejected and superseded by fresh-document, pre-captured defaults above. Its intermediate log is retained only in observer scratch. No rejected defaults are presented as observations.
- Hidden controls are marked; some disabled-channel or mode-dependent fields are exposed in the public description even when their parent tab is hidden. Their stored values are not a claim that the control is currently enabled in the GUI. The default Reflectance layer is recorded by its public label and fields.
- Scripts, raw parameter identifiers and scene data remain in observer scratch only. Permission for scripting is owner-confirmed in state; this is not an independent licence review.
- Required full golden check is blocked by the unrelated existing restrict-bend duplicate parameter table. Collection is distinct from publication: no commit or push; Queue remains partial.

## Batch-03 checkpoint (2026-11-18)

Hermes; public scripted bridge, version 25010. New mesh cases: 0; form-only observed rows: 13; not observed rows: 1. Full golden check blocked by unrelated restrict-bend duplicate params table; no commit or push. Queue remains partial, collection and publication tracked separately. Original observations and fixtures preserved.
