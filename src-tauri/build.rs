fn main() {
    // `option_env!` in state/mod.rs bakes the Google OAuth creds into the binary
    // at build time. Tell cargo to recompile when either changes, so a creds
    // update isn't masked by a stale cached build.
    println!("cargo:rerun-if-env-changed=GOOGLE_CLIENT_ID");
    println!("cargo:rerun-if-env-changed=GOOGLE_CLIENT_SECRET");
    tauri_build::build();
}
