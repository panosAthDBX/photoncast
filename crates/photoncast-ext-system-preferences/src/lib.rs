#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::must_use_candidate)]

//! System Preferences Extension for PhotonCast
//!
//! Provides quick access to macOS System Settings panes.

use abi_stable::prefix_type::PrefixTypeTrait;
use abi_stable::sabi_trait::prelude::TD_Opaque;
use abi_stable::std_types::{RBox, ROption, RResult, RString, RVec};
use photoncast_extension_api::prelude::*;
use photoncast_extension_api::{
    CommandHandlerTrait, ExtensionApiResult, ExtensionManifest, ExtensionSearchProvider_TO,
    Extension_TO,
};

/// A system settings pane
#[derive(Debug, Clone)]
struct SettingsPane {
    id: &'static str,
    name: &'static str,
    icon: &'static str,
    url: &'static str,
    keywords: &'static [&'static str],
}

impl SettingsPane {
    /// Creates a list item for this pane
    fn to_list_item(&self) -> ListItem {
        let mut actions = RVec::new();

        // Open action (primary)
        actions.push(Action {
            id: RString::from("open"),
            title: RString::from("Open"),
            icon: ROption::RSome(IconSource::SystemIcon {
                name: RString::from("arrow.right.circle"),
            }),
            shortcut: ROption::RSome(Shortcut::cmd("o")),
            style: ActionStyle::Primary,
            handler: ActionHandler::OpenUrl(RString::from(self.url)),
        });

        // Copy URL action
        actions.push(Action {
            id: RString::from("copy-url"),
            title: RString::from("Copy Deep Link"),
            icon: ROption::RSome(IconSource::SystemIcon {
                name: RString::from("doc.on.doc"),
            }),
            shortcut: ROption::RSome(Shortcut::cmd_shift("c")),
            style: ActionStyle::Default,
            handler: ActionHandler::CopyToClipboard(RString::from(self.url)),
        });

        ListItem {
            id: RString::from(self.id),
            title: RString::from(self.name),
            subtitle: ROption::RNone,
            icon: IconSource::SystemIcon {
                name: RString::from(self.icon),
            },
            accessories: RVec::new(),
            actions,
            preview: ROption::RNone,
            shortcut: ROption::RNone,
        }
    }

    /// Checks if this pane matches the query
    fn matches(&self, query: &str) -> bool {
        let query_lower = query.to_lowercase();
        self.name.to_lowercase().contains(&query_lower)
            || self.keywords.iter().any(|k| k.contains(&query_lower))
    }
}

/// All available settings panes
const SETTINGS_PANES: &[SettingsPane] = &[
    SettingsPane {
        id: "wifi",
        name: "Wi-Fi",
        icon: "wifi",
        url: "x-apple.systempreferences:com.apple.wifi-settings-extension",
        keywords: &["network", "wireless", "internet"],
    },
    SettingsPane {
        id: "bluetooth",
        name: "Bluetooth",
        icon: "bluetooth",
        url: "x-apple.systempreferences:com.apple.BluetoothSettings",
        keywords: &["wireless", "devices", "pair"],
    },
    SettingsPane {
        id: "network",
        name: "Network",
        icon: "network",
        url: "x-apple.systempreferences:com.apple.Network-Settings.extension",
        keywords: &["internet", "ethernet", "vpn", "dns"],
    },
    SettingsPane {
        id: "sound",
        name: "Sound",
        icon: "speaker.wave.3",
        url: "x-apple.systempreferences:com.apple.Sound-Settings.extension",
        keywords: &["audio", "volume", "output", "input", "microphone"],
    },
    SettingsPane {
        id: "displays",
        name: "Displays",
        icon: "display",
        url: "x-apple.systempreferences:com.apple.Displays-Settings.extension",
        keywords: &["monitor", "screen", "resolution", "brightness"],
    },
    SettingsPane {
        id: "appearance",
        name: "Appearance",
        icon: "paintbrush",
        url: "x-apple.systempreferences:com.apple.Appearance-Settings.extension",
        keywords: &["dark mode", "light mode", "theme", "accent color"],
    },
    SettingsPane {
        id: "notifications",
        name: "Notifications",
        icon: "bell",
        url: "x-apple.systempreferences:com.apple.Notifications-Settings.extension",
        keywords: &["alerts", "banners", "do not disturb", "focus"],
    },
    SettingsPane {
        id: "focus",
        name: "Focus",
        icon: "moon",
        url: "x-apple.systempreferences:com.apple.Focus-Settings.extension",
        keywords: &["do not disturb", "dnd", "notifications"],
    },
    SettingsPane {
        id: "privacy",
        name: "Privacy & Security",
        icon: "lock.shield",
        url: "x-apple.systempreferences:com.apple.settings.PrivacySecurity.extension",
        keywords: &[
            "permissions",
            "location",
            "camera",
            "microphone",
            "security",
        ],
    },
    SettingsPane {
        id: "battery",
        name: "Battery",
        icon: "battery.100",
        url: "x-apple.systempreferences:com.apple.Battery-Settings.extension",
        keywords: &["power", "energy", "charging", "low power mode"],
    },
    SettingsPane {
        id: "keyboard",
        name: "Keyboard",
        icon: "keyboard",
        url: "x-apple.systempreferences:com.apple.Keyboard-Settings.extension",
        keywords: &["typing", "shortcuts", "input sources", "text"],
    },
    SettingsPane {
        id: "trackpad",
        name: "Trackpad",
        icon: "rectangle.and.hand.point.up.left",
        url: "x-apple.systempreferences:com.apple.Trackpad-Settings.extension",
        keywords: &["gestures", "tap", "scroll", "click"],
    },
    SettingsPane {
        id: "mouse",
        name: "Mouse",
        icon: "computermouse",
        url: "x-apple.systempreferences:com.apple.Mouse-Settings.extension",
        keywords: &["pointer", "scroll", "click", "tracking"],
    },
    SettingsPane {
        id: "printers",
        name: "Printers & Scanners",
        icon: "printer",
        url: "x-apple.systempreferences:com.apple.Print-Scan-Settings.extension",
        keywords: &["print", "scan", "fax"],
    },
    SettingsPane {
        id: "general",
        name: "General",
        icon: "gear",
        url: "x-apple.systempreferences:com.apple.systempreferences",
        keywords: &["about", "software update", "airdrop", "handoff"],
    },
    SettingsPane {
        id: "accessibility",
        name: "Accessibility",
        icon: "accessibility",
        url: "x-apple.systempreferences:com.apple.Accessibility-Settings.extension",
        keywords: &["voiceover", "zoom", "display", "motor", "hearing"],
    },
    SettingsPane {
        id: "siri",
        name: "Siri & Spotlight",
        icon: "mic",
        url: "x-apple.systempreferences:com.apple.Siri-Settings.extension",
        keywords: &["voice", "assistant", "search"],
    },
    SettingsPane {
        id: "control-center",
        name: "Control Center",
        icon: "switch.2",
        url: "x-apple.systempreferences:com.apple.ControlCenter-Settings.extension",
        keywords: &["menu bar", "shortcuts"],
    },
    SettingsPane {
        id: "desktop-dock",
        name: "Desktop & Dock",
        icon: "dock.rectangle",
        url: "x-apple.systempreferences:com.apple.Desktop-Settings.extension",
        keywords: &[
            "wallpaper",
            "screen saver",
            "hot corners",
            "mission control",
        ],
    },
    SettingsPane {
        id: "time-machine",
        name: "Time Machine",
        icon: "clock.arrow.circlepath",
        url: "x-apple.systempreferences:com.apple.Time-Machine-Settings.extension",
        keywords: &["backup", "restore"],
    },
    SettingsPane {
        id: "users",
        name: "Users & Groups",
        icon: "person.2",
        url: "x-apple.systempreferences:com.apple.Users-Groups-Settings.extension",
        keywords: &["accounts", "login", "password"],
    },
    SettingsPane {
        id: "passwords",
        name: "Passwords",
        icon: "key",
        url: "x-apple.systempreferences:com.apple.Passwords-Settings.extension",
        keywords: &["keychain", "autofill", "security"],
    },
    SettingsPane {
        id: "internet-accounts",
        name: "Internet Accounts",
        icon: "at",
        url: "x-apple.systempreferences:com.apple.Internet-Accounts-Settings.extension",
        keywords: &["email", "calendar", "contacts", "icloud", "google"],
    },
    SettingsPane {
        id: "wallet",
        name: "Wallet & Apple Pay",
        icon: "creditcard",
        url: "x-apple.systempreferences:com.apple.WalletSettingsExtension",
        keywords: &["payment", "cards", "apple pay"],
    },
];

/// Command handler for opening settings
struct OpenSettingsHandler;

impl CommandHandlerTrait for OpenSettingsHandler {
    fn handle(&self, ctx: ExtensionContext, args: CommandArguments) -> ExtensionApiResult<()> {
        // Filter panes based on query
        let query = args
            .query
            .as_ref()
            .map(photoncast_extension_api::RString::as_str)
            .unwrap_or("");

        let filtered_panes: Vec<&SettingsPane> = if query.is_empty() {
            SETTINGS_PANES.iter().collect()
        } else {
            SETTINGS_PANES.iter().filter(|p| p.matches(query)).collect()
        };

        let items: RVec<ListItem> = filtered_panes.iter().map(|p| p.to_list_item()).collect();

        let sections = RVec::from(vec![ListSection {
            title: ROption::RSome(RString::from("System Settings")),
            items,
        }]);

        let view = ExtensionView::List(ListView {
            title: RString::from("System Settings"),
            search_bar: ROption::RSome(SearchBarConfig {
                placeholder: RString::from("Search settings..."),
                throttle_ms: 100,
            }),
            sections,
            empty_state: ROption::RSome(EmptyState {
                icon: ROption::RSome(IconSource::SystemIcon {
                    name: RString::from("gear"),
                }),
                title: RString::from("No settings found"),
                description: ROption::RSome(RString::from("Try a different search term")),
                actions: RVec::new(),
            }),
            show_preview: false,
        });

        match ctx.host.render_view(view) {
            RResult::ROk(_) => ExtensionApiResult::ROk(()),
            RResult::RErr(e) => ExtensionApiResult::RErr(e),
        }
    }
}

/// System Preferences Extension
pub struct SystemPreferencesExtension {
    ctx: Option<ExtensionContext>,
}

impl SystemPreferencesExtension {
    const fn new() -> Self {
        Self { ctx: None }
    }
}

impl Extension for SystemPreferencesExtension {
    fn manifest(&self) -> ExtensionManifest {
        ExtensionManifest {
            id: RString::from("com.photoncast.settings"),
            name: RString::from("System Preferences"),
            version: RString::from("1.0.0"),
            description: ROption::RSome(RString::from("Open System Settings panes quickly")),
            author: ROption::RSome(RString::from("PhotonCast")),
            license: ROption::RSome(RString::from("MIT")),
            homepage: ROption::RSome(RString::from("https://github.com/photoncast/photoncast")),
            min_photoncast_version: ROption::RNone,
            api_version: 1,
        }
    }

    fn activate(&mut self, ctx: ExtensionContext) -> ExtensionApiResult<()> {
        self.ctx = Some(ctx);
        ExtensionApiResult::ROk(())
    }

    fn deactivate(&mut self) -> ExtensionApiResult<()> {
        self.ctx = None;
        ExtensionApiResult::ROk(())
    }

    fn search_provider(&self) -> ROption<ExtensionSearchProvider_TO<'static, RBox<()>>> {
        // This extension uses view mode, not search mode
        ROption::RNone
    }

    fn commands(&self) -> RVec<ExtensionCommand> {
        RVec::from(vec![ExtensionCommand {
            id: RString::from("open-settings"),
            name: RString::from("Open System Settings"),
            mode: CommandMode::View,
            keywords: RVec::from(vec![
                RString::from("settings"),
                RString::from("preferences"),
                RString::from("system"),
            ]),
            handler: CommandHandler::new(OpenSettingsHandler),
            icon: ROption::RSome(IconSource::SystemIcon {
                name: RString::from("gear"),
            }),
            subtitle: ROption::RSome(RString::from("Open macOS System Settings")),
            permissions: RVec::new(),
        }])
    }
}

/// Creates the extension instance (called by PhotonCast)
#[no_mangle]
pub extern "C" fn create_extension() -> ExtensionBox {
    Extension_TO::from_value(SystemPreferencesExtension::new(), TD_Opaque)
}

#[abi_stable::export_root_module]
fn instantiate_root_module() -> ExtensionApiRootModule_Ref {
    ExtensionApiRootModule { create_extension }.leak_into_prefix()
}
