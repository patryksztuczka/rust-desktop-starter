use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use gpui::{
    AnyElement, App, Application, Bounds, Context, FocusHandle, KeyBinding, Menu, MenuItem,
    SharedString, SystemMenuType, Task, TitlebarOptions, Window, WindowBounds, WindowHandle,
    WindowOptions, actions, div, prelude::*, px, rgb, size,
};

use crate::desktop_ui::{self as ui, ButtonStyle};
use crate::login_item::{self, LoginItemStatus};
use crate::settings::Settings;
use crate::{APP_NAME, Pane, app_paths, sparkle};

actions!(
    starter,
    [
        CheckForUpdates,
        CloseWindow,
        HideApplication,
        MinimizeWindow,
        Quit,
        ShowAbout,
        ShowHome,
        ShowSettings,
        ToggleFullscreen,
    ]
);

const WINDOW_WIDTH: f32 = 920.0;
const WINDOW_HEIGHT: f32 = 620.0;

type MainWindow = Rc<RefCell<Option<WindowHandle<AppWindow>>>>;

/// Runs the GPUI application, opening the main window on `initial_pane`.
/// Returns when the app quits.
pub fn run(initial_pane: Pane) {
    let main_window = MainWindow::default();
    let application = Application::new();

    // Clicking the Dock icon with no open windows reopens the main window.
    let reopen = main_window.clone();
    application.on_reopen(move |cx| show_pane(&reopen, None, cx));

    application.run(move |cx: &mut App| {
        cx.bind_keys([
            KeyBinding::new("cmd-q", Quit, None),
            KeyBinding::new("cmd-h", HideApplication, None),
            KeyBinding::new("cmd-w", CloseWindow, None),
            KeyBinding::new("cmd-m", MinimizeWindow, None),
            KeyBinding::new("ctrl-cmd-f", ToggleFullscreen, None),
            KeyBinding::new("cmd-,", ShowSettings, None),
            KeyBinding::new("cmd-1", ShowHome, None),
            KeyBinding::new("cmd-2", ShowSettings, None),
            KeyBinding::new("cmd-3", ShowAbout, None),
        ]);

        // App-level actions work even when no window is open. Window-level
        // actions (close, minimize, full screen) live on the root element.
        cx.on_action(|_: &Quit, cx| cx.quit());
        cx.on_action(|_: &HideApplication, cx| cx.hide());
        cx.on_action(|_: &CheckForUpdates, cx| {
            cx.activate(true);
            sparkle::check_for_updates();
        });
        let window = main_window.clone();
        cx.on_action(move |_: &ShowHome, cx| show_pane(&window, Some(Pane::Home), cx));
        let window = main_window.clone();
        cx.on_action(move |_: &ShowSettings, cx| show_pane(&window, Some(Pane::Settings), cx));
        let window = main_window.clone();
        cx.on_action(move |_: &ShowAbout, cx| show_pane(&window, Some(Pane::About), cx));

        sparkle::start();
        cx.set_menus(menus());
        show_pane(&main_window, Some(initial_pane), cx);
    });
}

fn menus() -> Vec<Menu> {
    let mut app_items = vec![
        MenuItem::action(format!("About {APP_NAME}"), ShowAbout),
        MenuItem::separator(),
    ];
    if sparkle::is_available() {
        app_items.push(MenuItem::action("Check for Updates…", CheckForUpdates));
    }
    app_items.extend([
        MenuItem::action("Settings…", ShowSettings),
        MenuItem::separator(),
        MenuItem::os_submenu("Services", SystemMenuType::Services),
        MenuItem::separator(),
        MenuItem::action(format!("Hide {APP_NAME}"), HideApplication),
        MenuItem::separator(),
        MenuItem::action(format!("Quit {APP_NAME}"), Quit),
    ]);
    vec![
        Menu {
            name: APP_NAME.into(),
            items: app_items,
        },
        Menu {
            name: "File".into(),
            items: vec![MenuItem::action("Close Window", CloseWindow)],
        },
        Menu {
            name: "View".into(),
            items: vec![
                MenuItem::action("Home", ShowHome),
                MenuItem::action("Settings", ShowSettings),
                MenuItem::action("About", ShowAbout),
            ],
        },
        Menu {
            name: "Window".into(),
            items: vec![
                MenuItem::action("Minimize", MinimizeWindow),
                MenuItem::action("Enter Full Screen", ToggleFullscreen),
            ],
        },
    ]
}

/// Focuses the existing main window, or opens a new one, then optionally
/// switches it to `pane`.
fn show_pane(main_window: &MainWindow, pane: Option<Pane>, cx: &mut App) {
    let existing = main_window.borrow().as_ref().copied();
    if let Some(handle) = existing
        && handle
            .update(cx, |this, window, cx| {
                if let Some(pane) = pane {
                    this.select(pane, cx);
                }
                window.activate_window();
            })
            .is_ok()
    {
        cx.activate(true);
        return;
    }

    let bounds = Bounds::centered(None, size(px(WINDOW_WIDTH), px(WINDOW_HEIGHT)), cx);
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        titlebar: Some(TitlebarOptions {
            title: Some(APP_NAME.into()),
            appears_transparent: true,
            ..Default::default()
        }),
        window_min_size: Some(size(px(640.0), px(420.0))),
        ..Default::default()
    };
    match cx.open_window(options, |window, cx| {
        cx.new(|cx| AppWindow::new(pane.unwrap_or(Pane::Home), window, cx))
    }) {
        Ok(handle) => {
            *main_window.borrow_mut() = Some(handle);
            cx.activate(true);
        }
        Err(error) => tracing::error!(%error, "could not open the main window"),
    }
}

impl Pane {
    const ALL: [Pane; 3] = [Pane::Home, Pane::Settings, Pane::About];

    fn label(self) -> &'static str {
        match self {
            Pane::Home => "Home",
            Pane::Settings => "Settings",
            Pane::About => "About",
        }
    }

    fn symbol(self) -> &'static str {
        match self {
            Pane::Home => "house",
            Pane::Settings => "slider.horizontal.3",
            Pane::About => "info.circle",
        }
    }
}

enum Job {
    Idle,
    Running,
    Finished { primes: usize, elapsed: Duration },
}

enum LoginItem {
    Loading,
    Ready(LoginItemStatus),
    Failed(SharedString),
}

pub struct AppWindow {
    pane: Pane,
    settings: Settings,
    settings_error: Option<SharedString>,
    job: Job,
    login_item: LoginItem,
    focus_handle: FocusHandle,
    // Dropping a GPUI task cancels it, so in-flight work is owned here.
    _job_task: Option<Task<()>>,
    _login_item_task: Option<Task<()>>,
}

impl AppWindow {
    fn new(pane: Pane, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let (settings, settings_error) = match Settings::load() {
            Ok(settings) => (settings, None),
            Err(error) => {
                tracing::error!(%error, "could not load settings");
                (Settings::default(), Some(format!("{error}").into()))
            }
        };
        let focus_handle = cx.focus_handle();
        window.focus(&focus_handle);
        let mut this = Self {
            pane,
            settings,
            settings_error,
            job: Job::Idle,
            login_item: LoginItem::Loading,
            focus_handle,
            _job_task: None,
            _login_item_task: None,
        };
        this.request_login_item(None, cx);
        this
    }

    fn select(&mut self, pane: Pane, cx: &mut Context<Self>) {
        self.pane = pane;
        cx.notify();
    }

    /// Applies a change and persists it. Settings files are tiny, so saving
    /// inline is fine; move anything slow onto the background executor.
    fn update_settings(&mut self, cx: &mut Context<Self>, change: impl FnOnce(&mut Settings)) {
        change(&mut self.settings);
        self.settings_error = self.settings.save().err().map(|error| {
            tracing::error!(%error, "could not save settings");
            format!("Could not save settings: {error}").into()
        });
        cx.notify();
    }

    /// The pattern for any slow work: compute on the background executor,
    /// then hop back to the entity to update state and re-render.
    fn run_job(&mut self, cx: &mut Context<Self>) {
        if matches!(self.job, Job::Running) {
            return;
        }
        self.job = Job::Running;
        cx.notify();
        self._job_task = Some(cx.spawn(async move |this, cx| {
            let started = Instant::now();
            let primes = cx
                .background_executor()
                .spawn(async { count_primes(20_000_000) })
                .await;
            let elapsed = started.elapsed();
            this.update(cx, |this, cx| {
                this.job = Job::Finished { primes, elapsed };
                cx.notify();
            })
            .ok();
        }));
    }

    /// `None` refreshes the status; `Some(enabled)` registers or unregisters.
    fn request_login_item(&mut self, enabled: Option<bool>, cx: &mut Context<Self>) {
        self._login_item_task = Some(cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    match enabled {
                        Some(enabled) => login_item::set_enabled(enabled),
                        None => login_item::status(),
                    }
                })
                .await;
            this.update(cx, |this, cx| {
                this.login_item = match result {
                    Ok(status) => LoginItem::Ready(status),
                    Err(error) => LoginItem::Failed(format!("{error}").into()),
                };
                cx.notify();
            })
            .ok();
        }));
    }

    fn render_sidebar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        ui::sidebar().children(Pane::ALL.map(|pane| {
            ui::navigation_item(pane.label(), pane.symbol(), pane.label(), self.pane == pane)
                .on_click(cx.listener(move |this, _, _, cx| this.select(pane, cx)))
        }))
    }

    fn render_home(&self, cx: &mut Context<Self>) -> AnyElement {
        let counter = ui::panel().child(ui::setting_row(
            "Saved counter",
            "Stored in settings.json, so it survives restarts.",
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(
                    ui::button("decrement", "−", ButtonStyle::Secondary, true).on_click(
                        cx.listener(|this, _, _, cx| {
                            this.update_settings(cx, |settings| settings.counter -= 1)
                        }),
                    ),
                )
                .child(
                    div()
                        .min_w(px(48.0))
                        .flex()
                        .justify_center()
                        .text_size(px(16.0))
                        .child(self.settings.counter.to_string()),
                )
                .child(
                    ui::button("increment", "+", ButtonStyle::Secondary, true).on_click(
                        cx.listener(|this, _, _, cx| {
                            this.update_settings(cx, |settings| settings.counter += 1)
                        }),
                    ),
                ),
        ));

        let running = matches!(self.job, Job::Running);
        let job_detail: SharedString = match &self.job {
            Job::Idle => "Counts primes below 20 million without blocking the UI.".into(),
            Job::Running => "Working on the background executor…".into(),
            Job::Finished { primes, elapsed } => {
                format!("Found {primes} primes in {} ms.", elapsed.as_millis()).into()
            }
        };
        let job = ui::panel().child(ui::setting_row(
            "Background work",
            job_detail,
            ui::button(
                "run-job",
                if running { "Running…" } else { "Run" },
                ButtonStyle::Primary,
                !running,
            )
            .on_click(cx.listener(|this, _, _, cx| this.run_job(cx))),
        ));

        let tips = self.settings.show_tips.then(|| {
            ui::panel().p_4().gap_2().children([
                div()
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .child("Where to start")
                    .into_any_element(),
                tip("Add panes in src/app_window.rs; style them with src/desktop_ui.rs."),
                tip("Put app logic in plain modules with no GPUI dependency, so it is easy to test."),
                tip("Run `./scripts/install-app.sh` to try the packaged app from /Applications."),
            ])
        });

        ui::pane("home", "Home", "A starting point for your app.")
            .child(
                ui::pane_body()
                    .child(counter)
                    .child(job)
                    .children(tips)
                    .children(
                        self.settings_error
                            .clone()
                            .map(|e| ui::status_text(e, false)),
                    ),
            )
            .into_any_element()
    }

    fn render_settings(&self, cx: &mut Context<Self>) -> AnyElement {
        let show_tips = self.settings.show_tips;
        let general = ui::panel().child(ui::setting_row(
            "Show tips on Home",
            "An example of a persisted preference.",
            ui::toggle("show-tips", show_tips).on_click(cx.listener(move |this, _, _, cx| {
                this.update_settings(cx, |settings| settings.show_tips = !show_tips)
            })),
        ));

        let (enabled, detail): (bool, SharedString) = match &self.login_item {
            LoginItem::Loading => (false, "Checking…".into()),
            LoginItem::Ready(LoginItemStatus::Enabled) => (true, "Opens when you log in.".into()),
            LoginItem::Ready(LoginItemStatus::Disabled) => {
                (false, "Requires the signed app in /Applications.".into())
            }
            LoginItem::Ready(LoginItemStatus::RequiresApproval) => (
                true,
                "Waiting for approval in System Settings → Login Items.".into(),
            ),
            LoginItem::Failed(error) => (false, error.clone()),
        };
        let mut startup = ui::panel().child(ui::setting_row(
            "Open at login",
            detail,
            ui::toggle("login-item", enabled).on_click(cx.listener(move |this, _, _, cx| {
                this.login_item = LoginItem::Loading;
                this.request_login_item(Some(!enabled), cx);
                cx.notify();
            })),
        ));
        if matches!(
            self.login_item,
            LoginItem::Ready(LoginItemStatus::RequiresApproval)
        ) {
            startup = startup.child(ui::divider()).child(ui::setting_row(
                "Approve login item",
                "macOS asks you to confirm new login items.",
                ui::button(
                    "open-login-items",
                    "Open System Settings",
                    ButtonStyle::Secondary,
                    true,
                )
                .on_click(|_, _, _| login_item::open_system_settings()),
            ));
        }

        ui::pane(
            "settings",
            "Settings",
            "Preferences are saved as you change them.",
        )
        .child(
            ui::pane_body().child(general).child(startup).children(
                self.settings_error
                    .clone()
                    .map(|e| ui::status_text(e, false)),
            ),
        )
        .into_any_element()
    }

    fn render_about(&self) -> AnyElement {
        let data_dir = app_paths::support_dir().ok();
        let data_detail: SharedString = data_dir
            .as_ref()
            .map(|path| path.display().to_string().into())
            .unwrap_or_else(|| "Unavailable".into());
        let updates_available = sparkle::is_available();

        let info = ui::panel()
            .child(ui::setting_row(
                "Version",
                format!("{APP_NAME} {}", env!("CARGO_PKG_VERSION")),
                div(),
            ))
            .child(ui::divider())
            .child(ui::setting_row(
                "Data folder",
                data_detail,
                ui::button(
                    "reveal-data",
                    "Show in Finder",
                    ButtonStyle::Secondary,
                    data_dir.is_some(),
                )
                .on_click(move |_, _, cx| {
                    if let Some(path) = &data_dir {
                        cx.reveal_path(path);
                    }
                }),
            ))
            .child(ui::divider())
            .child(ui::setting_row(
                "Updates",
                if updates_available {
                    "Checks automatically through Sparkle."
                } else {
                    "Not configured for this build. See README → Updates."
                },
                ui::button(
                    "check-updates",
                    "Check for Updates…",
                    ButtonStyle::Secondary,
                    updates_available,
                )
                .on_click(move |_, window, cx| {
                    if updates_available {
                        window.dispatch_action(Box::new(CheckForUpdates), cx);
                    }
                }),
            ));

        ui::pane("about", "About", "Build and storage details.")
            .child(ui::pane_body().child(info))
            .into_any_element()
    }
}

impl Render for AppWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let content = match self.pane {
            Pane::Home => self.render_home(cx),
            Pane::Settings => self.render_settings(cx),
            Pane::About => self.render_about(),
        };
        ui::window_frame()
            .track_focus(&self.focus_handle)
            .on_action(|_: &CloseWindow, window, _| window.remove_window())
            .on_action(|_: &MinimizeWindow, window, _| window.minimize_window())
            .on_action(|_: &ToggleFullscreen, window, _| window.toggle_fullscreen())
            .child(self.render_sidebar(cx))
            .child(content)
    }
}

fn tip(text: &'static str) -> AnyElement {
    div()
        .flex()
        .gap_2()
        .text_color(rgb(ui::TEXT_SOFT))
        .child(div().text_color(rgb(ui::MUTED)).child("•"))
        .child(text)
        .into_any_element()
}

/// Deliberately CPU-heavy stand-in for real work.
fn count_primes(limit: usize) -> usize {
    let mut composite = vec![false; limit];
    let mut count = 0;
    for n in 2..limit {
        if !composite[n] {
            count += 1;
            for multiple in (n * n..limit).step_by(n) {
                composite[multiple] = true;
            }
        }
    }
    count
}

#[cfg(test)]
mod tests {
    use super::count_primes;

    #[test]
    fn counts_primes() {
        assert_eq!(count_primes(10), 4);
        assert_eq!(count_primes(100), 25);
    }
}
