fn main() {
    // Emit debug info so the `i-slint-backend-testing` ElementHandle API
    // (used by tests/navigation.rs) can look up elements by accessible
    // label. Without this the compiled UI has no element/id mapping and
    // every ElementHandle lookup silently returns nothing.
    //
    // Translations are bundled into the binary; the `.slint` text is the
    // English source and `translations/<lang>/LC_MESSAGES/kiosk-gui.po`
    // holds the others. No per-component context: one phrase, one entry.
    let config = slint_build::CompilerConfiguration::new()
        .with_debug_info(true)
        .with_bundled_translations("translations")
        .with_default_translation_context(slint_build::DefaultTranslationContext::None);
    slint_build::compile_with_config("ui/app.slint", config).unwrap();
}
