#!/usr/bin/env bash
# Drop in a GameCube DOL and run it: recompile -> build native binary -> run.
# Usage: ./play.sh path/to/game.dol
set -euo pipefail
DOL="${1:?usage: ./play.sh <game.dol>}"
echo ">> Recompiling $DOL ..."
cargo run -q -p gcrecomp-cli -- recompile --dol-file "$DOL"
echo ">> Building native binary ..."
cargo build -p game
echo ">> Running ..."
exec ./target/debug/game
