#!/bin/bash
# Unattended observer loop (docs/agents/PIPELINE.md §1, docs/parity/QUEUE.md).
# Copy to ~/forge-observer/ next to run-queue.md and .mcp.json, then run:
#   caffeinate -i ~/forge-observer/run-overnight.sh
# One fresh session per queue job. Ends when the queue is empty, a STOP file appears,
# or a pull fails. Commits and pushes go to the `observations` branch only.
cd ~/forge-observer || exit 1
REPO=${FORGE_REPO:-~/Github/open4d}
rm -f STOP
ALLOW="Read,Write,Edit,Glob,Grep,Bash(git *),Bash(cargo run *),Bash(mkdir *),Bash(ls *),Bash(touch *),mcp__refapp__ping,mcp__refapp__doctor,mcp__refapp__get_capabilities,mcp__refapp__execute_python_script,mcp__refapp__add_primitive,mcp__refapp__get_parameter,mcp__refapp__set_parameter,mcp__refapp__modify_object,mcp__refapp__list_objects,mcp__refapp__get_object_info,mcp__refapp__dump_object_tree,mcp__refapp__find_objects,mcp__refapp__scene_snapshot,mcp__refapp__scene_diff,mcp__refapp__set_keyframe,mcp__refapp__run_modeling_command,mcp__refapp__apply_deformer,mcp__refapp__add_field_to_scene,mcp__refapp__add_effector,mcp__refapp__create_via_command,mcp__refapp__enumerate_descids,mcp__refapp__find_command_by_name,mcp__refapp__undo,mcp__refapp__redo,mcp__refapp__begin_undo_group,mcp__refapp__end_undo_group,mcp__refapp__get_console_log,mcp__refapp__clear_console_log,mcp__refapp__render_frame,mcp__refapp__image_inspect"
# Bridge tools whose names the cleanroom gate forbids in the repo (the cloner tool) go,
# comma-separated, in ~/forge-observer/allow-extra.txt: copy them from your current script.
EXTRA=$(tr -d '\n ' < allow-extra.txt 2>/dev/null); [ -n "$EXTRA" ] && ALLOW="$ALLOW,$EXTRA"
# Tools that must never run (knowledge base, plugin installs, scene save/load) are listed,
# comma-separated, in ~/forge-observer/deny-tools.txt (kept out of the repo: it names the
# reference app, which the cleanroom gate forbids). Tools not in ALLOW are refused anyway.
DENY=$(tr -d '
 ' < deny-tools.txt 2>/dev/null)
git -C "$REPO" fetch -q origin || { echo "fetch failed"; exit 1; }
git -C "$REPO" checkout -q observations 2>/dev/null || git -C "$REPO" checkout -q -b observations origin/main || exit 1
for i in $(seq 1 40); do
  [ -f STOP ] && { echo "STOP file found: $(cat STOP)"; break; }
  git -C "$REPO" pull -q --no-edit origin main || { echo "pull from main failed; fix the tree and rerun"; break; }
  grep -q "| todo |" "$REPO/docs/parity/QUEUE.md" || { echo "Queue empty, done."; break; }
  echo "=== round $i: $(date) ==="
  claude -p "$(cat run-queue.md)

This run: do ONLY the first todo row of QUEUE.md on the observations branch, commit and push it (git push origin observations), then stop. Follow QUEUE.md rules 6, 10 and 11 exactly." \
    --add-dir "$REPO" --permission-mode acceptEdits \
    --allowedTools "$ALLOW" ${DENY:+--disallowedTools "$DENY"} 2>&1 | tee -a "run-$(date +%Y%m%d).log"
  git -C "$REPO" push -q origin observations || echo "push failed (will retry next round)"
done
