fn main() {
    // Emit debug info so the `i-slint-backend-testing` ElementHandle API
    // (used by tests/navigation.rs) can look up elements by accessible
    // label. Without this the compiled UI has no element/id mapping and
    // every ElementHandle lookup silently returns nothing.
    let config =
        slint_build::CompilerConfiguration::new().with_debug_info(true);
    slint_build::compile_with_config("ui/app.slint", config).unwrap();
}
