OPEN_CMD := if os() == "macos" { "open" } else { "xdg-open" }

import "Justfile-test"
import "Justfile-game"

default:
	-@just --choose
