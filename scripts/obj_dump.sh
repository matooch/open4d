#!/bin/sh
# Print an OBJ's vertices and faces with 0-based indices (the harness's numbering).
# Usage: scripts/obj_dump.sh tests/golden/<feature>/<case>/mesh.obj [max_lines]
# Faces keep only the vertex index of each corner ("f 1/2/3" -> 0).
awk '/^v /{printf "v%d: %.4f %.4f %.4f\n", n++, $2, $3, $4}
     /^f /{s="f" f++ ":"; for(i=2;i<=NF;i++){split($i,a,"/"); s=s " " a[1]-1} print s}' "$1" |
    head -n "${2:-100000}"
