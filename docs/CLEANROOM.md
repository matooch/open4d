# Clean-room policy
Allowed: public manuals/tutorials, public SDK documentation (read-only concepts), user-run black-box observation, published math/graphics literature.
Forbidden: decompilation, disassembly, memory inspection, reading app resource folders (res/, icons, presets, libs, shaders), copying UI layouts/icons/strings, using Maxon marks.
Process: (1) Observer session records behavior into docs/parity/<feature>.md (parameters seen, numeric outputs, exported meshes, reference renders). (2) Implementer session reads only those notes + public docs. (3) Parity harness compares.
Check the reference app's EULA yourself before running automated observation; stop if it prohibits it.
