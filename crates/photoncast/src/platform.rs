//! Platform-specific functionality for PhotonCast

#[cfg(target_os = "macos")]
mod macos {
    use std::cell::RefCell;
    use std::path::Path;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};

    use objc2::rc::Retained;
    use objc2::sel;
    #[allow(deprecated)]
    use objc2::{define_class, msg_send_id, AllocAnyThread, MainThreadOnly};
    use objc2_app_kit::{
        NSApplication, NSApplicationActivationPolicy, NSBitmapImageFileType, NSBitmapImageRep,
        NSButton, NSImage, NSMenu, NSMenuItem, NSStatusBar, NSStatusItem, NSWorkspace,
    };
    use objc2_foundation::{
        MainThreadMarker, NSDictionary, NSObject, NSObjectProtocol, NSRect, NSSize, NSString,
    };

    use tracing::{debug, info, warn};

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

    /// Sets the app activation policy to match the intended Dock visibility.
    ///
    /// `show_in_dock = true` uses a regular app activation policy.
    /// `show_in_dock = false` uses accessory mode so the app stays out of the Dock
    /// while still allowing windows and a menu bar status item.
    pub fn sync_activation_policy(show_in_dock: bool) -> Result<(), String> {
        let mtm = MainThreadMarker::new()
            .ok_or_else(|| "sync_activation_policy must run on the main thread".to_string())?;
        let app = NSApplication::sharedApplication(mtm);
        let policy = if show_in_dock {
            NSApplicationActivationPolicy::Regular
        } else {
            NSApplicationActivationPolicy::Accessory
        };

        #[allow(deprecated)]
        let changed = app.setActivationPolicy(policy);
        if changed {
            info!(show_in_dock, ?policy, "Updated app activation policy");
            Ok(())
        } else {
            Err(format!("Failed to set activation policy to {:?}", policy))
        }
    }

    /// Explicitly foregrounds the current app, even when running as a UIElement.
    ///
    /// This is intentionally a narrow launcher-facing escape hatch for cases
    /// where window-local GPUI activation is insufficient to make PhotonCast
    /// the true frontmost app.
    pub fn activate_ignoring_other_apps() -> Result<(), String> {
        let mtm = MainThreadMarker::new().ok_or_else(|| {
            "activate_ignoring_other_apps must run on the main thread".to_string()
        })?;
        let app = NSApplication::sharedApplication(mtm);
        #[allow(deprecated)]
        app.activateIgnoringOtherApps(true);
        info!("Requested app foreground activation");
        Ok(())
    }

    /// Gets app icon using NSWorkspace and returns PNG data.
    /// This handles all icon formats (icns, asset catalogs, etc.).
    ///
    /// If called off the main thread, the AppKit work is dispatched
    /// synchronously onto the main queue.
    pub fn get_app_icon_png(app_path: &Path, size: u32) -> Option<Vec<u8>> {
        if MainThreadMarker::new().is_some() {
            return get_app_icon_png_on_main_thread(app_path, size);
        }

        let app_path = app_path.to_path_buf();
        dispatch::Queue::main().exec_sync(move || get_app_icon_png_on_main_thread(&app_path, size))
    }

    fn get_app_icon_png_on_main_thread(app_path: &Path, size: u32) -> Option<Vec<u8>> {
        let _mtm = MainThreadMarker::new().expect("App icon extraction must run on main thread");

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
    // Global Hotkey Registration via Carbon RegisterEventHotKey
    // =========================================================================

    mod carbon_hotkey {
        use std::ffi::c_void;

        #[repr(C)]
        pub struct EventTypeSpec {
            pub event_class: u32,
            pub event_kind: u32,
        }

        #[repr(C)]
        #[derive(Clone, Copy)]
        pub struct EventHotKeyID {
            pub signature: u32,
            pub id: u32,
        }

        pub const EVENT_CLASS_KEYBOARD: u32 = u32::from_be_bytes(*b"keyb");
        pub const EVENT_HOT_KEY_PRESSED: u32 = 5;
        pub const EVENT_PARAM_DIRECT_OBJECT: u32 = u32::from_be_bytes(*b"----");
        pub const TYPE_EVENT_HOT_KEY_ID: u32 = u32::from_be_bytes(*b"hkid");
        pub const CMD_KEY_MASK: u32 = 1 << 8;
        pub const SHIFT_KEY_MASK: u32 = 1 << 9;
        pub const NO_ERR: i32 = 0;

        pub type EventHandlerCallRef = *mut c_void;
        pub type EventRef = *mut c_void;

        #[link(name = "Carbon", kind = "framework")]
        extern "C" {
            pub fn GetApplicationEventTarget() -> *mut c_void;
            pub fn InstallEventHandler(
                target: *mut c_void,
                handler: unsafe extern "C" fn(EventHandlerCallRef, EventRef, *mut c_void) -> i32,
                num_types: u32,
                list: *const EventTypeSpec,
                user_data: *mut c_void,
                out_ref: *mut *mut c_void,
            ) -> i32;
            pub fn RegisterEventHotKey(
                hot_key_code: u32,
                hot_key_modifiers: u32,
                hot_key_id: EventHotKeyID,
                target: *mut c_void,
                options: u32,
                out_ref: *mut *mut c_void,
            ) -> i32;
            pub fn UnregisterEventHotKey(hot_key_ref: *mut c_void) -> i32;
            pub fn GetEventParameter(
                event: *mut c_void,
                name: u32,
                desired_type: u32,
                actual_type: *mut u32,
                buffer_size: u32,
                actual_size: *mut u32,
                data: *mut c_void,
            ) -> i32;
        }
    }

    // Carbon virtual key codes (from HIToolbox/Events.h)
    const KEY_SPACE: u32 = 49;
    const KEY_V: u32 = 9;

    // Hotkey identifiers for our registered hotkeys
    const HOTKEY_ID_LAUNCHER: u32 = 1;
    const HOTKEY_ID_CLIPBOARD: u32 = 2;
    const HOTKEY_SIGNATURE: u32 = u32::from_be_bytes(*b"PC01");

    /// Callback type for hotkey activation
    pub type HotkeyCallback = Box<dyn Fn() + Send + Sync>;

    /// Wrapper for EventHotKeyRef to allow storage in Mutex across threads.
    /// SAFETY: The pointer is only accessed on the main thread for registration/unregistration.
    struct HotKeyRefWrapper(*mut std::ffi::c_void);
    unsafe impl Send for HotKeyRefWrapper {}

    /// Global state for the hotkey system
    static HOTKEY_ACTIVE: AtomicBool = AtomicBool::new(false);
    static HOTKEY_CALLBACK: Mutex<Option<Arc<HotkeyCallback>>> = Mutex::new(None);
    /// Callback for clipboard hotkey (Cmd+Shift+V)
    static CLIPBOARD_HOTKEY_CALLBACK: Mutex<Option<Arc<HotkeyCallback>>> = Mutex::new(None);
    /// Stored hotkey refs for proper unregistration
    static LAUNCHER_HOTKEY_REF: Mutex<Option<HotKeyRefWrapper>> = Mutex::new(None);
    static CLIPBOARD_HOTKEY_REF: Mutex<Option<HotKeyRefWrapper>> = Mutex::new(None);

    /// Carbon event handler callback — dispatches hotkey events to registered callbacks.
    /// Uses dispatch queue instead of spawning threads to avoid unbounded thread creation.
    pub(super) fn dispatch_hotkey_callback(callback: Arc<HotkeyCallback>) {
        use dispatch::Queue;
        Queue::main().exec_async(move || callback());
    }

    unsafe extern "C" fn hotkey_event_handler(
        _call_ref: carbon_hotkey::EventHandlerCallRef,
        event: carbon_hotkey::EventRef,
        _user_data: *mut std::ffi::c_void,
    ) -> i32 {
        use carbon_hotkey::*;

        let mut hotkey_id = EventHotKeyID {
            signature: 0,
            id: 0,
        };

        let status = GetEventParameter(
            event,
            EVENT_PARAM_DIRECT_OBJECT,
            TYPE_EVENT_HOT_KEY_ID,
            std::ptr::null_mut(),
            std::mem::size_of::<EventHotKeyID>() as u32,
            std::ptr::null_mut(),
            &mut hotkey_id as *mut _ as *mut std::ffi::c_void,
        );

        if status != NO_ERR {
            return status;
        }

        match hotkey_id.id {
            HOTKEY_ID_LAUNCHER => {
                debug!("Carbon hotkey: Cmd+Space");
                if let Ok(cb) = HOTKEY_CALLBACK.lock() {
                    if let Some(callback) = cb.as_ref() {
                        dispatch_hotkey_callback(Arc::clone(callback));
                    }
                }
            },
            HOTKEY_ID_CLIPBOARD => {
                debug!("Carbon hotkey: Cmd+Shift+V");
                if let Ok(cb) = CLIPBOARD_HOTKEY_CALLBACK.lock() {
                    if let Some(callback) = cb.as_ref() {
                        dispatch_hotkey_callback(Arc::clone(callback));
                    }
                }
            },
            _ => {},
        }

        NO_ERR
    }

    /// Registers a global hotkey (Cmd+Space) via Carbon RegisterEventHotKey.
    ///
    /// This API does not require accessibility permissions and works reliably
    /// across macOS versions. The callback will be invoked on a background thread.
    pub fn register_global_hotkey<F>(callback: F) -> Result<(), String>
    where
        F: Fn() + Send + Sync + 'static,
    {
        if HOTKEY_ACTIVE.load(Ordering::SeqCst) {
            warn!("Global hotkey already registered");
            return Ok(());
        }

        info!("Registering global hotkey (Cmd+Space) via Carbon");

        {
            let mut cb = HOTKEY_CALLBACK.lock().map_err(|e| e.to_string())?;
            *cb = Some(Arc::new(Box::new(callback)));
        }

        unsafe {
            use carbon_hotkey::*;

            let target = GetApplicationEventTarget();

            let event_spec = EventTypeSpec {
                event_class: EVENT_CLASS_KEYBOARD,
                event_kind: EVENT_HOT_KEY_PRESSED,
            };

            let status = InstallEventHandler(
                target,
                hotkey_event_handler,
                1,
                &event_spec,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            );

            if status != NO_ERR {
                return Err(format!("InstallEventHandler failed with status {status}"));
            }

            let hotkey_id = EventHotKeyID {
                signature: HOTKEY_SIGNATURE,
                id: HOTKEY_ID_LAUNCHER,
            };

            let mut hotkey_ref: *mut std::ffi::c_void = std::ptr::null_mut();
            let status = RegisterEventHotKey(
                KEY_SPACE,
                CMD_KEY_MASK,
                hotkey_id,
                target,
                0,
                &mut hotkey_ref,
            );

            if status != NO_ERR {
                return Err(format!(
                    "RegisterEventHotKey failed for Cmd+Space (status {status}). \
                     Cmd+Space may be in use by Spotlight. \
                     Go to System Settings > Keyboard > Keyboard Shortcuts > Spotlight \
                     and disable or change the Spotlight shortcut."
                ));
            }

            // Store the ref for later unregistration
            if let Ok(mut ref_guard) = LAUNCHER_HOTKEY_REF.lock() {
                *ref_guard = Some(HotKeyRefWrapper(hotkey_ref));
            }

            info!("Cmd+Space hotkey registered successfully via Carbon");
        }

        HOTKEY_ACTIVE.store(true, Ordering::SeqCst);
        Ok(())
    }

    /// Unregisters all global hotkeys and clears callbacks.
    pub fn unregister_global_hotkey() {
        info!("Unregistering global hotkeys");

        // Unregister launcher hotkey
        if let Ok(mut ref_guard) = LAUNCHER_HOTKEY_REF.lock() {
            if let Some(HotKeyRefWrapper(hotkey_ref)) = ref_guard.take() {
                if !hotkey_ref.is_null() {
                    unsafe {
                        carbon_hotkey::UnregisterEventHotKey(hotkey_ref);
                    }
                }
            }
        }

        // Unregister clipboard hotkey
        if let Ok(mut ref_guard) = CLIPBOARD_HOTKEY_REF.lock() {
            if let Some(HotKeyRefWrapper(hotkey_ref)) = ref_guard.take() {
                if !hotkey_ref.is_null() {
                    unsafe {
                        carbon_hotkey::UnregisterEventHotKey(hotkey_ref);
                    }
                }
            }
        }

        HOTKEY_ACTIVE.store(false, Ordering::SeqCst);
        if let Ok(mut cb) = HOTKEY_CALLBACK.lock() {
            *cb = None;
        }
        if let Ok(mut cb) = CLIPBOARD_HOTKEY_CALLBACK.lock() {
            *cb = None;
        }
    }

    /// Registers the clipboard hotkey (Cmd+Shift+V) via Carbon RegisterEventHotKey.
    /// Must be called after `register_global_hotkey` (which installs the event handler).
    pub fn register_clipboard_hotkey<F>(callback: F) -> Result<(), String>
    where
        F: Fn() + Send + Sync + 'static,
    {
        // Check that the event handler has been installed
        if !HOTKEY_ACTIVE.load(Ordering::SeqCst) {
            return Err(
                "register_clipboard_hotkey must be called after register_global_hotkey".to_string(),
            );
        }

        info!("Registering clipboard hotkey (Cmd+Shift+V) via Carbon");

        {
            let mut cb = CLIPBOARD_HOTKEY_CALLBACK
                .lock()
                .map_err(|e| e.to_string())?;
            *cb = Some(Arc::new(Box::new(callback)));
        }

        unsafe {
            use carbon_hotkey::*;

            let target = GetApplicationEventTarget();

            let hotkey_id = EventHotKeyID {
                signature: HOTKEY_SIGNATURE,
                id: HOTKEY_ID_CLIPBOARD,
            };

            let mut hotkey_ref: *mut std::ffi::c_void = std::ptr::null_mut();
            let status = RegisterEventHotKey(
                KEY_V,
                CMD_KEY_MASK | SHIFT_KEY_MASK,
                hotkey_id,
                target,
                0,
                &mut hotkey_ref,
            );

            if status != NO_ERR {
                return Err(format!(
                    "RegisterEventHotKey failed for Cmd+Shift+V (status {status})"
                ));
            }

            // Store the ref for later unregistration
            if let Ok(mut ref_guard) = CLIPBOARD_HOTKEY_REF.lock() {
                *ref_guard = Some(HotKeyRefWrapper(hotkey_ref));
            }

            info!("Cmd+Shift+V hotkey registered successfully via Carbon");
        }

        Ok(())
    }

    /// Returns whether a global hotkey is currently registered.
    #[allow(dead_code)]
    pub fn is_hotkey_registered() -> bool {
        HOTKEY_ACTIVE.load(Ordering::SeqCst)
    }

    // =========================================================================
    // Menu Bar (Status Item) Integration
    // =========================================================================

    // Thread-local storage for the status item (NSStatusItem is MainThreadOnly)
    thread_local! {
        static MENU_BAR_STATUS_ITEM: RefCell<Option<Retained<NSStatusItem>>> = const { RefCell::new(None) };
        static MENU_BAR_TARGET: RefCell<Option<Retained<MenuBarTarget>>> = const { RefCell::new(None) };
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
        /// Check for updates
        CheckForUpdates,
        /// Show about dialog
        About,
        /// Quit the application
        Quit,
    }

    /// Tracks the last click type to distinguish left vs right clicks
    static LAST_CLICK_WAS_RIGHT: AtomicBool = AtomicBool::new(false);

    /// Global callback for menu bar actions
    static MENU_BAR_CALLBACK: Mutex<Option<Arc<MenuBarCallback>>> = Mutex::new(None);

    define_class!(
        // SAFETY: NSObject has no subclassing requirements and we don't implement Drop.
        #[unsafe(super(NSObject))]
        #[thread_kind = MainThreadOnly]
        #[name = "PhotonCastMenuBarTarget"]
        struct MenuBarTarget;

        impl MenuBarTarget {
            /// Called when a menu item is selected from the context menu
            #[unsafe(method(menuBarItemSelected:))]
            fn menu_bar_item_selected(&self, item: &NSMenuItem) {
                let tag = item.tag();
                let action = match tag {
                    1 => Some(MenuBarActionKind::ToggleLauncher),
                    2 => Some(MenuBarActionKind::OpenPreferences),
                    3 => Some(MenuBarActionKind::CheckForUpdates),
                    4 => Some(MenuBarActionKind::About),
                    5 => Some(MenuBarActionKind::Quit),
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

            /// Called when the menu bar button is clicked (left click)
            /// We determine if it was a left or right click based on LAST_CLICK_WAS_RIGHT
            #[unsafe(method(menuBarButtonClicked:))]
            fn menu_bar_button_clicked(&self, _sender: &NSButton) {
                // Check if this was a right-click (handled separately via sendActionOn:)
                let was_right_click = LAST_CLICK_WAS_RIGHT.swap(false, Ordering::SeqCst);

                if was_right_click {
                    // Right click: the menu will be shown automatically by NSStatusItem
                    // We don't need to do anything here as NSStatusItem handles the menu
                    debug!("Menu bar right-click detected, showing context menu");
                } else {
                    // Left click: toggle the launcher
                    debug!("Menu bar left-click detected, toggling launcher");
                    if let Ok(cb_guard) = MENU_BAR_CALLBACK.lock() {
                        if let Some(callback) = cb_guard.as_ref() {
                            callback(MenuBarActionKind::ToggleLauncher);
                        }
                    }
                }
            }
        }

        unsafe impl NSObjectProtocol for MenuBarTarget {}
    );

    /// Loads the menu bar icon from the app bundle and sets it on the button.
    /// Returns true if the icon was loaded successfully.
    fn load_menu_bar_icon(button: &NSButton) -> bool {
        use objc2_foundation::NSData;

        // Try to find the icon in the app bundle
        let exe_path = std::env::current_exe();

        let icon_paths = [
            // When running from app bundle
            exe_path
                .ok()
                .and_then(|p| p.parent().map(|p| p.to_path_buf()))
                .map(|p| p.join("../Resources/AppIcon.icns")),
            // Development fallback
            Some(std::path::PathBuf::from("resources/AppIcon.icns")),
        ];

        for maybe_path in icon_paths.iter().flatten() {
            if maybe_path.exists() {
                debug!("Loading menu bar icon from: {}", maybe_path.display());

                // Read the icon file
                if let Ok(icon_data) = std::fs::read(maybe_path) {
                    // Create NSData from the bytes
                    let ns_data = NSData::with_bytes(&icon_data);

                    // Create NSImage from the data
                    let image = NSImage::initWithData(NSImage::alloc(), &ns_data);

                    if let Some(image) = image {
                        // Set size appropriate for menu bar (18x18 is standard)
                        let size = NSSize::new(18.0, 18.0);
                        image.setSize(size);

                        // Don't use template mode - our icon has colors
                        // Template mode converts to grayscale based on alpha
                        image.setTemplate(false);

                        // Set the image on the button
                        button.setImage(Some(&image));

                        info!("Menu bar icon loaded successfully");
                        return true;
                    }
                }
            }
        }

        warn!("Could not load menu bar icon from any known path");
        false
    }

    /// Creates and shows the menu bar status item with click handling.
    ///
    /// Left-click toggles the launcher window.
    /// Right-click shows the context menu.
    ///
    /// This function must be called from the main thread (which GPUI ensures).
    ///
    /// # Arguments
    /// * `callback` - Function called when menu items are selected or launcher is toggled
    ///
    /// # Returns
    /// `Ok(())` on success, or an error message on failure.
    pub fn create_menu_bar_item<F>(callback: F) -> Result<(), String>
    where
        F: Fn(MenuBarActionKind) + Send + Sync + 'static,
    {
        info!("Creating menu bar status item with click handlers");

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

        // Create the target for menu actions
        let target = mtm.alloc::<MenuBarTarget>().set_ivars(());
        #[allow(deprecated)]
        let target: Retained<MenuBarTarget> = unsafe { msg_send_id![super(target), init] };

        // Set up the button with title and click handling
        #[allow(deprecated)]
        let button: Option<Retained<NSButton>> = unsafe { msg_send_id![&status_item, button] };

        if let Some(button) = button {
            // Try to load the menu bar icon from the app bundle
            let icon_loaded = load_menu_bar_icon(&button);

            if !icon_loaded {
                // Fallback to text if icon loading fails
                let title = NSString::from_str("PC");
                button.setTitle(&title);
                debug!("Using text fallback for menu bar icon");
            }

            // Set up the target for click handling
            unsafe {
                button.setTarget(Some(target.as_ref()));
                // Use sendActionOn: to capture both left and right mouse clicks
                // NSEventMaskLeftMouseDown | NSEventMaskRightMouseDown = 1 << 1 | 1 << 3 = 2 | 8 = 10
                // However, we need to detect which click it was, so we'll use a different approach
                button.setAction(Some(sel!(menuBarButtonClicked:)));
            }
        } else {
            warn!("Could not get status item button to set title");
        }

        // Create the context menu for right-click
        let menu = create_context_menu(mtm, &target);

        // Set the menu on the status item (this enables right-click menu)
        status_item.setMenu(Some(&menu));

        // Store the status item and target in thread-local storage to keep them alive
        MENU_BAR_TARGET.with(|cell| {
            *cell.borrow_mut() = Some(target);
        });

        MENU_BAR_STATUS_ITEM.with(|cell| {
            *cell.borrow_mut() = Some(status_item);
        });

        info!("Menu bar status item created successfully with click handlers");
        Ok(())
    }

    /// Creates the context menu for the status item (shown on right-click).
    fn create_context_menu(mtm: MainThreadMarker, target: &MenuBarTarget) -> Retained<NSMenu> {
        let menu = NSMenu::new(mtm);

        // "Open PhotonCast" item (shows ⌘Space shortcut hint)
        let open_title = NSString::from_str("Open PhotonCast");
        let open_key = NSString::from_str(" "); // Space key
        let open_item = unsafe {
            NSMenuItem::initWithTitle_action_keyEquivalent(
                mtm.alloc::<NSMenuItem>(),
                &open_title,
                Some(sel!(menuBarItemSelected:)),
                &open_key,
            )
        };
        open_item.setTag(1);
        unsafe {
            open_item.setTarget(Some(target));
        }
        // Keep enabled to show it's clickable, even though shortcut hint is shown
        open_item.setEnabled(true);
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
                Some(sel!(menuBarItemSelected:)),
                &prefs_key,
            )
        };
        prefs_item.setTag(2);
        unsafe {
            prefs_item.setTarget(Some(target));
        }
        menu.addItem(&prefs_item);

        // "Check for Updates" item
        let updates_title = NSString::from_str("Check for Updates");
        let updates_item = unsafe {
            NSMenuItem::initWithTitle_action_keyEquivalent(
                mtm.alloc::<NSMenuItem>(),
                &updates_title,
                Some(sel!(menuBarItemSelected:)),
                &NSString::from_str(""),
            )
        };
        updates_item.setTag(3);
        unsafe {
            updates_item.setTarget(Some(target));
        }
        menu.addItem(&updates_item);

        // Separator
        let separator2 = NSMenuItem::separatorItem(mtm);
        menu.addItem(&separator2);

        // "About PhotonCast" item
        let about_title = NSString::from_str("About PhotonCast");
        let about_item = unsafe {
            NSMenuItem::initWithTitle_action_keyEquivalent(
                mtm.alloc::<NSMenuItem>(),
                &about_title,
                Some(sel!(menuBarItemSelected:)),
                &NSString::from_str(""),
            )
        };
        about_item.setTag(4);
        unsafe {
            about_item.setTarget(Some(target));
        }
        menu.addItem(&about_item);

        // Separator
        let separator3 = NSMenuItem::separatorItem(mtm);
        menu.addItem(&separator3);

        // "Quit PhotonCast" item
        let quit_title = NSString::from_str("Quit PhotonCast");
        let quit_key = NSString::from_str("q");
        let quit_item = unsafe {
            NSMenuItem::initWithTitle_action_keyEquivalent(
                mtm.alloc::<NSMenuItem>(),
                &quit_title,
                Some(sel!(menuBarItemSelected:)),
                &quit_key,
            )
        };
        quit_item.setTag(5);
        unsafe {
            quit_item.setTarget(Some(target));
        }
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
pub fn sync_activation_policy(_show_in_dock: bool) -> Result<(), String> {
    Ok(())
}

#[cfg(not(target_os = "macos"))]
pub fn activate_ignoring_other_apps() -> Result<(), String> {
    Ok(())
}

#[cfg(not(target_os = "macos"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuBarActionKind {
    ToggleLauncher,
    OpenPreferences,
    CheckForUpdates,
    About,
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
    #[cfg(target_os = "macos")]
    use std::sync::{mpsc, Arc};
    #[cfg(target_os = "macos")]
    use std::time::{Duration, Instant};

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

    #[cfg(target_os = "macos")]
    fn wait_for_hotkey_callback(
        rx: &mpsc::Receiver<Instant>,
        timeout: Duration,
    ) -> Result<Instant, mpsc::RecvTimeoutError> {
        use core_foundation::runloop::{kCFRunLoopDefaultMode, CFRunLoop};

        let deadline = Instant::now() + timeout;
        loop {
            match rx.try_recv() {
                Ok(instant) => return Ok(instant),
                Err(mpsc::TryRecvError::Empty) => {
                    if Instant::now() >= deadline {
                        return Err(mpsc::RecvTimeoutError::Timeout);
                    }
                    let _ = unsafe {
                        CFRunLoop::run_in_mode(
                            kCFRunLoopDefaultMode,
                            Duration::from_millis(10),
                            false,
                        )
                    };
                },
                Err(mpsc::TryRecvError::Disconnected) => {
                    return Err(mpsc::RecvTimeoutError::Disconnected);
                },
            }
        }
    }

    #[test]
    #[ignore = "requires a macOS app-shell/main-run-loop environment; command-line unit tests do not service the hotkey callback path reliably"]
    #[cfg(target_os = "macos")]
    fn test_hotkey_callback_dispatch_latency_snapshot() {
        let (tx, rx) = mpsc::channel();
        let callback: Arc<HotkeyCallback> = Arc::new(Box::new(move || {
            let _ = tx.send(Instant::now());
        }));

        let start = Instant::now();
        super::macos::dispatch_hotkey_callback(callback);

        let fired_at = wait_for_hotkey_callback(&rx, Duration::from_millis(250))
            .expect("main-queue hotkey callback should fire promptly");
        let elapsed = fired_at.duration_since(start);

        eprintln!(
            "Hotkey async callback dispatch snapshot: {:?} (target {:?})",
            elapsed,
            Duration::from_millis(50)
        );
        assert!(elapsed > Duration::ZERO);
    }

    #[test]
    #[ignore = "requires a macOS app-shell/main-run-loop environment and a representative machine"]
    #[cfg(target_os = "macos")]
    fn test_hotkey_callback_dispatch_under_50ms_strict() {
        let (tx, rx) = mpsc::channel();
        let callback: Arc<HotkeyCallback> = Arc::new(Box::new(move || {
            let _ = tx.send(Instant::now());
        }));

        let start = Instant::now();
        super::macos::dispatch_hotkey_callback(callback);

        let fired_at = wait_for_hotkey_callback(&rx, Duration::from_millis(250))
            .expect("main-queue hotkey callback should fire promptly");
        let elapsed = fired_at.duration_since(start);

        assert!(
            elapsed <= Duration::from_millis(50),
            "Hotkey callback dispatch took {:?}, target is 50ms",
            elapsed
        );
    }
}
