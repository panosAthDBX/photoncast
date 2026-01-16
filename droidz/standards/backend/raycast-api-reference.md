# Raycast API Reference

> Complete API reference for Raycast extension compatibility layer

## Overview

This document defines all Raycast API components, hooks, and utilities that PhotonCast must implement to achieve 80%+ extension compatibility.

---

## UI Components

### List

The primary component for displaying searchable lists of items.

```typescript
interface ListProps {
  // Loading state
  isLoading?: boolean;
  
  // Search configuration
  searchBarPlaceholder?: string;
  searchText?: string;
  onSearchTextChange?: (text: string) => void;
  filtering?: boolean | { keepSectionOrder: boolean };
  throttle?: boolean;
  
  // Navigation
  navigationTitle?: string;
  isShowingDetail?: boolean;
  
  // Pagination
  pagination?: { pageSize: number; hasMore: boolean; onLoadMore: () => void };
  
  // Selection
  selectedItemId?: string;
  onSelectionChange?: (id: string | null) => void;
  
  // Actions
  actions?: ActionPanel;
  searchBarAccessory?: JSX.Element;
}

// List.Item
interface ListItemProps {
  id: string;
  title: string;
  subtitle?: string;
  icon?: Image.ImageLike;
  keywords?: string[];
  accessories?: List.Item.Accessory[];
  actions?: ActionPanel;
  detail?: List.Item.Detail;
}

// List.Item.Accessory
interface Accessory {
  text?: string | { value: string; color?: Color };
  icon?: Image.ImageLike;
  date?: Date;
  tag?: { value: string; color?: Color };
  tooltip?: string;
}

// List.Item.Detail (for split view)
interface DetailProps {
  markdown?: string;
  metadata?: Detail.Metadata;
  isLoading?: boolean;
}

// List.Section
interface ListSectionProps {
  title?: string;
  subtitle?: string;
  children: ListItem[];
}

// List.EmptyView
interface EmptyViewProps {
  title?: string;
  description?: string;
  icon?: Image.ImageLike;
  actions?: ActionPanel;
}

// List.Dropdown (search bar accessory)
interface DropdownProps {
  tooltip: string;
  value?: string;
  onChange?: (value: string) => void;
  storeValue?: boolean;
  children: Dropdown.Item[] | Dropdown.Section[];
}
```

**GPUI Mapping:**
```rust
/// PhotonCast List implementation
pub struct ExtensionList {
    is_loading: bool,
    search_query: String,
    items: Vec<ExtensionListItem>,
    sections: Vec<ExtensionListSection>,
    selected_index: usize,
    show_detail: bool,
}

impl Render for ExtensionList {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        h_flex()
            .w_full()
            .h_full()
            .child(
                v_flex()
                    .w(if self.show_detail { px(300.0) } else { px(600.0) })
                    .child(self.render_search_bar(cx))
                    .child(self.render_items(cx))
            )
            .when(self.show_detail, |el| {
                el.child(self.render_detail_panel(cx))
            })
    }
}
```

---

### Grid

Display items in a grid layout with customizable columns.

```typescript
interface GridProps {
  // Core props (same as List)
  isLoading?: boolean;
  searchBarPlaceholder?: string;
  searchText?: string;
  onSearchTextChange?: (text: string) => void;
  filtering?: boolean;
  throttle?: boolean;
  
  // Grid-specific
  columns?: number; // 1-8, default based on window width
  aspectRatio?: "1" | "3/2" | "2/3" | "4/3" | "3/4" | "16/9" | "9/16";
  fit?: "contain" | "fill";
  inset?: "sm" | "md" | "lg";
  
  // Selection
  selectedItemId?: string;
  onSelectionChange?: (id: string | null) => void;
}

// Grid.Item
interface GridItemProps {
  id: string;
  title?: string;
  subtitle?: string;
  content: Image.ImageLike | Grid.Item.Content;
  keywords?: string[];
  accessory?: Grid.Item.Accessory;
  actions?: ActionPanel;
  quickLook?: { path: string; name?: string };
}

// Grid.Section
interface GridSectionProps {
  title?: string;
  subtitle?: string;
  aspectRatio?: string;
  fit?: string;
  inset?: string;
  columns?: number;
  children: GridItem[];
}
```

---

### Detail

Display rich content with markdown support.

```typescript
interface DetailProps {
  markdown?: string;
  navigationTitle?: string;
  isLoading?: boolean;
  actions?: ActionPanel;
  metadata?: Detail.Metadata;
}

// Detail.Metadata - sidebar metadata
interface MetadataProps {
  children: (
    | Detail.Metadata.Label
    | Detail.Metadata.Link
    | Detail.Metadata.TagList
    | Detail.Metadata.Separator
  )[];
}

// Metadata components
interface LabelProps {
  title: string;
  text?: string;
  icon?: Image.ImageLike;
}

interface LinkProps {
  title: string;
  target: string;
  text: string;
}

interface TagListProps {
  title: string;
  children: Detail.Metadata.TagList.Item[];
}

interface TagItemProps {
  text: string;
  color?: Color;
  onAction?: () => void;
}
```

**Markdown Support:**
- Headers (h1-h6)
- Bold, italic, strikethrough
- Code blocks with syntax highlighting
- Links (external and internal)
- Images (local and remote)
- Tables
- Lists (ordered and unordered)
- Blockquotes
- Horizontal rules

---

### Form

Collect user input with various field types.

```typescript
interface FormProps {
  navigationTitle?: string;
  isLoading?: boolean;
  actions?: ActionPanel;
  enableDrafts?: boolean;
}

// Form field types
interface TextFieldProps {
  id: string;
  title?: string;
  placeholder?: string;
  defaultValue?: string;
  value?: string;
  error?: string;
  info?: string;
  storeValue?: boolean;
  autoFocus?: boolean;
  onChange?: (value: string) => void;
  onBlur?: (event: Event) => void;
}

interface TextAreaProps extends TextFieldProps {
  enableMarkdown?: boolean;
}

interface PasswordFieldProps extends TextFieldProps {}

interface CheckboxProps {
  id: string;
  title?: string;
  label: string;
  defaultValue?: boolean;
  value?: boolean;
  storeValue?: boolean;
  onChange?: (value: boolean) => void;
}

interface DatePickerProps {
  id: string;
  title?: string;
  defaultValue?: Date;
  value?: Date;
  type?: "date" | "datetime";
  min?: Date;
  max?: Date;
  onChange?: (value: Date | null) => void;
}

interface DropdownProps {
  id: string;
  title?: string;
  defaultValue?: string;
  value?: string;
  storeValue?: boolean;
  onChange?: (value: string) => void;
  children: Form.Dropdown.Item[] | Form.Dropdown.Section[];
}

interface TagPickerProps {
  id: string;
  title?: string;
  defaultValue?: string[];
  value?: string[];
  onChange?: (value: string[]) => void;
  children: Form.TagPicker.Item[];
}

interface FilePickerProps {
  id: string;
  title?: string;
  defaultValue?: string[];
  value?: string[];
  allowMultipleSelection?: boolean;
  canChooseDirectories?: boolean;
  canChooseFiles?: boolean;
  onChange?: (value: string[]) => void;
}

interface SeparatorProps {}

interface DescriptionProps {
  title?: string;
  text: string;
}
```

---

### Action & ActionPanel

Define actions available on items.

```typescript
interface ActionPanelProps {
  title?: string;
  children: (Action | ActionPanel.Section | ActionPanel.Submenu)[];
}

interface ActionPanelSectionProps {
  title?: string;
  children: Action[];
}

interface ActionPanelSubmenuProps {
  title: string;
  icon?: Image.ImageLike;
  shortcut?: Keyboard.Shortcut;
  children: (Action | ActionPanel.Section)[];
}

// Built-in Actions
namespace Action {
  // Navigation
  interface PushProps {
    title: string;
    icon?: Image.ImageLike;
    shortcut?: Keyboard.Shortcut;
    target: JSX.Element;
  }
  
  interface PopProps {
    title?: string;
    icon?: Image.ImageLike;
    shortcut?: Keyboard.Shortcut;
  }
  
  // Clipboard
  interface CopyToClipboardProps {
    title?: string;
    content: string | number | { text: string; html?: string };
    icon?: Image.ImageLike;
    shortcut?: Keyboard.Shortcut;
    concepialed?: boolean;
    transient?: boolean;
  }
  
  interface PasteProps {
    title?: string;
    content: string | number | { text: string; html?: string };
    icon?: Image.ImageLike;
    shortcut?: Keyboard.Shortcut;
  }
  
  // URLs & Files
  interface OpenInBrowserProps {
    title?: string;
    url: string;
    icon?: Image.ImageLike;
    shortcut?: Keyboard.Shortcut;
  }
  
  interface OpenProps {
    title: string;
    target: string;
    application?: Application | string;
    icon?: Image.ImageLike;
    shortcut?: Keyboard.Shortcut;
  }
  
  interface ShowInFinderProps {
    title?: string;
    path: string;
    icon?: Image.ImageLike;
    shortcut?: Keyboard.Shortcut;
  }
  
  interface TrashProps {
    title?: string;
    paths: string | string[];
    icon?: Image.ImageLike;
    shortcut?: Keyboard.Shortcut;
  }
  
  // Form submission
  interface SubmitFormProps {
    title?: string;
    icon?: Image.ImageLike;
    shortcut?: Keyboard.Shortcut;
    onSubmit: (values: Form.Values) => void | Promise<void>;
  }
  
  // Selection
  interface PickDateProps {
    title: string;
    icon?: Image.ImageLike;
    shortcut?: Keyboard.Shortcut;
    onChange: (date: Date | null) => void;
    type?: "date" | "datetime";
    min?: Date;
    max?: Date;
  }
  
  // Custom
  interface Props {
    title: string;
    icon?: Image.ImageLike;
    shortcut?: Keyboard.Shortcut;
    style?: Action.Style;
    autoFocus?: boolean;
    onAction: () => void | Promise<void>;
  }
}

// Action styles
enum ActionStyle {
  Regular = "regular",
  Destructive = "destructive",
}

// Keyboard shortcuts
interface KeyboardShortcut {
  modifiers: ("cmd" | "ctrl" | "opt" | "shift")[];
  key: string;
}
```

**Default Shortcuts:**
| Action | Shortcut |
|--------|----------|
| Primary action | `Enter` |
| Secondary action | `Cmd+Enter` |
| Copy to clipboard | `Cmd+C` |
| Open in browser | `Cmd+O` |
| Show in Finder | `Cmd+Shift+O` |
| Delete/Trash | `Cmd+Backspace` |
| Refresh | `Cmd+R` |
| Open action panel | `Cmd+K` |

---

## Navigation Hooks

### useNavigation

```typescript
interface UseNavigation {
  push: <T extends { [key: string]: unknown }>(
    component: React.ComponentType<T>,
    props?: T
  ) => void;
  pop: () => void;
}

// Usage
function MyComponent() {
  const { push, pop } = useNavigation();
  
  return (
    <Action
      title="View Details"
      onAction={() => push(DetailView, { id: "123" })}
    />
  );
}
```

---

## Data Hooks

### useCachedPromise

Fetch and cache async data with automatic revalidation.

```typescript
function useCachedPromise<T, U = undefined>(
  fn: (...args: any[]) => Promise<T>,
  args?: any[],
  options?: {
    initialData?: U;
    keepPreviousData?: boolean;
    abortable?: React.MutableRefObject<AbortController | null>;
    execute?: boolean;
    onError?: (error: Error) => void;
    onData?: (data: T) => void;
    onWillExecute?: (args: any[]) => void;
    failureToastOptions?: Partial<Toast.Options>;
  }
): {
  data: T | U | undefined;
  error: Error | undefined;
  isLoading: boolean;
  revalidate: () => Promise<T>;
  mutate: (
    data?: T | Promise<T> | ((currentData: T | U | undefined) => T | Promise<T>),
    options?: { optimisticUpdate?: (data: T | U | undefined) => T; rollbackOnError?: boolean }
  ) => Promise<T | U | undefined>;
};

// Usage
function MyCommand() {
  const { data, isLoading, revalidate } = useCachedPromise(
    async (query: string) => {
      const response = await fetch(`/api/search?q=${query}`);
      return response.json();
    },
    [searchText],
    { keepPreviousData: true }
  );
  
  return <List isLoading={isLoading}>{/* ... */}</List>;
}
```

### useCachedState

Persist state across command executions.

```typescript
function useCachedState<T>(
  key: string,
  initialState?: T
): [T | undefined, (value: T | ((prev: T | undefined) => T)) => Promise<void>];

// Usage
function MyCommand() {
  const [favorites, setFavorites] = useCachedState<string[]>("favorites", []);
  
  const toggleFavorite = async (id: string) => {
    await setFavorites((prev) => 
      prev?.includes(id) 
        ? prev.filter(x => x !== id)
        : [...(prev ?? []), id]
    );
  };
}
```

### usePromise

Execute async functions with loading state.

```typescript
function usePromise<T>(
  fn: (...args: any[]) => Promise<T>,
  args?: any[],
  options?: {
    abortable?: React.MutableRefObject<AbortController | null>;
    execute?: boolean;
    onError?: (error: Error) => void;
    onData?: (data: T) => void;
    onWillExecute?: (args: any[]) => void;
    failureToastOptions?: Partial<Toast.Options>;
  }
): {
  data: T | undefined;
  error: Error | undefined;
  isLoading: boolean;
  revalidate: () => Promise<T>;
  mutate: MutatePromise<T | undefined>;
};
```

### useFetch

Fetch data from URLs with caching.

```typescript
function useFetch<T, U = undefined>(
  url: RequestInfo,
  options?: RequestInit & {
    mapResult?: (result: unknown) => T;
    initialData?: U;
    keepPreviousData?: boolean;
    execute?: boolean;
    onError?: (error: Error) => void;
    onData?: (data: T) => void;
    onWillExecute?: (args: [RequestInfo, RequestInit]) => void;
    failureToastOptions?: Partial<Toast.Options>;
  }
): {
  data: T | U | undefined;
  error: Error | undefined;
  isLoading: boolean;
  revalidate: () => Promise<T>;
  mutate: MutatePromise<T | U | undefined>;
};
```

### useLocalStorage (from @raycast/utils)

```typescript
function useLocalStorage<T>(
  key: string,
  initialValue?: T
): {
  value: T | undefined;
  setValue: (value: T | ((prev: T | undefined) => T)) => Promise<void>;
  removeValue: () => Promise<void>;
  isLoading: boolean;
};
```

---

## Storage APIs

### LocalStorage

Persistent key-value storage for extensions.

```typescript
namespace LocalStorage {
  function getItem(key: string): Promise<string | undefined>;
  function setItem(key: string, value: string): Promise<void>;
  function removeItem(key: string): Promise<void>;
  function allItems(): Promise<Record<string, string>>;
  function clear(): Promise<void>;
}
```

**PhotonCast Implementation:**
```rust
pub struct ExtensionStorage {
    db: rusqlite::Connection,
    extension_id: String,
}

impl ExtensionStorage {
    pub async fn get(&self, key: &str) -> Result<Option<String>> {
        let row: Option<String> = self.db.query_row(
            "SELECT value FROM extension_storage WHERE extension_id = ?1 AND key = ?2",
            params![&self.extension_id, key],
            |row| row.get(0),
        ).optional()?;
        Ok(row)
    }
    
    pub async fn set(&self, key: &str, value: &str) -> Result<()> {
        self.db.execute(
            "INSERT OR REPLACE INTO extension_storage (extension_id, key, value) VALUES (?1, ?2, ?3)",
            params![&self.extension_id, key, value],
        )?;
        Ok(())
    }
}
```

### Cache

Temporary caching with TTL.

```typescript
class Cache {
  constructor(options?: { namespace?: string; capacity?: number });
  
  get(key: string): string | undefined;
  set(key: string, value: string): void;
  remove(key: string): boolean;
  has(key: string): boolean;
  clear(): void;
  isEmpty: boolean;
  
  subscribe(subscriber: (key: string | undefined, data: string | undefined) => void): Subscription;
}
```

---

## Clipboard API

```typescript
namespace Clipboard {
  // Content types
  interface TextContent {
    text: string;
  }
  
  interface HtmlContent {
    text: string;
    html: string;
  }
  
  interface FileContent {
    file: string; // file path
  }
  
  type Content = string | TextContent | HtmlContent | FileContent;
  
  // Functions
  function copy(content: Content, options?: { concealed?: boolean; transient?: boolean }): Promise<void>;
  function paste(content: Content): Promise<void>;
  function read(): Promise<ReadContent>;
  function readText(): Promise<string | undefined>;
  function clear(): Promise<void>;
}

// Read content structure
interface ReadContent {
  text?: string;
  html?: string;
  file?: string;
}
```

---

## Toast & Notifications

```typescript
// showToast
function showToast(options: Toast.Options): Promise<Toast>;
function showToast(style: Toast.Style, title: string, message?: string): Promise<Toast>;

interface ToastOptions {
  style?: Toast.Style;
  title: string;
  message?: string;
  primaryAction?: Toast.ActionOptions;
  secondaryAction?: Toast.ActionOptions;
}

enum ToastStyle {
  Success = "success",
  Failure = "failure",
  Animated = "animated",
}

interface ToastActionOptions {
  title: string;
  shortcut?: Keyboard.Shortcut;
  onAction: () => void | Promise<void>;
}

class Toast {
  title: string;
  message?: string;
  style: Toast.Style;
  primaryAction?: Toast.ActionOptions;
  secondaryAction?: Toast.ActionOptions;
  
  hide(): Promise<void>;
  show(): Promise<void>;
}

// showHUD - brief overlay message
function showHUD(title: string, options?: { clearRootSearch?: boolean; popToRootType?: PopToRootType }): Promise<void>;
```

---

## OAuth

```typescript
// OAuth Provider setup (from @raycast/utils)
class OAuthService {
  static github(options: { scope: string; personalAccessToken?: string }): OAuthService;
  static google(options: { scope: string; personalAccessToken?: string }): OAuthService;
  static slack(options: { scope: string; personalAccessToken?: string }): OAuthService;
  static linear(options: { scope: string; personalAccessToken?: string }): OAuthService;
  static asana(options: { scope: string }): OAuthService;
  static jira(options: { scope: string }): OAuthService;
  static zoom(options: { scope: string }): OAuthService;
}

// Custom OAuth
interface OAuthServiceOptions {
  client: OAuth.PKCEClient;
  clientId: string;
  scope: string;
  authorizeUrl: string;
  tokenUrl: string;
  refreshTokenUrl?: string;
  personalAccessToken?: string;
  extraParameters?: Record<string, string>;
  bodyEncoding?: "json" | "url-encoded";
  tokenResponseParser?: (response: unknown) => OAuth.TokenResponse;
  tokenRefreshResponseParser?: (response: unknown) => OAuth.TokenResponse;
  onAuthorize?: (params: OAuth.AuthorizationRequest) => void | Promise<void>;
}

// HOC for authorized components
function withAccessToken<T>(
  service: OAuthService
): (Component: React.ComponentType<T>) => React.ComponentType<T>;

// Get token in component
function getAccessToken(): { token: string; type?: string; idToken?: string };
```

**PhotonCast OAuth Flow:**
1. Extension requests OAuth via IPC
2. PhotonCast opens system browser for auth
3. Redirect to `photoncast://oauth/callback`
4. PhotonCast exchanges code for token
5. Token stored in secure storage (Keychain)
6. Token returned to extension via IPC

---

## Environment & Preferences

```typescript
// Environment
const environment: {
  // Extension info
  commandName: string;
  commandMode: "view" | "no-view" | "menu-bar";
  extensionName: string;
  
  // Paths
  assetsPath: string;
  supportPath: string;
  
  // State
  launchType: LaunchType;
  launchContext?: LaunchContext;
  isDevelopment: boolean;
  
  // Appearance
  appearance: "light" | "dark";
  textSize: "medium" | "large";
  
  // Raycast info (stubbed in PhotonCast)
  raycastVersion: string;
  theme: Theme;
};

enum LaunchType {
  UserInitiated = "userInitiated",
  Background = "background",
}

// Preferences
function getPreferenceValues<T extends Record<string, unknown>>(): T;

// Preference types in manifest
type PreferenceType = 
  | "textfield"
  | "password"
  | "checkbox"
  | "dropdown"
  | "appPicker"
  | "file"
  | "directory";

interface Preference {
  name: string;
  type: PreferenceType;
  required: boolean;
  title: string;
  description: string;
  default?: unknown;
  placeholder?: string;
  data?: { title: string; value: string }[]; // for dropdown
}
```

---

## Images & Icons

```typescript
namespace Image {
  type ImageLike = 
    | string // URL or asset path
    | Icon
    | FileIcon
    | { source: string | Icon; mask?: Mask; fallback?: ImageLike; tintColor?: Color }
    | { fileIcon: string }
    | { light: string; dark: string };
  
  enum Mask {
    Circle = "circle",
    RoundedRectangle = "roundedRectangle",
  }
}

// Built-in icons
enum Icon {
  // Common
  ArrowRight,
  ArrowLeft,
  ArrowUp,
  ArrowDown,
  Checkmark,
  Circle,
  XmarkCircle,
  Plus,
  Minus,
  Trash,
  Pencil,
  
  // Files
  Document,
  Folder,
  FolderOpen,
  Image,
  Video,
  Music,
  
  // Apps
  AppWindow,
  Terminal,
  Globe,
  Link,
  
  // Actions
  Copy,
  Clipboard,
  Download,
  Upload,
  Refresh,
  
  // Status
  Star,
  StarFilled,
  Heart,
  HeartFilled,
  Bell,
  BellFilled,
  
  // Navigation
  ChevronRight,
  ChevronLeft,
  ChevronUp,
  ChevronDown,
  
  // ... 100+ more icons
}

// Colors
enum Color {
  Red = "red",
  Orange = "orange",
  Yellow = "yellow",
  Green = "green",
  Blue = "blue",
  Purple = "purple",
  Magenta = "magenta",
  
  PrimaryText = "primaryText",
  SecondaryText = "secondaryText",
}
```

---

## Utility Functions

```typescript
// Opening URLs & Files
function open(target: string, application?: Application | string): Promise<void>;
function openExtensionPreferences(): Promise<void>;
function openCommandPreferences(): Promise<void>;

// Get Applications
function getApplications(directory?: string): Promise<Application[]>;
function getDefaultApplication(path: string): Promise<Application>;
function getFrontmostApplication(): Promise<Application>; // macOS-specific

interface Application {
  name: string;
  path: string;
  bundleId?: string;
}

// Close Raycast (hide PhotonCast)
function closeMainWindow(options?: { clearRootSearch?: boolean; popToRootType?: PopToRootType }): Promise<void>;

function popToRoot(options?: { clearSearchBar?: boolean }): Promise<void>;

enum PopToRootType {
  Default = "default",
  Immediate = "immediate",
  Suspended = "suspended",
}

// Confirm Alert
function confirmAlert(options: Alert.Options): Promise<boolean>;

interface AlertOptions {
  title: string;
  message?: string;
  icon?: Image.ImageLike;
  primaryAction?: Alert.ActionOptions;
  dismissAction?: Alert.ActionOptions;
  rememberUserChoice?: boolean;
}
```

---

## Compatibility Notes

### Fully Supported (✅)
- List, Grid, Detail, Form components
- Action and ActionPanel
- All hooks (useNavigation, useCachedPromise, etc.)
- LocalStorage and Cache
- Clipboard API
- Toast and HUD
- Preferences
- Most Icons and Colors

### Partially Supported (⚠️)
- OAuth (basic flows work, some providers may need adjustment)
- Menu bar commands (system tray fallback)
- Application APIs (limited to installed apps)
- File system access (sandboxed)

### Not Supported (❌)
- `runAppleScript` - macOS only
- Deep system integration (Finder, Spotlight)
- Some macOS-specific icons
- AI features (by design)

---

## Implementation Priority

| Component | Priority | Complexity |
|-----------|----------|------------|
| List | P0 | Medium |
| Action/ActionPanel | P0 | Medium |
| Detail | P0 | Low |
| Form | P1 | High |
| Grid | P1 | Medium |
| LocalStorage | P0 | Low |
| Clipboard | P0 | Low |
| Toast/HUD | P0 | Low |
| useNavigation | P0 | Low |
| useCachedPromise | P1 | Medium |
| OAuth | P2 | High |
| Preferences | P1 | Medium |
