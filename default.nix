# This file provides backward compatibility for tools and environments
# that don't support flakes directly.
# For now, it just imports shell.nix since we can't use flake-compat
# without adding it as a flake input.
import ./shell.nix