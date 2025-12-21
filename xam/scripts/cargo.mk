ifeq ($(VERBOSE),1)
	verbose := -v
else ifeq ($(VERBOSE),2)
	verbose := -vv
else
	verbose :=
endif

build_args-release := --release

build_args := \
	-Z unstable-options \
	--target $(TARGET) \
	$(build_args-$(MODE)) \
	$(verbose)

define cargo_build
	$(call run_cmd,cargo build,--manifest-path $(1)/Cargo.toml $(build_args))
endef

clippy_args := -A clippy::new_without_default -A unsafe_op_in_unsafe_fn

define cargo_clippy
	$(call run_cmd,cargo clippy,--all-features --workspace $(1) $(verbose) -- $(clippy_args))
endef

define cargo_clean
	$(call run_cmd,cargo clean,-Z unstable-options --manifest-path $(1)/Cargo.toml)
endef
