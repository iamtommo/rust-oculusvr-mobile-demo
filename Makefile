mobile:
	cargo ndk --platform 21 --target x86_64-linux-android build

test:
	cargo test --target aarch64-linux-android