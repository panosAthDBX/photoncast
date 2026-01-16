# macOS Platform Integration

## Overview

PhotonCast deeply integrates with macOS for application launching, global hotkeys, accessibility, and system services. This document covers platform-specific patterns.

## When to Apply

- Implementing macOS-specific features
- Working with Objective-C APIs from Rust
- Handling system permissions
- Integrating with Spotlight, Services, etc.

## Core Principles

1. **Use safe abstractions** - Prefer `objc2` over raw FFI
2. **Handle permissions gracefully** - Request and explain permissions
3. **Respect system conventions** - Follow macOS HIG
4. **Graceful degradation** - Work even without optional permissions

## ✅ DO

### DO: Use objc2 for Safe Objective-C Interop

**✅ DO**:
```rust
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2_app_kit::{NSRunningApplication, NSWorkspace};
use objc2_foundation::{NSArray, NSString, NSURL};

pub fn get_running_applications() -> Vec<RunningApp> {
    unsafe {
        let workspace = NSWorkspace::sharedWorkspace();
        let apps = workspace.runningApplications();
        
        apps.iter()
            .filter_map(|app| {
                let name = app.localizedName()?.to_string();
                let bundle_id = app.bundleIdentifier()?.to_string();
                let bundle_url = app.bundleURL()?;
                let path = bundle_url.path()?.to_string();
                
                Some(RunningApp {
                    name,
                    bundle_id,
                    path: PathBuf::from(path),
                })
            })
            .collect()
    }
}
```

### DO: Register Global Hotkeys Properly

**✅ DO**:
```rust
use core_foundation::runloop::{CFRunLoop, kCFRunLoopDefaultMode};
use core_graphics::event::{CGEventTap, CGEventTapLocation, CGEventType};

pub struct HotkeyManager {
    registered_hotkeys: Vec<RegisteredHotkey>,
}

impl HotkeyManager {
    pub fn register(
        &mut self,
        key: Key,
        modifiers: Modifiers,
        callback: impl Fn() + Send + 'static,
    ) -> Result<HotkeyId> {
        // Use Carbon Events or CGEventTap for global hotkeys
        let hotkey_id = unsafe {
            let event_tap = CGEventTap::new(
                CGEventTapLocation::Session,
                CGEventTapPlacement::HeadInsert,
                CGEventTapOptions::Default,
                vec![CGEventType::KeyDown],
                |_, event_type, event| {
                    // Check if this matches our hotkey
                    if matches_hotkey(event, key, modifiers) {
                        callback();
                        None  // Consume the event
                    } else {
                        Some(event)
                    }
                },
            )?;
            
            // Enable and add to run loop
            event_tap.enable();
            CFRunLoop::get_current().add_source(
                &event_tap.as_source(),
                kCFRunLoopDefaultMode,
            );
            
            self.registered_hotkeys.push(RegisteredHotkey {
                id: HotkeyId(self.registered_hotkeys.len()),
                tap: event_tap,
            });
            
            HotkeyId(self.registered_hotkeys.len() - 1)
        };
        
        Ok(hotkey_id)
    }
}
```

### DO: Request Permissions Gracefully

**✅ DO**:
```rust
use objc2_app_kit::NSWorkspace;

pub enum PermissionStatus {
    Granted,
    Denied,
    NotDetermined,
}

pub fn check_accessibility_permission() -> PermissionStatus {
    let trusted = unsafe {
        // AXIsProcessTrusted()
        let options = std::ptr::null();  // or with prompt
        AXIsProcessTrustedWithOptions(options)
    };
    
    if trusted {
        PermissionStatus::Granted
    } else {
        PermissionStatus::Denied
    }
}

pub fn request_accessibility_permission() {
    unsafe {
        let options = CFDictionaryCreate(
            std::ptr::null(),
            &[kAXTrustedCheckOptionPrompt as *const _] as *const _,
            &[kCFBooleanTrue as *const _] as *const _,
            1,
            &kCFTypeDictionaryKeyCallBacks,
            &kCFTypeDictionaryValueCallBacks,
        );
        
        AXIsProcessTrustedWithOptions(options);
        CFRelease(options as *const _);
    }
}

// Show user-friendly message
pub fn show_permission_dialog(permission: &str) -> bool {
    let message = match permission {
        "accessibility" => {
            "PhotonCast needs accessibility access to:\n\
             • Register global keyboard shortcuts\n\
             • Read window information\n\n\
             Click 'Open Settings' to grant access."
        }
        "automation" => {
            "PhotonCast needs automation access to:\n\
             • Launch applications\n\
             • Control system features"
        }
        _ => return false,
    };
    
    // Show alert with options
    show_alert("Permission Required", message, &["Open Settings", "Cancel"])
        .map(|choice| {
            if choice == 0 {
                open_system_preferences(permission);
                true
            } else {
                false
            }
        })
        .unwrap_or(false)
}
```

### DO: Use Application Services for Launching

**✅ DO**:
```rust
use objc2_app_kit::NSWorkspace;
use objc2_foundation::NSURL;

pub async fn launch_application(bundle_id: &str) -> Result<()> {
    let result = tokio::task::spawn_blocking({
        let bundle_id = bundle_id.to_string();
        move || unsafe {
            let workspace = NSWorkspace::sharedWorkspace();
            let bundle_id = NSString::from_str(&bundle_id);
            
            let url = workspace.URLForApplicationWithBundleIdentifier(&bundle_id)
                .ok_or_else(|| LaunchError::AppNotFound)?;
            
            let config = NSWorkspaceOpenConfiguration::new();
            config.setActivates(true);
            
            // Async launch with completion handler
            let (tx, rx) = std::sync::mpsc::channel();
            
            workspace.openURL_configuration_completionHandler(
                &url,
                &config,
                Some(&|app, error| {
                    if let Some(err) = error {
                        tx.send(Err(LaunchError::Failed(err.to_string()))).ok();
                    } else {
                        tx.send(Ok(())).ok();
                    }
                }),
            );
            
            rx.recv().map_err(|_| LaunchError::Timeout)?
        }
    }).await?;
    
    result
}
```

### DO: Index Applications Properly

**✅ DO**:
```rust
use std::path::PathBuf;
use plist::Value;

pub struct ApplicationInfo {
    pub name: String,
    pub bundle_id: String,
    pub path: PathBuf,
    pub icon_path: Option<PathBuf>,
    pub keywords: Vec<String>,
    pub category: Option<String>,
}

pub async fn index_applications() -> Result<Vec<ApplicationInfo>> {
    let search_paths = vec![
        PathBuf::from("/Applications"),
        PathBuf::from("/System/Applications"),
        dirs::home_dir()
            .map(|h| h.join("Applications"))
            .unwrap_or_default(),
    ];
    
    let mut apps = Vec::new();
    
    for search_path in search_paths {
        if !search_path.exists() {
            continue;
        }
        
        let mut entries = tokio::fs::read_dir(&search_path).await?;
        
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            
            if path.extension().map(|e| e == "app").unwrap_or(false) {
                if let Some(app_info) = parse_app_bundle(&path).await {
                    apps.push(app_info);
                }
            }
        }
    }
    
    Ok(apps)
}

async fn parse_app_bundle(path: &Path) -> Option<ApplicationInfo> {
    let info_plist_path = path.join("Contents/Info.plist");
    let contents = tokio::fs::read(&info_plist_path).await.ok()?;
    let plist: Value = plist::from_bytes(&contents).ok()?;
    let dict = plist.as_dictionary()?;
    
    let name = dict.get("CFBundleName")
        .or_else(|| dict.get("CFBundleDisplayName"))
        .and_then(|v| v.as_string())
        .map(|s| s.to_string())
        .or_else(|| {
            path.file_stem()
                .and_then(|s| s.to_str())
                .map(|s| s.trim_end_matches(".app").to_string())
        })?;
    
    let bundle_id = dict.get("CFBundleIdentifier")
        .and_then(|v| v.as_string())
        .map(|s| s.to_string())?;
    
    let icon_file = dict.get("CFBundleIconFile")
        .and_then(|v| v.as_string())
        .map(|s| {
            let icon_name = if s.ends_with(".icns") { s.to_string() } else { format!("{}.icns", s) };
            path.join("Contents/Resources").join(icon_name)
        });
    
    Some(ApplicationInfo {
        name,
        bundle_id,
        path: path.to_path_buf(),
        icon_path: icon_file,
        keywords: Vec::new(),
        category: dict.get("LSApplicationCategoryType")
            .and_then(|v| v.as_string())
            .map(|s| s.to_string()),
    })
}
```

## ❌ DON'T

### DON'T: Use Hardcoded Paths

**❌ DON'T**:
```rust
let config_path = "/Users/someuser/.config/photoncast/config.toml";
let cache_path = "/Users/someuser/Library/Caches/photoncast";
```

**✅ DO**:
```rust
use directories::ProjectDirs;

pub fn get_project_dirs() -> Option<ProjectDirs> {
    ProjectDirs::from("", "PhotonCast", "PhotonCast")
}

pub fn config_path() -> PathBuf {
    get_project_dirs()
        .map(|d| d.config_dir().to_path_buf())
        .unwrap_or_else(|| {
            dirs::home_dir()
                .unwrap_or_default()
                .join(".config/photoncast")
        })
}

pub fn cache_path() -> PathBuf {
    get_project_dirs()
        .map(|d| d.cache_dir().to_path_buf())
        .unwrap_or_else(|| {
            dirs::cache_dir()
                .unwrap_or_default()
                .join("photoncast")
        })
}
```

### DON'T: Ignore Error Handling in FFI

**❌ DON'T**:
```rust
unsafe fn launch_app(bundle_id: &str) {
    let workspace = NSWorkspace::sharedWorkspace();
    // No error checking!
    workspace.launchApplication(&NSString::from_str(bundle_id));
}
```

**✅ DO**:
```rust
pub fn launch_app(bundle_id: &str) -> Result<(), LaunchError> {
    unsafe {
        let workspace = NSWorkspace::sharedWorkspace();
        let ns_bundle_id = NSString::from_str(bundle_id);
        
        // Check if app exists first
        let url = workspace.URLForApplicationWithBundleIdentifier(&ns_bundle_id)
            .ok_or(LaunchError::NotFound(bundle_id.to_string()))?;
        
        // Launch with error handling
        let success = workspace.openURL(&url);
        
        if success {
            Ok(())
        } else {
            Err(LaunchError::Failed(format!("Failed to launch {}", bundle_id)))
        }
    }
}
```

### DON'T: Poll for Events

**❌ DON'T**:
```rust
loop {
    // DON'T: Polling wastes CPU
    if check_for_hotkey() {
        handle_hotkey();
    }
    std::thread::sleep(Duration::from_millis(10));
}
```

**✅ DO**:
```rust
// DO: Use event-driven approach with run loop
fn setup_event_handler() {
    let event_tap = create_event_tap(|event| {
        // Called only when events occur
        handle_event(event);
    });
    
    // Add to run loop - no polling needed
    CFRunLoop::get_current().add_source(&event_tap, kCFRunLoopDefaultMode);
    CFRunLoop::run();  // Blocks until stopped, uses no CPU when idle
}
```

### DON'T: Assume Permissions

**❌ DON'T**:
```rust
fn register_global_hotkey() {
    // Assumes accessibility is granted - will silently fail!
    unsafe {
        CGEventTapCreate(...);
    }
}
```

**✅ DO**:
```rust
pub fn register_global_hotkey() -> Result<(), HotkeyError> {
    // Check permission first
    if !check_accessibility_permission() {
        return Err(HotkeyError::PermissionDenied {
            message: "Accessibility permission required for global hotkeys".into(),
            can_request: true,
        });
    }
    
    // Now safe to register
    unsafe {
        let tap = CGEventTapCreate(...);
        if tap.is_null() {
            return Err(HotkeyError::RegistrationFailed);
        }
        // ...
    }
    
    Ok(())
}
```

## Common Crates for macOS

| Task | Crate | Notes |
|------|-------|-------|
| Objective-C interop | `objc2`, `objc2-foundation`, `objc2-app-kit` | Safe bindings |
| Core Foundation | `core-foundation` | CF types |
| Core Graphics | `core-graphics` | CGEvent, etc. |
| Security | `security-framework` | Keychain access |
| Global hotkeys | `global-hotkey` | Cross-platform |
| File watching | `notify` | FSEvents backend |
| System info | `sysinfo` | Process info |
| App directories | `directories` | Standard paths |
| Plist parsing | `plist` | Info.plist |
| Icon loading | `icns` | .icns files |

## Resources

- [objc2 Documentation](https://docs.rs/objc2/)
- [Apple Developer: App Services](https://developer.apple.com/documentation/appkit/app_and_environment)
- [Apple HIG](https://developer.apple.com/design/human-interface-guidelines/)
- [Accessibility API](https://developer.apple.com/documentation/applicationservices/axuielement)
