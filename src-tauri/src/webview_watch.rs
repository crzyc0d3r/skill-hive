/// Forward WebView process crashes into the Rust log.
/// Windows is the only target whose webview reports process-failed.
pub fn watch_process_failures(window: &tauri::WebviewWindow) {
    #[cfg(windows)]
    watch_windows(window);
    #[cfg(not(windows))]
    let _ = window;
}

#[cfg(windows)]
fn watch_windows(window: &tauri::WebviewWindow) {
    use webview2_com::Microsoft::Web::WebView2::Win32::COREWEBVIEW2_PROCESS_FAILED_KIND;

    if let Err(err) = window.with_webview(|webview| {
        let core = match unsafe { webview.controller().CoreWebView2() } {
            Ok(core) => core,
            Err(err) => {
                log::error!("webview-process-failed hook skipped: {err}");
                return;
            }
        };
        let handler =
            match webview2_com::ProcessFailedEventHandler::create(Box::new(|_sender, args| {
                let mut kind = COREWEBVIEW2_PROCESS_FAILED_KIND::default();
                if let Some(args) = args.as_ref() {
                    unsafe {
                        let _ = args.ProcessFailedKind(&mut kind);
                    }
                }
                log::error!("webview-process-failed kind={kind:?}");
                Ok(())
            })) {
                Ok(handler) => handler,
                Err(err) => {
                    log::error!("webview-process-failed handler: {err}");
                    return;
                }
            };
        let mut token = 0i64;
        if let Err(err) = unsafe { core.add_ProcessFailed(&handler, &mut token) } {
            log::error!("webview-process-failed subscribe: {err}");
            return;
        }
        // The event source holds its own reference. Forget the wrapper so a
        // later drop cannot release the callback while the window is alive.
        std::mem::forget(handler);
    }) {
        log::error!("webview-process-failed attach: {err}");
    }
}
