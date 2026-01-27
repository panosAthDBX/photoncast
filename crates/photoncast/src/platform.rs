//! Platform-specific functionality for PhotonCast

#[cfg(target_os = "macos")]
mod macos {
    use std::cell::RefCell;
    use std::path::Path;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};

    use core_foundation::runloop::{kCFRunLoopCommonModes, kCFRunLoopDefaultMode, CFRunLoop};
    use core_graphics::event::{
        CGEvent, CGEventFlags, CGEventTap, CGEventTapLocation, CGEventTapOptions,
        CGEventTapPlacement, CGEventType,
    };

    use objc2::rc::Retained;
    use objc2::sel;
    #[allow(deprecated)]
    use objc2::{define_class, msg_send_id, AllocAnyThread, MainThreadOnly};
    use objc2_app_kit::{
        NSApplication, NSBitmapImageFileType, NSBitmapImageRep, NSButton, NSImage, NSMenu,
        NSMenuItem, NSStatusBar, NSStatusItem, NSWorkspace,
    };
    use objc2_foundation::{
        MainThreadMarker, NSDictionary, NSObject, NSObjectProtocol, NSRect, NSSize, NSString,
    };

    use tracing::{debug, error, info, warn};

    /// Resize the key window to the specified size, keeping top-center position fixed.
    /// Uses dispatch_async to defer the resize outside of GPUI's event loop.
    pub fn resize_window(new_width: f64, new_height: f64) {
        use dispatch::Queue;

        // Use dispatch_async to schedule resize on next run loop iteration
        // This avoids RefCell borrow conflicts with GPUI's window management
        Queue::main().exec_async(move || {
            // SAFETY: dispatch_async to main queue ensures we're on the main thread
            let mtm = unsafe { MainThreadMarker::new_unchecked() };

            let app = NSApplication::sharedApplication(mtm);
            let Some(window) = app.keyWindow() else {
                return;
            };

            let current_frame = window.frame();

            // Calculate new frame - keep top-center position fixed
            let width_diff = new_width - current_frame.size.width;
            let height_diff = new_height - current_frame.size.height;
            let new_frame = NSRect::new(
                objc2_foundation::NSPoint::new(
                    current_frame.origin.x - (width_diff / 2.0), // Center horizontally
                    current_frame.origin.y - height_diff,        // Keep top fixed, expand downward
                ),
                objc2_foundation::NSSize::new(new_width, new_height),
            );

            // Animate the resize
            window.setFrame_display_animate(new_frame, true, true);
        });
    }

    /// Gets app icon using NSWorkspace and returns PNG data.
    /// This handles all icon formats (icns, asset catalogs, etc.)
    ///
    /// MUST be called from the main thread (GPUI ensures this for UI operations).
    pub fn get_app_icon_png(app_path: &Path, size: u32) -> Option<Vec<u8>> {
        // SAFETY: This function is called from GPUI UI code which runs on the main thread
        let _mtm = unsafe { MainThreadMarker::new_unchecked() };

        let path_str = app_path.to_string_lossy();
        let ns_path = NSString::from_str(&path_str);

        // Get the shared workspace (requires main thread)
        let workspace = NSWorkspace::sharedWorkspace();
        let icon: Retained<NSImage> = workspace.iconForFile(&ns_path);

        // Set the icon size for rendering
        let target_size = NSSize::new(f64::from(size), f64::from(size));
        icon.setSize(target_size);

        // Get TIFF representation from NSImage
        let Some(tiff_data) = icon.TIFFRepresentation() else {
            tracing::warn!(
                "Failed to get TIFF representation for {}",
                app_path.display()
            );
            return None;
        };

        // Create bitmap image rep from the TIFF data
        let bitmap: Option<Retained<NSBitmapImageRep>> =
            NSBitmapImageRep::initWithData(NSBitmapImageRep::alloc(), &tiff_data);

        let Some(bitmap) = bitmap else {
            tracing::warn!("Failed to create bitmap rep for {}", app_path.display());
            return None;
        };

        // Create empty dictionary for PNG properties
        let empty_dict: Retained<NSDictionary<NSString>> = NSDictionary::dictionary();

        // Convert to PNG data
        let Some(png_data) = (unsafe {
            bitmap.representationUsingType_properties(NSBitmapImageFileType::PNG, &empty_dict)
        }) else {
            tracing::warn!("Failed to convert to PNG for {}", app_path.display());
            return None;
        };

        // Convert NSData to Vec<u8>
        // Safety: We're not mutating png_data while the slice is alive
        let bytes = unsafe { png_data.as_bytes_unchecked() };
        Some(bytes.to_vec())
    }

    /// Saves app icon as PNG to the specified path.
    /// Returns true on success.
    pub fn save_app_icon_as_png(app_path: &Path, output_path: &Path, size: u32) -> bool {
        match get_app_icon_png(app_path, size) {
            Some(png_data) => {
                if let Err(e) = std::fs::write(output_path, &png_data) {
                    tracing::warn!("Failed to write icon file: {}", e);
                    false
                } else {
                    true
                }
            },
            None => {
                tracing::debug!("get_app_icon_png returned None for {}", app_path.display());
                false
            },
        }
    }

    /// Gets app path for a bundle ID using NSWorkspace.
    #[allow(dead_code)]
    pub fn get_app_path_for_bundle_id(bundle_id: &str) -> Option<std::path::PathBuf> {
        use objc2::rc::Retained;
        use objc2_app_kit::NSWorkspace;
        use objc2_foundation::NSString;

        let workspace = NSWorkspace::sharedWorkspace();
        let bundle_id_ns = NSString::from_str(bundle_id);

        let url: Option<Retained<objc2_foundation::NSURL>> =
            workspace.URLForApplicationWithBundleIdentifier(&bundle_id_ns);

        url.and_then(|u| u.path())
            .map(|p| std::path::PathBuf::from(p.to_string()))
    }

    // =========================================================================
    // Global Hotkey Registration via CGEventTap
    // =========================================================================

    /// Virtual key code for Space
    const KEY_SPACE: i64 = 49;
    /// Virtual key code for V
    const KEY_V: i64 = 9;

    /// Callback type for hotkey activation
    pub type HotkeyCallback = Box<dyn Fn() + Send + Sync>;

    /// Global state for the hotkey system
    static HOTKEY_ACTIVE: AtomicBool = AtomicBool::new(false);
    static HOTKEY_CALLBACK: Mutex<Option<Arc<HotkeyCallback>>> = Mutex::new(None);
    /// Callback for clipboard hotkey (Cmd+Shift+V)
    static CLIPBOARD_HOTKEY_CALLBACK: Mutex<Option<Arc<HotkeyCallback>>> = Mutex::new(None);

    /// Registers a global hotkey (Cmd+Space by default).
    ///
    /// Returns `Ok(())` if registration succeeded, or an error message.
    /// The callback will be invoked on a background thread when the hotkey is pressed.
    pub fn register_global_hotkey<F>(callback: F) -> Result<(), String>
    where
        F: Fn() + Send + Sync + 'static,
    {
        // Check if already registered
        if HOTKEY_ACTIVE.load(Ordering::SeqCst) {
            warn!("Global hotkey already registered");
            return Ok(());
        }

        info!("Registering global hotkey (Cmd+Space)");

        // Store the callback
        {
            let mut cb = HOTKEY_CALLBACK.lock().map_err(|e| e.to_string())?;
            *cb = Some(Arc::new(Box::new(callback)));
        }

        // Spawn a thread to run the event tap
        std::thread::spawn(|| {
            if let Err(e) = run_event_tap() {
                error!("Event tap failed: {}", e);
                HOTKEY_ACTIVE.store(false, Ordering::SeqCst);
            }
        });

        HOTKEY_ACTIVE.store(true, Ordering::SeqCst);
        Ok(())
    }

    /// Unregisters the global hotkey.
    pub fn unregister_global_hotkey() {
        info!("Unregistering global hotkey");
        HOTKEY_ACTIVE.store(false, Ordering::SeqCst);
        if let Ok(mut cb) = HOTKEY_CALLBACK.lock() {
            *cb = None;
        }
        if let Ok(mut cb) = CLIPBOARD_HOTKEY_CALLBACK.lock() {
            *cb = None;
        }
    }

    /// Registers the clipboard hotkey (Cmd+Shift+V).
    /// Must be called after `register_global_hotkey` as it uses the same event tap.
    pub fn register_clipboard_hotkey<F>(callback: F) -> Result<(), String>
    where
        F: Fn() + Send + Sync + 'static,
    {
        info!("Registering clipboard hotkey (Cmd+Shift+V)");
        let mut cb = CLIPBOARD_HOTKEY_CALLBACK
            .lock()
            .map_err(|e| e.to_string())?;
        *cb = Some(Arc::new(Box::new(callback)));
        Ok(())
    }

    /// Returns whether a global hotkey is currently registered.
    #[allow(dead_code)]
    pub fn is_hotkey_registered() -> bool {
        HOTKEY_ACTIVE.load(Ordering::SeqCst)
    }

    /// Event tap callback that checks for Cmd+Space
    fn event_tap_callback(
        _proxy: core_graphics::event::CGEventTapProxy,
        event_type: CGEventType,
        event: &CGEvent,
    ) -> Option<CGEvent> {
        // Only handle key down events
        if !matches!(event_type, CGEventType::KeyDown) {
            return Some(event.clone());
        }

        // Get key code and modifiers
        let keycode =
            event.get_integer_value_field(core_graphics::event::EventField::KEYBOARD_EVENT_KEYCODE);
        let flags = event.get_flags();

        // Check for Cmd+Space (keycode 49 = Space, command flag set)
        let is_cmd_space = keycode == KEY_SPACE
            && flags.contains(CGEventFlags::CGEventFlagCommand)
            && !flags.contains(CGEventFlags::CGEventFlagShift)
            && !flags.contains(CGEventFlags::CGEventFlagControl)
            && !flags.contains(CGEventFlags::CGEventFlagAlternate);

        // Check for Cmd+Shift+V (keycode 9 = V, command + shift flags set)
        let is_cmd_shift_v = keycode == KEY_V
            && flags.contains(CGEventFlags::CGEventFlagCommand)
            && flags.contains(CGEventFlags::CGEventFlagShift)
            && !flags.contains(CGEventFlags::CGEventFlagControl)
            && !flags.contains(CGEventFlags::CGEventFlagAlternate);

        if is_cmd_space {
            debug!("Hotkey detected: Cmd+Space");

            // Invoke the callback
            if let Ok(cb_guard) = HOTKEY_CALLBACK.lock() {
                if let Some(callback) = cb_guard.as_ref() {
                    let callback = Arc::clone(callback);
                    // Invoke callback (don't block the event tap)
                    std::thread::spawn(move || {
                        callback();
                    });
                }
            }

            // Consume the event (don't pass to other apps)
            return None;
        }

        if is_cmd_shift_v {
            debug!("Hotkey detected: Cmd+Shift+V");

            // Invoke the clipboard callback
            if let Ok(cb_guard) = CLIPBOARD_HOTKEY_CALLBACK.lock() {
                if let Some(callback) = cb_guard.as_ref() {
                    let callback = Arc::clone(callback);
                    // Invoke callback (don't block the event tap)
                    std::thread::spawn(move || {
                        callback();
                    });
                }
            }

            // Consume the event (don't pass to other apps)
            return None;
        }

        // Pass through other events
        Some(event.clone())
    }

    /// Runs the `CGEventTap` on the current thread.
    fn run_event_tap() -> Result<(), String> {
        // Create event tap
        let event_tap = CGEventTap::new(
            CGEventTapLocation::Session,
            CGEventTapPlacement::HeadInsertEventTap,
            CGEventTapOptions::Default,
            vec![CGEventType::KeyDown],
            event_tap_callback,
        )
        .map_err(|()| "Failed to create event tap. Is accessibility permission granted?")?;

        // Enable the tap
        event_tap.enable();

        // Add to run loop
        let source = event_tap
            .mach_port
            .create_runloop_source(0)
            .map_err(|()| "Failed to create run loop source")?;

        let run_loop = CFRunLoop::get_current();
        run_loop.add_source(&source, unsafe { kCFRunLoopCommonModes });

        info!("Event tap started, listening for Cmd+Space");

        // Run the loop until hotkey is unregistered
        while HOTKEY_ACTIVE.load(Ordering::SeqCst) {
            // Run for a short interval then check if we should stop
            // Note: Must use kCFRunLoopDefaultMode, not kCFRunLoopCommonModes
            // kCFRunLoopCommonModes is only valid for adding sources, not running
            CFRunLoop::run_in_mode(
                unsafe { kCFRunLoopDefaultMode },
                std::time::Duration::from_millis(100),
                false,
            );
        }

        info!("Event tap stopped");
        Ok(())
    }

    // =========================================================================
    // Menu Bar (Status Item) Integration
    // =========================================================================

    // Thread-local storage for the status item (NSStatusItem is MainThreadOnly)
    thread_local! {
        static MENU_BAR_STATUS_ITEM: RefCell<Option<Retained<NSStatusItem>>> = const { RefCell::new(None) };
    }

    /// Callback type for menu bar actions
    pub type MenuBarCallback = Box<dyn Fn(MenuBarActionKind) + Send + Sync>;

    /// Menu bar action kinds.
    ///
    /// Note: Variants are constructed in main.rs, not in this module.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    #[allow(dead_code)]
    pub enum MenuBarActionKind {
        /// Toggle the launcher window
        ToggleLauncher,
        /// Open preferences
        OpenPreferences,
        /// Quit the application
        Quit,
    }

    /// Global callback for menu bar actions
    static MENU_BAR_CALLBACK: Mutex<Option<Arc<MenuBarCallback>>> = Mutex::new(None);

    define_class!(
        // SAFETY: NSObject has no subclassing requirements and we don't implement Drop.
        #[unsafe(super(NSObject))]
        #[thread_kind = MainThreadOnly]
        #[name = "PhotonCastMenuBarTarget"]
        struct MenuBarTarget;

        impl MenuBarTarget {
            #[unsafe(method(menuBarItemSelected:))]
            fn menu_bar_item_selected(&self, item: &NSMenuItem) {
                let tag = item.tag();
                let action = match tag {
                    1 => Some(MenuBarActionKind::ToggleLauncher),
                    2 => Some(MenuBarActionKind::OpenPreferences),
                    3 => Some(MenuBarActionKind::Quit),
                    _ => None,
                };

                if let Some(action) = action {
                    if let Ok(cb_guard) = MENU_BAR_CALLBACK.lock() {
                        if let Some(callback) = cb_guard.as_ref() {
                            callback(action);
                        }
                    }
                }
            }
        }

        unsafe impl NSObjectProtocol for MenuBarTarget {}
    );

    thread_local! {
        static MENU_BAR_TARGET: RefCell<Option<Retained<MenuBarTarget>>> = const { RefCell::new(None) };
    }

    /// Creates and shows the menu bar status item with a menu.
    ///
    /// This function must be called from the main thread (which GPUI ensures).
    ///
    /// # Arguments
    /// * `callback` - Function called when menu items are selected
    ///
    /// # Returns
    /// `Ok(())` on success, or an error message on failure.
    pub fn create_menu_bar_item<F>(callback: F) -> Result<(), String>
    where
        F: Fn(MenuBarActionKind) + Send + Sync + 'static,
    {
        info!("Creating menu bar status item");

        // Store the callback
        {
            let mut cb = MENU_BAR_CALLBACK.lock().map_err(|e| e.to_string())?;
            *cb = Some(Arc::new(Box::new(callback)));
        }

        // SAFETY: This function is called from GPUI which runs on the main thread
        let mtm = unsafe { MainThreadMarker::new_unchecked() };

        // Get the system status bar
        let status_bar = NSStatusBar::systemStatusBar();

        // Create status item with variable length (NSVariableStatusItemLength = -1)
        let status_item = status_bar.statusItemWithLength(-1.0);

        // Set the button title to show an icon in the menu bar
        // Use objc2 msg_send! since button() isn't exposed in objc2-app-kit 0.2
        #[allow(deprecated)]
        let button: Option<Retained<NSButton>> = unsafe { msg_send_id![&status_item, button] };

        if let Some(button) = button {
            let title = NSString::from_str("⚡");
            button.setTitle(&title);
        } else {
            warn!("Could not get status item button to set title");
        }

        // Create the menu
        let menu = create_status_menu(mtm);

        let target = mtm.alloc::<MenuBarTarget>().set_ivars(());
        #[allow(deprecated)]
        let target: Retained<MenuBarTarget> = unsafe { msg_send_id![super(target), init] };
        for tag in [1, 2, 3] {
            if let Some(item) = menu.itemWithTag(tag) {
                unsafe {
                    item.setTarget(Some(target.as_ref()));
                    item.setAction(Some(sel!(menuBarItemSelected:)));
                }
            }
        }

        MENU_BAR_TARGET.with(|cell| {
            *cell.borrow_mut() = Some(target);
        });

        status_item.setMenu(Some(&menu));

        // Store the status item in thread-local storage to keep it alive
        MENU_BAR_STATUS_ITEM.with(|cell| {
            *cell.borrow_mut() = Some(status_item);
        });

        info!("Menu bar status item created successfully");
        Ok(())
    }

    /// Creates the dropdown menu for the status item.
    fn create_status_menu(mtm: MainThreadMarker) -> Retained<NSMenu> {
        let menu = NSMenu::new(mtm);

        // "Open PhotonCast" item (shows ⌘Space shortcut hint)
        let open_title = NSString::from_str("Open PhotonCast");
        let open_key = NSString::from_str(" "); // Space key
        let open_item = unsafe {
            NSMenuItem::initWithTitle_action_keyEquivalent(
                mtm.alloc::<NSMenuItem>(),
                &open_title,
                None, // No action - user uses global hotkey
                &open_key,
            )
        };
        open_item.setTag(1);
        // Disable the item since it doesn't have an action (shows greyed out with shortcut hint)
        open_item.setEnabled(false);
        menu.addItem(&open_item);

        // Separator
        let separator = NSMenuItem::separatorItem(mtm);
        menu.addItem(&separator);

        // "Preferences..." item
        let prefs_title = NSString::from_str("Preferences...");
        let prefs_key = NSString::from_str(",");
        let prefs_item = unsafe {
            NSMenuItem::initWithTitle_action_keyEquivalent(
                mtm.alloc::<NSMenuItem>(),
                &prefs_title,
                None,
                &prefs_key,
            )
        };
        prefs_item.setTag(2);
        menu.addItem(&prefs_item);

        // Separator
        let separator2 = NSMenuItem::separatorItem(mtm);
        menu.addItem(&separator2);

        // "Quit PhotonCast" item - uses NSApplication's terminate: selector
        let quit_title = NSString::from_str("Quit PhotonCast");
        let quit_key = NSString::from_str("q");
        let quit_item = unsafe {
            NSMenuItem::initWithTitle_action_keyEquivalent(
                mtm.alloc::<NSMenuItem>(),
                &quit_title,
                None,
                &quit_key,
            )
        };
        quit_item.setTag(3);
        menu.addItem(&quit_item);

        menu
    }

    /// Removes the menu bar status item.
    ///
    /// Note: Currently unused but kept as part of the public API for future use
    /// (e.g., cleanup on shutdown, dynamic menu bar toggling).
    #[allow(dead_code)]
    pub fn remove_menu_bar_item() {
        info!("Removing menu bar status item");

        MENU_BAR_STATUS_ITEM.with(|cell| {
            if let Some(status_item) = cell.borrow_mut().take() {
                let status_bar = NSStatusBar::systemStatusBar();
                status_bar.removeStatusItem(&status_item);
            }
        });

        if let Ok(mut cb) = MENU_BAR_CALLBACK.lock() {
            *cb = None;
        }
    }

    /// Returns true if the menu bar item is active.
    ///
    /// Note: Currently unused but kept as part of the public API for future use.
    #[allow(dead_code)]
    pub fn is_menu_bar_active() -> bool {
        MENU_BAR_STATUS_ITEM.with(|cell| cell.borrow().is_some())
    }

    /// Triggers the menu bar callback with the specified action.
    /// This is called when a menu item is selected.
    ///
    /// Note: Currently unused but kept as part of the public API for future use
    /// (e.g., programmatic menu actions, testing).
    #[allow(dead_code)]
    pub fn trigger_menu_bar_action(action: MenuBarActionKind) {
        if let Ok(cb_guard) = MENU_BAR_CALLBACK.lock() {
            if let Some(callback) = cb_guard.as_ref() {
                let callback = Arc::clone(callback);
                // Invoke callback
                callback(action);
            }
        }
    }
}

#[cfg(target_os = "macos")]
pub use macos::*;

#[cfg(not(target_os = "macos"))]
pub fn resize_window(_new_width: f64, _new_height: f64) {
    // No-op on other platforms
}

#[cfg(not(target_os = "macos"))]
pub fn get_app_icon_png(_app_path: &std::path::Path, _size: u32) -> Option<Vec<u8>> {
    None
}

#[cfg(not(target_os = "macos"))]
pub fn save_app_icon_as_png(
    _app_path: &std::path::Path,
    _output_path: &std::path::Path,
    _size: u32,
) -> bool {
    false
}

#[cfg(not(target_os = "macos"))]
pub fn get_app_path_for_bundle_id(_bundle_id: &str) -> Option<std::path::PathBuf> {
    None
}

#[cfg(not(target_os = "macos"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuBarActionKind {
    ToggleLauncher,
    OpenPreferences,
    Quit,
}

#[cfg(not(target_os = "macos"))]
pub fn create_menu_bar_item<F>(_callback: F) -> Result<(), String>
where
    F: Fn(MenuBarActionKind) + Send + Sync + 'static,
{
    Ok(())
}

#[cfg(not(target_os = "macos"))]
pub fn remove_menu_bar_item() {}

#[cfg(not(target_os = "macos"))]
pub fn is_menu_bar_active() -> bool {
    false
}

#[cfg(not(target_os = "macos"))]
pub fn trigger_menu_bar_action(_action: MenuBarActionKind) {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    #[cfg(target_os = "macos")]
    fn test_get_app_icon_png() {
        let safari_path = Path::new("/Applications/Safari.app");
        if safari_path.exists() {
            let result = get_app_icon_png(safari_path, 64);
            assert!(result.is_some(), "Should be able to extract Safari icon");
            let data = result.unwrap();
            assert!(!data.is_empty(), "Icon data should not be empty");
            // Check PNG magic bytes
            assert_eq!(
                &data[0..8],
                &[137, 80, 78, 71, 13, 10, 26, 10],
                "Should be valid PNG"
            );
        }
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_save_app_icon_as_png() {
        let safari_path = Path::new("/Applications/Safari.app");
        if safari_path.exists() {
            let output_path = std::env::temp_dir().join("test_safari_icon.png");
            let result = save_app_icon_as_png(safari_path, &output_path, 64);
            assert!(result, "Should successfully save Safari icon");
            assert!(output_path.exists(), "Output file should exist");
            // Cleanup
            let _ = std::fs::remove_file(&output_path);
        }
    }
}
