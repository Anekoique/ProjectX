LOG := off

export X_LOG=$(LOG)

all: run

run:
	cargo run
