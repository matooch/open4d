# Plugins

A plugin is a folder holding a manifest (`plugin.toml`) and one Rhai script:

```
my-plugin/
  plugin.toml
  main.rhai
```

The manifest names the plugin (its namespace), its version, the entry script, the
permissions it needs, and the node types and commands it adds. Each node type declares its
parameters as data (id, kind, default, range, unit), so the new object gets an Attribute
manager, undo, animation and file support with no extra code.

A node type's build function receives its parameters and returns points and polygons:

```rhai
fn build(params) {
    let s = params.size;
    #{ points: [[0,0,0],[s,0,0],[s,s,0],[0,s,0]], polys: [[0,1,2,3]] }
}
```

Rules worth knowing:

- every node type and command id must start with `<plugin name>.`;
- only function definitions of the entry script are used, and it cannot import modules;
- a plugin that fails to load is reported but does not stop the others;
- results are validated and capped at one million points and one million polygons.

Load a plugin folder with `--plugins <dir>` on the command line, or use `--no-plugins`
(or `FORGE_NO_PLUGINS=1`) to load none.
