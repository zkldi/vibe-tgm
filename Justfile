OPEN_CMD := if os() == "macos" { "open" } else { "xdg-open" }

import "Justfile-test"
import "Justfile-game"
import "Justfile-build"

default:
	-@just --choose
