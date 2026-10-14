fn main() {
    // Embeds the Windows manifest (Common Controls v6) and the app icon;
    // without it the executable fails to start with STATUS_ENTRYPOINT_NOT_FOUND.
    tauri_build::build();
}
