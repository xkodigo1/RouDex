fn main() {
    // Embeds the Windows manifest (Common Controls v6) and the app icon;
    // without it the executable fails to start with STATUS_ENTRYPOINT_NOT_FOUND.
    tauri_build::build();
    // tauri-build links that manifest into binaries only. Integration tests
    // that start the mock runtime need it too, so they get the same manifest.
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        let manifest =
            std::path::Path::new(&std::env::var("OUT_DIR").unwrap()).join("tests-app-manifest.xml");
        std::fs::write(&manifest, TESTS_MANIFEST).expect("write tests manifest");
        println!("cargo:rustc-link-arg-tests=/MANIFEST:EMBED");
        println!(
            "cargo:rustc-link-arg-tests=/MANIFESTINPUT:{}",
            manifest.display()
        );
    }
}

/// Same content as tauri-build's default Windows app manifest.
const TESTS_MANIFEST: &str = r#"<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <dependency>
    <dependentAssembly>
      <assemblyIdentity
        type="win32"
        name="Microsoft.Windows.Common-Controls"
        version="6.0.0.0"
        processorArchitecture="*"
        publicKeyToken="6595b64144ccf1df"
        language="*"
      />
    </dependentAssembly>
  </dependency>
</assembly>
"#;
