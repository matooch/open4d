# Redshift integration (planned, Phase 12)
Fact: Redshift is proprietary and has no public SDK for third-party host apps; the official C4D integration uses Maxon's C++ plugin API. A Rust app cannot load it as a normal plugin.
Spike options (evaluate in order, each behind the `RenderEngine` trait):
A. Standalone path: export scene to Redshift's own standalone/proxy formats and launch the licensed Redshift standalone renderer on Linux, stream results back (IPR via file/tile watching). Needs a license check and its terms.
B. Ask Maxon/Redshift for a host-integration SDK or C API; wrap via FFI in `render-bridge`.
C. Reimplement a Redshift-like biased GPU renderer ourselves (render-gpu): GI (brute force+irradiance cache), RS-style material/shader nodes, light types, AOVs, proxies, volumes.
Plan now: keep material graph and light/camera/AOV data models rich enough to translate to Redshift node names; write a translator trait `SceneExporter`. No Redshift code or binaries enter the repo.
