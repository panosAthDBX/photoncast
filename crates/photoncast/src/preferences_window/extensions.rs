//! Extension management settings section.

use super::*;

impl PreferencesWindow {
    pub(super) fn render_extensions_section(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let colors = get_colors(cx);

        // Get extensions from PhotonCastApp if available
        let extensions = self
            .photoncast_app
            .as_ref()
            .map(|app| app.read().get_all_extensions())
            .unwrap_or_default();

        div()
            .id("extensions-section")
            .flex()
            .flex_col()
            .gap(SECTION_GAP)
            .child(
                div()
                    .text_size(px(13.0))
                    .text_color(colors.text_muted)
                    .child("Manage installed extensions and their permissions."),
            )
            .when(extensions.is_empty(), |this: gpui::Stateful<gpui::Div>| {
                this.child(
                    div()
                        .p(px(20.0))
                        .rounded(px(8.0))
                        .bg(colors.surface)
                        .border_1()
                        .border_color(colors.border)
                        .flex()
                        .flex_col()
                        .items_center()
                        .gap(px(8.0))
                        .child(
                            div()
                                .text_size(px(14.0))
                                .text_color(colors.text_muted)
                                .child("No extensions installed"),
                        )
                        .child(
                            div()
                                .text_size(TEXT_SIZE_SM)
                                .text_color(colors.text_placeholder)
                                .child("Extensions will appear here once installed."),
                        ),
                )
            })
            .children(extensions.into_iter().map(
                |(id, name, enabled, state, permissions, has_consent, commands)| {
                    self.render_extension_row(
                        &id,
                        &name,
                        enabled,
                        state,
                        &permissions,
                        has_consent,
                        &commands,
                        &colors,
                        cx,
                    )
                },
            ))
    }

    #[allow(clippy::too_many_arguments)]
    fn render_extension_row(
        &self,
        id: &str,
        name: &str,
        enabled: bool,
        state: ExtensionState,
        permissions: &[String],
        has_consent: bool,
        commands: &[String],
        colors: &PrefsColors,
        cx: &mut ViewContext<Self>,
    ) -> impl IntoElement {
        let id_owned = id.to_string();
        let id_for_toggle = id.to_string();
        let id_for_revoke = id.to_string();

        let state_text = match state {
            ExtensionState::Discovered => "Discovered",
            ExtensionState::Loaded => "Loaded",
            ExtensionState::Active => "Active",
            ExtensionState::Disabled => "Disabled",
            ExtensionState::Failed => "Failed",
            ExtensionState::Unloaded => "Unloaded",
        };

        let state_color = match state {
            ExtensionState::Active => hsla(0.33, 0.7, 0.45, 1.0), // Green
            ExtensionState::Failed => hsla(0.0, 0.7, 0.5, 1.0),   // Red
            ExtensionState::Disabled => colors.text_placeholder,
            _ => colors.text_muted,
        };

        let surface_hover = colors.surface_hover;
        let text_color = colors.text;
        let text_muted = colors.text_muted;
        let accent = colors.accent;

        div()
            .id(SharedString::from(format!("ext-{}", id_owned)))
            .p(px(12.0))
            .rounded(px(8.0))
            .bg(colors.surface)
            .border_1()
            .border_color(colors.border)
            .flex()
            .flex_col()
            .gap(px(8.0))
            // Header row with name and toggle
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .child(
                                div()
                                    .text_size(px(14.0))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(text_color)
                                    .child(name.to_string()),
                            )
                            .child(
                                div()
                                    .px(px(6.0))
                                    .py(px(2.0))
                                    .rounded(px(4.0))
                                    .bg(state_color.opacity(0.15))
                                    .text_size(px(10.0))
                                    .text_color(state_color)
                                    .child(state_text),
                            ),
                    )
                    .child({
                        let toggle_bg = if enabled { accent } else { surface_hover };
                        let knob_x = if enabled { px(14.0) } else { px(2.0) };
                        div()
                            .id(SharedString::from(format!("toggle-{}", id_for_toggle.clone())))
                            .w(px(36.0))
                            .h(px(20.0))
                            .rounded(px(10.0))
                            .bg(toggle_bg)
                            .cursor_pointer()
                            .on_click(cx.listener(move |this, _, cx| {
                                if let Some(app) = &this.photoncast_app {
                                    let _ = app.read().toggle_extension_enabled(&id_for_toggle);
                                    cx.notify();
                                }
                            }))
                            .child(
                                div()
                                    .absolute()
                                    .top(px(2.0))
                                    .left(knob_x)
                                    .size(ICON_SIZE_SM)
                                    .rounded_full()
                                    .bg(white()),
                            )
                    }),
            )
            // ID and permissions
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(4.0))
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(text_muted)
                            .child(format!("ID: {}", id_owned)),
                    )
                    .when(!permissions.is_empty(), |this| {
                        this.child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(4.0))
                                .child(
                                    div()
                                        .text_size(px(11.0))
                                        .text_color(text_muted)
                                        .child("Permissions:"),
                                )
                                .children(permissions.iter().map(|p| {
                                    let icon = match p.as_str() {
                                        "Network" => "🌐",
                                        "Clipboard" => "📋",
                                        "Notifications" => "🔔",
                                        "Filesystem" => "📁",
                                        _ => "•",
                                    };
                                    div()
                                        .px(px(4.0))
                                        .py(px(1.0))
                                        .rounded(px(3.0))
                                        .bg(surface_hover)
                                        .text_size(px(10.0))
                                        .text_color(text_muted)
                                        .child(format!("{} {}", icon, p))
                                })),
                        )
                    }),
            )
            // Commands list
            .when(!commands.is_empty(), |this| {
                this.child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(4.0))
                        .child(
                            div()
                                .text_size(px(11.0))
                                .text_color(text_muted)
                                .child("Commands:"),
                        )
                        .children(commands.iter().map(|cmd| {
                            div()
                                .px(px(4.0))
                                .py(px(1.0))
                                .rounded(px(3.0))
                                .bg(surface_hover)
                                .text_size(px(10.0))
                                .text_color(text_muted)
                                .child(cmd.clone())
                        })),
                )
            })
            // Consent status and grant/revoke button
            .when(!permissions.is_empty(), |this| {
                let id_for_grant = id_for_revoke.clone();
                this.child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(
                            div()
                                .text_size(px(11.0))
                                .text_color(if has_consent {
                                    hsla(0.33, 0.7, 0.45, 1.0)
                                } else {
                                    hsla(0.08, 0.7, 0.5, 1.0)
                                })
                                .child(if has_consent {
                                    "✓ Permissions granted"
                                } else {
                                    "⚠ Permissions not granted"
                                }),
                        )
                        .when(has_consent, |this| {
                            this.child(
                                div()
                                    .id(SharedString::from(format!("revoke-{}", id_for_revoke.clone())))
                                    .px(px(8.0))
                                    .py(px(4.0))
                                    .rounded(px(4.0))
                                    .bg(hsla(0.0, 0.6, 0.5, 0.15))
                                    .hover(|s| s.bg(hsla(0.0, 0.6, 0.5, 0.25)))
                                    .cursor_pointer()
                                    .on_click(cx.listener(move |this, _, cx| {
                                        if let Some(app) = &this.photoncast_app {
                                            let _ = app.read().revoke_extension_permissions(&id_for_revoke);
                                            cx.notify();
                                        }
                                    }))
                                    .child(
                                        div()
                                            .text_size(px(11.0))
                                            .text_color(hsla(0.0, 0.7, 0.5, 1.0))
                                            .child("Revoke"),
                                    ),
                            )
                        })
                        .when(!has_consent, |this| {
                            this.child(
                                div()
                                    .id(SharedString::from(format!("grant-{}", id_for_grant.clone())))
                                    .px(px(8.0))
                                    .py(px(4.0))
                                    .rounded(px(4.0))
                                    .bg(accent.opacity(0.15))
                                    .hover(|s| s.bg(accent.opacity(0.25)))
                                    .cursor_pointer()
                                    .on_click(cx.listener(move |this, _, cx| {
                                        if let Some(app) = &this.photoncast_app {
                                            let _ = app.read().accept_extension_permissions(&id_for_grant);
                                            cx.notify();
                                        }
                                    }))
                                    .child(
                                        div()
                                            .text_size(px(11.0))
                                            .text_color(accent)
                                            .child("Grant Access"),
                                    ),
                            )
                        }),
                )
            })
    }
}
