pub fn send_notification(title: &str, body: &str) {
    #[cfg(target_os = "windows")]
    {
        use notify_rust::Notification;
        let _ = Notification::new()
            .app_id("VampYTD")
            .appname("VampYTD")
            .summary(title)
            .body(body)
            .show();
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

