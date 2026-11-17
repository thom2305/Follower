// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Thom (thom2305)

mod archive;
mod birthdays;
mod lookup;
mod people;
mod person;
mod settings;
mod ui;

use adw::prelude::*;
use follower_core::*;
use gtk::glib;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::{Arc, RwLock};

const APP_ID: &str = "ca.thoms.follower";

pub struct App {
    pub window: adw::ApplicationWindow,
    pub nav: adw::NavigationView,
    pub stack: adw::ViewStack,
    toasts: adw::ToastOverlay,
    pub repo: Repo,
    pub cfg: Arc<RwLock<Config>>,
    store: ConfigStore,
    pub people: RefCell<Vec<PersonEntry>>,
    pub tags: RefCell<Vec<TagCount>>,
    pub loading: Cell<bool>,
    pub progress: RefCell<Option<String>>,
    listeners: RefCell<Vec<Box<dyn Fn()>>>,
}

impl App {
    pub fn toast(&self, msg: &str) {
        let t = adw::Toast::builder().title(ui::esc(msg)).build();
        self.toasts.add_toast(t);
    }

    pub fn zone(&self) -> Tz {
        self.cfg.read().unwrap().zone()
    }

    pub fn ready(&self) -> bool {
        self.cfg.read().unwrap().ready()
    }

    pub fn on_people_changed(&self, f: impl Fn() + 'static) {
        self.listeners.borrow_mut().push(Box::new(f));
    }

    pub fn notify(&self) {
        for f in self.listeners.borrow().iter() {
            f();
        }
    }

    pub fn task<T: Send + 'static>(
        self: &Rc<Self>,
        work: impl FnOnce() -> Result<T, ApiError> + Send + 'static,
        done: impl FnOnce(T) + 'static,
    ) {
        self.task_or(work, done, || {});
    }

    pub fn task_or<T: Send + 'static>(
        self: &Rc<Self>,
        work: impl FnOnce() -> Result<T, ApiError> + Send + 'static,
        done: impl FnOnce(T) + 'static,
        fail: impl FnOnce() + 'static,
    ) {
        let app = self.clone();
        ui::run(
            move || work().map_err(|e| e.0),
            move |r| match r {
                Ok(v) => done(v),
                Err(e) => {
                    fail();
                    app.toast(&e);
                }
            },
        );
    }

    pub fn load_people(self: &Rc<Self>) {
        self.loading.set(true);
        self.notify();
        let repo = self.repo.clone();
        let app = self.clone();
        ui::run(
            move || {
                let p = repo.list_people().map_err(|e| e.0)?;
                let t = repo.all_tags().map_err(|e| e.0)?;
                Ok((p, t))
            },
            move |r| {
                app.loading.set(false);
                match r {
                    Ok((p, t)) => {
                        *app.people.borrow_mut() = p;
                        *app.tags.borrow_mut() = t;
                    }
                    Err(e) => app.toast(&e),
                }
                app.notify();
            },
        );
    }

    pub fn refresh_all(self: &Rc<Self>, ids: Vec<i64>) {
        if ids.is_empty() || self.progress.borrow().is_some() {
            return;
        }
        self.refresh_step(Rc::new(ids), 0, 0, 0);
    }

    fn refresh_step(self: &Rc<Self>, ids: Rc<Vec<i64>>, i: usize, changed: usize, gone: usize) {
        if i >= ids.len() {
            *self.progress.borrow_mut() = None;
            self.load_people();
            let gone_txt = if gone > 0 { format!(" · {gone} not found") } else { String::new() };
            self.toast(&format!("Refreshed {} · {} changed{}", ids.len(), changed, gone_txt));
            return;
        }
        *self.progress.borrow_mut() = Some(format!("Refreshing {}/{}…", i + 1, ids.len()));
        self.notify();
        let repo = self.repo.clone();
        let id = ids[i];
        let app = self.clone();
        ui::run(
            move || repo.take_snapshot(id).map_err(|e| e.0),
            move |r| match r {
                Ok(s) => {
                    let (c, g) = if s.profile.is_none() {
                        (changed, gone + 1)
                    } else if !s.changes.is_empty() {
                        (changed + 1, gone)
                    } else {
                        (changed, gone)
                    };
                    app.refresh_step(ids, i + 1, c, g);
                }
                Err(e) => {
                    *app.progress.borrow_mut() = None;
                    app.notify();
                    app.toast(&e);
                }
            },
        );
    }

    pub fn save_config(self: &Rc<Self>, mut c: Config, test: bool) {
        c.url = c.url.trim().to_owned();
        c.token = c.token.trim().to_owned();
        c.timezone = c.timezone.trim().to_owned();
        if let Err(e) = self.store.save(&c) {
            self.toast(&format!("Couldn't save settings: {e}"));
            return;
        }
        *self.cfg.write().unwrap() = c;
        if test {
            let repo = self.repo.clone();
            let app = self.clone();
            self.task(
                move || repo.counts(),
                move |(p, a)| {
                    app.toast(&format!("Connected: {p} people, {a} archived messages"));
                    app.load_people();
                },
            );
        } else {
            self.toast("Saved");
        }
    }

    pub fn open_person(self: &Rc<Self>, id: i64) {
        let page = person::page(self, id);
        self.nav.push(&page);
    }
}

const ICON_PNG: &[u8] = include_bytes!("../assets/ca.thoms.follower.png");

fn write_if_changed(path: &std::path::Path, bytes: &[u8]) {
    if std::fs::read(path).map(|old| old == bytes).unwrap_or(false) {
        return;
    }
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(path, bytes);
}

fn install_icon() {
    let base = glib::user_cache_dir().join("follower").join("icons");
    write_if_changed(&base.join("hicolor/512x512/apps/ca.thoms.follower.png"), ICON_PNG);
    if let Some(display) = gtk::gdk::Display::default() {
        gtk::IconTheme::for_display(&display).add_search_path(&base);
    }
    gtk::Window::set_default_icon_name(APP_ID);

    if std::env::var_os("FLATPAK_ID").is_some() || std::env::var_os("FOLLOWER_NO_INSTALL").is_some() {
        return;
    }
    let exe = std::env::var_os("APPIMAGE")
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::current_exe().ok());
    let Some(exe) = exe else { return };
    let exe = exe.to_string_lossy().replace('\\', "\\\\").replace('"', "\\\"");

    let data = glib::user_data_dir();
    write_if_changed(&data.join("icons/hicolor/512x512/apps/ca.thoms.follower.png"), ICON_PNG);
    let desktop = format!(
        "[Desktop Entry]\nType=Application\nName=Follower\n\
         Comment=Track and archive people from your Discord bot\n\
         Exec=\"{exe}\"\nIcon=ca.thoms.follower\nCategories=Network;Utility;\n\
         Terminal=false\nStartupWMClass=ca.thoms.follower\n"
    );
    write_if_changed(&data.join("applications/ca.thoms.follower.desktop"), desktop.as_bytes());
}

fn build(application: &adw::Application) {
    install_icon();
    let window = adw::ApplicationWindow::builder()
        .application(application)
        .title("Follower")
        .default_width(900)
        .default_height(720)
        .width_request(420)
        .build();

    let store = ConfigStore::new();
    let config = store.load();
    let ready = config.ready();
    let cfg = Arc::new(RwLock::new(config));
    let repo = Repo::new(Api::new(cfg.clone()));

    let app = Rc::new(App {
        window: window.clone(),
        nav: adw::NavigationView::new(),
        stack: adw::ViewStack::new(),
        toasts: adw::ToastOverlay::new(),
        repo,
        cfg,
        store,
        people: RefCell::new(vec![]),
        tags: RefCell::new(vec![]),
        loading: Cell::new(false),
        progress: RefCell::new(None),
        listeners: RefCell::new(vec![]),
    });

    let tabs: [(&str, &str, &str, gtk::Widget); 5] = [
        ("people", "People", "system-users-symbolic", people::build(&app)),
        ("lookup", "Lookup", "system-search-symbolic", lookup::build(&app)),
        ("birthdays", "Birthdays", "x-office-calendar-symbolic", birthdays::build(&app)),
        ("archive", "Archive", "folder-symbolic", archive::build(&app)),
        ("settings", "Settings", "emblem-system-symbolic", settings::build(&app)),
    ];
    for (name, title, icon, widget) in tabs {
        app.stack.add_titled_with_icon(&widget, Some(name), title, icon);
    }

    let header = adw::HeaderBar::new();
    let switcher = adw::ViewSwitcher::builder().stack(&app.stack).policy(adw::ViewSwitcherPolicy::Wide).build();
    header.set_title_widget(Some(&switcher));

    let bottom = adw::ViewSwitcherBar::builder().stack(&app.stack).build();
    let breakpoint = adw::Breakpoint::new(adw::BreakpointCondition::new_length(
        adw::BreakpointConditionLengthType::MaxWidth,
        560.0,
        adw::LengthUnit::Sp,
    ));
    breakpoint.add_setter(&bottom, "reveal", Some(&true.to_value()));
    breakpoint.add_setter(&switcher, "visible", Some(&false.to_value()));
    window.add_breakpoint(breakpoint);

    let tv = adw::ToolbarView::new();
    tv.add_top_bar(&header);
    tv.add_bottom_bar(&bottom);
    tv.set_content(Some(&app.stack));

    let root = adw::NavigationPage::builder().title("Follower").child(&tv).build();
    app.nav.add(&root);
    app.toasts.set_child(Some(&app.nav));
    window.set_content(Some(&app.toasts));

    if !ready {
        app.stack.set_visible_child_name("settings");
    }
    window.present();
    if ready {
        app.load_people();
    }
}

fn main() -> glib::ExitCode {
    let application = adw::Application::builder().application_id(APP_ID).build();
    application.connect_activate(build);
    application.run()
}
