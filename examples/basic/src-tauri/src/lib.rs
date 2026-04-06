use tauri::App;

pub fn run_app(app: &mut App) -> Result<(), Box<dyn std::error::Error>> {
    let _ = app;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let context = tauri::generate_context!();

    // Option A: Read config from tauri.conf.json (recommended)
    let (hotswap_plugin, context) = tauri_plugin_hotswap::init(context)
        .expect("failed to initialize hotswap plugin");

    // Option B: Explicit config
    // let (hotswap_plugin, context) = tauri_plugin_hotswap::init_with_config(
    //     context,
    //     tauri_plugin_hotswap::HotswapConfig::new("REPLACE_WITH_YOUR_PUBKEY")
    //         .endpoint("https://example.com/api/ota/{{current_sequence}}"),
    // ).expect("failed to initialize hotswap plugin");

    // Option C: Custom resolver
    // let (hotswap_plugin, context) = tauri_plugin_hotswap::HotswapBuilder::new("YOUR_PUBKEY")
    //     .resolver(tauri_plugin_hotswap::StaticFileResolver::new(
    //         "https://cdn.example.com/ota/latest.json",
    //     ))
    //     .build(context)
    //     .expect("failed to initialize hotswap plugin");

    tauri::Builder::default()
        .plugin(hotswap_plugin)
        .setup(|app| run_app(app))
        .run(context)
        .expect("error while running tauri application");
}
