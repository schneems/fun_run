//! Detect support for `command_resolved_envs` <https://github.com/rust-lang/rust/issues/149070>
//!
//! Uses `autocfg` to compile a probe with the same rustc the crate is built
//! with. The probe only succeeds on a toolchain where the unstable
//! `command_resolved_envs` feature exists (currently nightly).

const PROBE: &str = r#"
#![feature(command_resolved_envs)]
pub fn probe() {
    let cmd = std::process::Command::new("does-not-run");
    let _ = cmd.get_resolved_envs();
}
"#;

fn main() {
    autocfg::rerun_path("build.rs");

    let ac = autocfg::new();
    // Declare the cfg for check-cfg so clippy's `--deny warnings` stays quiet.
    autocfg::emit_possibility("command_resolved_envs");
    if ac.probe_raw(PROBE).is_ok() {
        autocfg::emit("command_resolved_envs");
    }
}
