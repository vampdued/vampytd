pub fn send_notification(title: &str, body: &str) {
    #[cfg(target_os = "windows")]
    {
        use notify_rust::Notification;
        let mut notif = Notification::new();
        notif.app_id("VampYTD")
            .appname("VampYTD")
            .summary(title)
            .body(body);

        if let Some(local_app_data) = std::env::var_os("LOCALAPPDATA") {
            let icon_path = std::path::PathBuf::from(local_app_data).join("VampYTD").join("app-icon.png");
            if icon_path.exists() {
                if let Some(icon_str) = icon_path.to_str() {
                    notif.icon(icon_str);
                }
            }
        }
        let _ = notif.show();
    }
    #[cfg(not(target_os = "windows"))]
    {
        use notify_rust::Notification;
        let _ = Notification::new()
            .appname("VampYTD")
            .summary(title)
            .body(body)
            .show();
    }
}
