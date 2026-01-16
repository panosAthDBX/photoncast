# GPUI Components

## Overview

GPUI is a GPU-accelerated UI framework with a hybrid immediate/retained mode rendering model. Components are built using a declarative, Tailwind-like API with strong Rust typing.

## When to Apply

- Building any UI component
- Creating reusable UI elements
- Implementing views and layouts

## Core Principles

1. **Composition over inheritance** - Build complex UIs from simple components
2. **Type-safe styling** - Use GPUI's builder API, not raw CSS
3. **State locality** - Keep state as close to where it's used as possible
4. **Performance first** - GPUI renders at 120 FPS, don't break that

## ✅ DO

### DO: Use the Render Trait

**✅ DO**:
```rust
use gpui::prelude::*;

pub struct SearchBar {
    query: String,
    focused: bool,
}

impl Render for SearchBar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .px_4()
            .py_2()
            .bg(cx.theme().colors().surface)
            .rounded_lg()
            .child(
                input()
                    .placeholder("Search...")
                    .value(&self.query)
                    .on_change(cx.listener(|this, value, cx| {
                        this.query = value;
                        cx.notify();
                    }))
            )
    }
}
```

### DO: Use gpui-component Library

**✅ DO**:
```rust
use gpui_component::prelude::*;

fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    v_flex()
        .gap_2()
        .child(
            Button::new("search")
                .label("Search")
                .icon(Icon::Search)
                .on_click(cx.listener(|this, _, cx| {
                    this.perform_search(cx);
                }))
        )
        .child(
            Input::new("query")
                .placeholder("Type to search...")
                .value(&self.query)
                .on_change(cx.listener(|this, value, cx| {
                    this.query = value;
                    cx.notify();
                }))
        )
}
```

### DO: Use Flex Layouts

**✅ DO**:
```rust
// Vertical flex container
v_flex()
    .gap_4()
    .p_4()
    .child(header())
    .child(content())
    .child(footer())

// Horizontal flex container  
h_flex()
    .justify_between()
    .items_center()
    .child(left_content())
    .child(right_content())

// With specific sizing
div()
    .flex()
    .flex_col()
    .w_full()
    .h(px(400.0))
    .overflow_y_scroll()
```

### DO: Handle Events with Listeners

**✅ DO**:
```rust
impl SearchResults {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .children(self.results.iter().enumerate().map(|(idx, result)| {
                ResultItem::new(result.clone())
                    .selected(idx == self.selected_index)
                    .on_click(cx.listener(move |this, _, cx| {
                        this.select(idx, cx);
                    }))
                    .on_double_click(cx.listener(move |this, _, cx| {
                        this.activate(idx, cx);
                    }))
            }))
    }
}
```

### DO: Use Actions for Keyboard Shortcuts

**✅ DO**:
```rust
use gpui::actions;

// Define actions
actions!(launcher, [
    SelectNext,
    SelectPrevious,
    Activate,
    Cancel,
    ToggleDetails,
]);

// Register key bindings
fn register_bindings(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("down", SelectNext, Some("Launcher")),
        KeyBinding::new("up", SelectPrevious, Some("Launcher")),
        KeyBinding::new("enter", Activate, Some("Launcher")),
        KeyBinding::new("escape", Cancel, Some("Launcher")),
        KeyBinding::new("cmd-i", ToggleDetails, Some("Launcher")),
    ]);
}

// Handle actions in component
impl Launcher {
    fn select_next(&mut self, _: &SelectNext, cx: &mut Context<Self>) {
        self.selected_index = (self.selected_index + 1).min(self.results.len() - 1);
        cx.notify();
    }
}
```

### DO: Use Theming

**✅ DO**:
```rust
fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    let theme = cx.theme();
    let colors = theme.colors();
    
    div()
        .bg(colors.background)
        .text_color(colors.text)
        .border_1()
        .border_color(colors.border)
        .child(/* ... */)
}
```

### DO: Create Reusable Components

**✅ DO**:
```rust
pub struct ResultItem {
    result: SearchResult,
    selected: bool,
    on_click: Option<Box<dyn Fn(&mut Window, &mut App) + 'static>>,
}

impl ResultItem {
    pub fn new(result: SearchResult) -> Self {
        Self {
            result,
            selected: false,
            on_click: None,
        }
    }
    
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }
    
    pub fn on_click(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Box::new(handler));
        self
    }
}

impl RenderOnce for ResultItem {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let colors = cx.theme().colors();
        
        div()
            .flex()
            .items_center()
            .gap_3()
            .px_4()
            .py_2()
            .rounded_md()
            .when(self.selected, |el| el.bg(colors.selection))
            .when_some(self.on_click, |el, handler| {
                el.on_click(move |_, window, cx| handler(window, cx))
            })
            .child(Icon::new(self.result.icon))
            .child(
                v_flex()
                    .child(div().text_sm().font_medium().child(&self.result.name))
                    .child(div().text_xs().text_color(colors.text_muted).child(&self.result.path))
            )
    }
}
```

### DO: Use Entity for Stateful Components

**✅ DO**:
```rust
use gpui::{Entity, Context};

pub struct SearchState {
    query: String,
    results: Vec<SearchResult>,
    selected: usize,
}

// Create entity
let search_state = cx.new(|_| SearchState {
    query: String::new(),
    results: Vec::new(),
    selected: 0,
});

// Update entity
cx.update_entity(&search_state, |state, cx| {
    state.query = "new query".to_string();
    cx.notify();
});

// Read entity
let query = cx.read_entity(&search_state, |state, _| state.query.clone());
```

## ❌ DON'T

### DON'T: Use Raw Pixel Values Everywhere

**❌ DON'T**:
```rust
div()
    .w(px(384.0))    // Magic number
    .h(px(512.0))    // What does this mean?
    .p(px(16.0))     // Inconsistent with design system
```

**✅ DO**:
```rust
// Define constants or use theme spacing
const LAUNCHER_WIDTH: Pixels = px(600.0);
const LAUNCHER_HEIGHT: Pixels = px(400.0);

div()
    .w(LAUNCHER_WIDTH)
    .h(LAUNCHER_HEIGHT)
    .p_4()  // Use Tailwind-like spacing scale
```

### DON'T: Inline Complex Logic in render()

**❌ DON'T**:
```rust
fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    // Don't do heavy computation in render
    let filtered: Vec<_> = self.items
        .iter()
        .filter(|i| i.name.contains(&self.query))
        .collect();
    
    // Don't sort in render
    let mut sorted = filtered;
    sorted.sort_by(|a, b| b.score.cmp(&a.score));
    
    div().children(sorted.iter().map(/* ... */))
}
```

**✅ DO**:
```rust
impl SearchView {
    fn update_results(&mut self, cx: &mut Context<Self>) {
        // Do filtering and sorting outside render
        self.filtered_results = self.items
            .iter()
            .filter(|i| i.name.contains(&self.query))
            .cloned()
            .collect();
        
        self.filtered_results.sort_by(|a, b| b.score.cmp(&a.score));
        cx.notify();
    }
}

fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
    // Render uses pre-computed results
    div().children(self.filtered_results.iter().map(/* ... */))
}
```

### DON'T: Create New Closures in Loops Without Care

**❌ DON'T**:
```rust
fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    div().children(self.items.iter().map(|item| {
        let id = item.id.clone();  // Clone every render
        div()
            .on_click(cx.listener(move |this, _, cx| {
                this.select(&id, cx);  // Closure captures clone
            }))
    }))
}
```

**✅ DO**:
```rust
fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    div().children(self.items.iter().enumerate().map(|(idx, item)| {
        // Use index instead of cloning ID
        div()
            .on_click(cx.listener(move |this, _, cx| {
                this.select_index(idx, cx);
            }))
            .child(&item.name)
    }))
}
```

### DON'T: Forget to Call notify()

**❌ DON'T**:
```rust
fn update_query(&mut self, query: String, _cx: &mut Context<Self>) {
    self.query = query;
    // Forgot cx.notify() - UI won't update!
}
```

**✅ DO**:
```rust
fn update_query(&mut self, query: String, cx: &mut Context<Self>) {
    self.query = query;
    cx.notify();  // Triggers re-render
}
```

### DON'T: Block the Main Thread

**❌ DON'T**:
```rust
fn search(&mut self, cx: &mut Context<Self>) {
    // DON'T: Blocking I/O in UI code
    let results = std::fs::read_dir("/Applications")
        .unwrap()
        .collect::<Vec<_>>();
    
    self.results = process(results);
    cx.notify();
}
```

**✅ DO**:
```rust
fn search(&mut self, cx: &mut Context<Self>) {
    let query = self.query.clone();
    
    cx.spawn(|this, mut cx| async move {
        // Async I/O off main thread
        let results = search_async(&query).await;
        
        this.update(&mut cx, |this, cx| {
            this.results = results;
            cx.notify();
        }).ok();
    }).detach();
}
```

## Layout Patterns

### Pattern: Main Launcher Window

```rust
pub struct LauncherView {
    search_input: Entity<SearchInput>,
    results: Entity<ResultsList>,
}

impl Render for LauncherView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .w(px(600.0))
            .max_h(px(400.0))
            .bg(cx.theme().colors().surface)
            .rounded_xl()
            .shadow_lg()
            .border_1()
            .border_color(cx.theme().colors().border)
            .overflow_hidden()
            .child(self.search_input.clone())
            .child(Divider::horizontal())
            .child(
                div()
                    .flex_1()
                    .overflow_y_scroll()
                    .child(self.results.clone())
            )
    }
}
```

### Pattern: List Item with Selection

```rust
pub struct ListItem {
    pub icon: IconName,
    pub title: SharedString,
    pub subtitle: Option<SharedString>,
    pub selected: bool,
}

impl RenderOnce for ListItem {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let colors = cx.theme().colors();
        
        h_flex()
            .gap_3()
            .px_3()
            .py_2()
            .w_full()
            .rounded_md()
            .cursor_pointer()
            .when(self.selected, |el| {
                el.bg(colors.selection)
            })
            .hover(|el| el.bg(colors.hover))
            .child(
                Icon::new(self.icon)
                    .size_5()
                    .text_color(colors.icon)
            )
            .child(
                v_flex()
                    .flex_1()
                    .overflow_hidden()
                    .child(
                        div()
                            .text_sm()
                            .font_medium()
                            .truncate()
                            .child(self.title)
                    )
                    .when_some(self.subtitle, |el, subtitle| {
                        el.child(
                            div()
                                .text_xs()
                                .text_color(colors.text_muted)
                                .truncate()
                                .child(subtitle)
                        )
                    })
            )
    }
}
```

### Pattern: Keyboard Navigation

```rust
impl LauncherView {
    fn handle_key_down(&mut self, event: &KeyDownEvent, cx: &mut Context<Self>) {
        match event.keystroke.key.as_str() {
            "down" => self.select_next(cx),
            "up" => self.select_previous(cx),
            "enter" => self.activate_selected(cx),
            "escape" => self.close(cx),
            _ => {}
        }
    }
    
    fn select_next(&mut self, cx: &mut Context<Self>) {
        if self.selected_index < self.results.len().saturating_sub(1) {
            self.selected_index += 1;
            cx.notify();
        }
    }
    
    fn select_previous(&mut self, cx: &mut Context<Self>) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
            cx.notify();
        }
    }
}
```

## Resources

- [GPUI Documentation](https://gpui.rs)
- [gpui-component](https://longbridge.github.io/gpui-component/)
- [Zed Source Code](https://github.com/zed-industries/zed) - Real-world GPUI usage
- [Loungy Source Code](https://github.com/MatthiasGrandl/Loungy) - Launcher example
