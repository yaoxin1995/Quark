.PHONY: debug release cc_debug cc_release

debug:
	CARGO_TARGET_DIR=../target cargo +nightly-2023-12-11-x86_64-unknown-linux-gnu build 

release:
	CARGO_TARGET_DIR=../target cargo +nightly-2023-12-11-x86_64-unknown-linux-gnu build --release

cc_debug:
	RUSTFLAGS="--cfg aes_force_soft --cfg polyval_force_soft" CARGO_TARGET_DIR=../target cargo +nightly-2023-12-11-x86_64-unknown-linux-gnu build --features cc

cc_release:
	RUSTFLAGS="--cfg aes_force_soft --cfg polyval_force_soft" CARGO_TARGET_DIR=../target cargo +nightly-2023-12-11-x86_64-unknown-linux-gnu build --release --features cc