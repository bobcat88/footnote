# Windows: native windows and WebView2

Browser gates cover a web page. A Win32 window, or a WebView2 or Tauri app in its real host, needs these checks in the final pass.

## Capture
- Capture the real window, not a browser stand-in: Windows.Graphics.Capture (per window, includes DirectComposition content) or `PrintWindow` with `PW_RENDERFULLCONTENT` for a quick check. `BitBlt` from the window's own DC misses DirectComposition and GPU-rendered content.
- Record display scale and resolution with each shot; `deviceScaleFactor` in a browser doesn't stand for it.
- Test at the scales the product supports (100% and 150% at least on a 1920×1080 panel) and across a move between monitors with different scales.

## DPI
- The process declares Per-Monitor V2 awareness (manifest `dpiAwareness` = `PerMonitorV2`), handles `WM_DPICHANGED` by resizing to the suggested rectangle, and scales fonts, icons and hit areas from `GetDpiForWindow`.
- Failure tells: blurry text after moving monitors (bitmap stretching), clipped labels at 150%, 1 px borders that become 2 px on one side.

## System settings to honour
| Setting | Read it with | Expect |
|---|---|---|
| Animation effects off | `SystemParametersInfo(SPI_GETCLIENTAREAANIMATION)`, `UISettings.AnimationsEnabled` | Movement off or reduced; WebView2 content sees `prefers-reduced-motion` |
| Transparency effects off | `UISettings.AdvancedEffectsEnabled` | Blur and translucency replaced by solid surfaces |
| Contrast themes | `SystemParametersInfo(SPI_GETHIGHCONTRAST)`, `forced-colors` in CSS | System colours used; no information carried by colour alone; focus still visible |
| Dark or light app mode | `UISettings.GetColorValue(Background)` or the registry `AppsUseLightTheme` | Window frame and content follow it (`DWMWA_USE_IMMERSIVE_DARK_MODE` for the title bar) |
| Text size (Accessibility > Text size) | `UISettings.TextScaleFactor` | Text grows without clipping |

## Accessibility
- Run Accessibility Insights for Windows (FastPass) on each main window: every control has a UI Automation name, role and keyboard path.
- Keyboard: Tab and arrow navigation, Alt access keys on menus, Escape closes popups, focus returns to the invoker.
- Narrator reads the window title, then the focused control's name and state.

## Window behaviour
- Snap layouts, minimise, maximise and restore work; a custom title bar keeps a draggable region and system menu (Alt+Space).
- Minimum size is enforced (`WM_GETMINMAXINFO`) and the layout holds at it.
- Size and position are restored on next launch, clamped to a monitor that still exists.

## Sources
[High DPI desktop apps](https://learn.microsoft.com/en-us/windows/win32/hidpi/high-dpi-desktop-application-development-on-windows) · [WM_DPICHANGED](https://learn.microsoft.com/en-us/windows/win32/hidpi/wm-dpichanged) · [SystemParametersInfo](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-systemparametersinfow) · [UISettings](https://learn.microsoft.com/en-us/uwp/api/windows.ui.viewmanagement.uisettings) · [Contrast themes](https://learn.microsoft.com/en-us/windows/apps/design/accessibility/high-contrast-themes) · [forced-colors (MDN)](https://developer.mozilla.org/en-US/docs/Web/CSS/@media/forced-colors) · [Screen capture](https://learn.microsoft.com/en-us/windows/uwp/audio-video-camera/screen-capture) · [PrintWindow](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-printwindow) · [DwmSetWindowAttribute](https://learn.microsoft.com/en-us/windows/win32/api/dwmapi/nf-dwmapi-dwmsetwindowattribute) · [Accessibility Insights for Windows](https://accessibilityinsights.io/docs/windows/overview/) · [Title bar customization](https://learn.microsoft.com/en-us/windows/apps/develop/title-bar)
