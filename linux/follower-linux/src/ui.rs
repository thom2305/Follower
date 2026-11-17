// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Thom (thom2305)

use crate::App;
use adw::prelude::*;
use gtk::{gdk, gio, glib};
use std::{cell::RefCell, collections::HashMap, rc::Rc};

pub fn run<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, String> + Send + 'static,
    done: impl FnOnce(Result<T, String>) + 'static,
) {
    glib::spawn_future_local(async move {
        let r = match gio::spawn_blocking(work).await {
            Ok(r) => r,
            Err(_) => Err("Background task failed".to_string()),
        };
        done(r);
    });
}

pub fn esc(s: &str) -> String {
    glib::markup_escape_text(s).to_string()
}

pub fn trunc(s: &str, n: usize) -> String {
    if s.chars().count() > n {
        format!("{}…", s.chars().take(n).collect::<String>())
    } else {
        s.to_owned()
    }
}

pub fn margins(w: &impl IsA<gtk::Widget>, m: i32) {
    w.set_margin_top(m);
    w.set_margin_bottom(m);
    w.set_margin_start(m);
    w.set_margin_end(m);
}

pub fn clear_box(b: &gtk::Box) {
    while let Some(c) = b.first_child() {
        b.remove(&c);
    }
}

pub fn clear_list(l: &gtk::ListBox) {
    while let Some(c) = l.first_child() {
        l.remove(&c);
    }
}

pub fn dropdown(labels: &[&str]) -> gtk::DropDown {
    gtk::DropDown::new(Some(gtk::StringList::new(labels)), gtk::Expression::NONE)
}

pub fn scrolled(child: &impl IsA<gtk::Widget>) -> gtk::ScrolledWindow {
    let clamp = adw::Clamp::builder().maximum_size(720).tightening_threshold(560).child(child).build();
    gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vexpand(true)
        .child(&clamp)
        .build()
}

pub fn prop_row(title: &str, value: &str) -> adw::ActionRow {
    let row = adw::ActionRow::builder().title(esc(title)).subtitle(esc(value)).build();
    row.add_css_class("property");
    row.set_subtitle_selectable(true);
    row
}

pub fn label(text: &str, classes: &[&str]) -> gtk::Label {
    let l = gtk::Label::builder().label(text).halign(gtk::Align::Start).xalign(0.0).wrap(true).build();
    for c in classes {
        l.add_css_class(c);
    }
    l
}

pub fn heading(text: &str) -> gtk::Label {
    label(text, &["heading"])
}

pub fn confirm(app: &Rc<App>, heading: &str, body: &str, action: &str, destructive: bool, on_ok: impl Fn() + 'static) {
    let dlg = adw::MessageDialog::new(Some(&app.window), Some(heading), Some(body));
    dlg.add_responses(&[("cancel", "Cancel"), ("ok", action)]);
    dlg.set_response_appearance(
        "ok",
        if destructive { adw::ResponseAppearance::Destructive } else { adw::ResponseAppearance::Suggested },
    );
    dlg.set_close_response("cancel");
    dlg.connect_response(None, move |_, resp| {
        if resp == "ok" {
            on_ok();
        }
    });
    dlg.present();
}

thread_local! {
    static CACHE: RefCell<HashMap<String, gdk::Texture>> = RefCell::new(HashMap::new());
}

fn cdn_png(url: &str, size: i32) -> String {
    if !(url.contains("discordapp.") || url.contains("discord.com")) {
        return url.to_owned();
    }
    let base = url.split('?').next().unwrap_or(url);
    let base = base
        .strip_suffix(".webp")
        .or_else(|| base.strip_suffix(".gif"))
        .map(|b| format!("{b}.png"))
        .unwrap_or_else(|| base.to_owned());
    format!("{base}?size={}", (size.max(16) as u32 * 2).next_power_of_two().min(1024))
}

pub fn load_image(url: String, done: impl FnOnce(gdk::Texture) + 'static) {
    if let Some(t) = CACHE.with(|c| c.borrow().get(&url).cloned()) {
        done(t);
        return;
    }
    let u = url.clone();
    run(
        move || follower_core::fetch_url(&u).map_err(|e| e.0),
        move |r| {
            if let Ok(bytes) = r {
                if let Ok(tex) = gdk::Texture::from_bytes(&glib::Bytes::from_owned(bytes)) {
                    CACHE.with(|c| c.borrow_mut().insert(url, tex.clone()));
                    done(tex);
                }
            }
        },
    );
}

pub fn avatar(url: Option<&str>, name: &str, size: i32) -> adw::Avatar {
    let a = adw::Avatar::new(size, Some(name), true);
    if let Some(u) = url {
        let a2 = a.clone();
        load_image(cdn_png(u, size), move |t| a2.set_custom_image(Some(&t)));
    }
    a
}
