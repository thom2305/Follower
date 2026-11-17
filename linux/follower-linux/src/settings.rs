// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Thom (thom2305)

use crate::{ui, App};
use adw::prelude::*;
use follower_core::Config;
use gtk::gio;
use std::rc::Rc;

pub fn build(app: &Rc<App>) -> gtk::Widget {
    let c = app.cfg.read().unwrap().clone();

    let url = adw::EntryRow::builder().title("Server URL").text(c.url.as_str()).build();
    let token = adw::PasswordEntryRow::builder().title("API token").text(c.token.as_str()).build();
    let tz = adw::EntryRow::builder().title("Timezone (e.g. America/Toronto)").text(c.timezone.as_str()).build();

    let group = adw::PreferencesGroup::builder()
        .title("Bot server")
        .description(ui::esc(
            "e.g. https://follower.example.com or http://100.x.y.z:8080. The token is saved in a file only \
             your user can read (~/.config/follower). Prefer an https:// URL, or a private network such as \
             Tailscale, since http:// sends the token unencrypted.",
        ))
        .build();
    group.add(&url);
    group.add(&token);
    group.add(&tz);

    let test = gtk::Button::builder().label("Save & test").css_classes(["suggested-action", "pill"]).build();
    let save = gtk::Button::builder().label("Save").css_classes(["pill"]).build();
    let buttons = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    buttons.set_halign(gtk::Align::Center);
    buttons.append(&test);
    buttons.append(&save);
    let actions = adw::PreferencesGroup::new();
    actions.add(&buttons);

    let current = {
        let (url, token, tz) = (url.clone(), token.clone(), tz.clone());
        move || Config { url: url.text().into(), token: token.text().into(), timezone: tz.text().into() }
    };
    {
        let app_test = app.clone();
        let cur_test = current.clone();
        test.connect_clicked(move |_| app_test.save_config(cur_test(), true));

        let app_save = app.clone();
        let cur_save = current.clone();
        save.connect_clicked(move |_| app_save.save_config(cur_save(), false));
    }

    let about = adw::PreferencesGroup::new();
    let about_btn = gtk::Button::builder().label("About Follower").css_classes(["pill"]).halign(gtk::Align::Center).build();
    {
        let app_about = app.clone();
        about_btn.connect_clicked(move |_| show_about(&app_about));
    }
    about.add(&about_btn);

    let page = adw::PreferencesPage::new();
    page.add(&group);
    page.add(&actions);
    page.add(&about);
    page.upcast()
}

const REPO_URL: &str = "https://github.com/thom2305/Follower";

fn show_about(app: &Rc<App>) {
    let body = gtk::Box::new(gtk::Orientation::Vertical, 12);
    body.set_valign(gtk::Align::Center);
    ui::margins(&body, 24);

    if let Ok(tex) = gtk::gdk::Texture::from_bytes(&gtk::glib::Bytes::from_static(crate::ICON_PNG)) {
        let img = gtk::Image::from_paintable(Some(&tex));
        img.set_pixel_size(128);
        img.set_margin_bottom(6);
        body.append(&img);
    }
    let centered = |text: &str, classes: &[&str]| {
        let l = gtk::Label::builder().label(text).halign(gtk::Align::Center).justify(gtk::Justification::Center).wrap(true).build();
        for c in classes {
            l.add_css_class(c);
        }
        l
    };
    body.append(&centered("Follower", &["title-1"]));
    body.append(&centered("Thom (thom2305)", &["dim-label"]));
    body.append(&centered(&format!("Version {}", env!("CARGO_PKG_VERSION")), &["caption", "dim-label"]));

    let win = adw::Window::builder()
        .transient_for(&app.window)
        .modal(true)
        .default_width(420)
        .default_height(560)
        .title("About Follower")
        .build();

    let group = adw::PreferencesGroup::new();
    group.set_margin_top(18);
    group.add(&link_row(&win, "Source code", REPO_URL));
    group.add(&link_row(&win, "Report an issue", &format!("{REPO_URL}/issues")));
    let legal = adw::ActionRow::builder().title("Legal and licenses").activatable(true).build();
    legal.add_suffix(&gtk::Image::from_icon_name("go-next-symbolic"));
    {
        let win2 = win.clone();
        legal.connect_activated(move |_| show_legal(&win2));
    }
    group.add(&legal);
    body.append(&group);

    let scroll = gtk::ScrolledWindow::builder().hscrollbar_policy(gtk::PolicyType::Never).vexpand(true).child(&body).build();
    let tv = adw::ToolbarView::new();
    tv.add_top_bar(&adw::HeaderBar::builder().show_title(false).build());
    tv.set_content(Some(&scroll));
    win.set_content(Some(&tv));
    win.present();
}

fn link_row(win: &adw::Window, title: &str, url: &str) -> adw::ActionRow {
    let row = adw::ActionRow::builder().title(title).activatable(true).build();
    row.add_suffix(&gtk::Image::from_icon_name("adw-external-link-symbolic"));
    let (win, url) = (win.clone(), url.to_owned());
    row.connect_activated(move |_| {
        gtk::UriLauncher::new(&url).launch(Some(&win), None::<&gio::Cancellable>, |_| {});
    });
    row
}

fn show_legal(parent: &adw::Window) {
    let text = format!(
        "Follower is licensed under the Apache License 2.0.\n\n{}\n\n{}\n\n{}",
        include_str!("../../LICENSE"),
        "=".repeat(78),
        include_str!("../../THIRD_PARTY_NOTICES.txt"),
    );
    let view = gtk::TextView::builder()
        .editable(false)
        .cursor_visible(false)
        .monospace(true)
        .wrap_mode(gtk::WrapMode::WordChar)
        .top_margin(12)
        .bottom_margin(12)
        .left_margin(12)
        .right_margin(12)
        .build();
    view.buffer().set_text(&text);
    let scroll = gtk::ScrolledWindow::builder().vexpand(true).child(&view).build();
    let tv = adw::ToolbarView::new();
    tv.add_top_bar(&adw::HeaderBar::new());
    tv.set_content(Some(&scroll));
    let w = adw::Window::builder()
        .transient_for(parent)
        .modal(true)
        .default_width(680)
        .default_height(560)
        .title("Legal and licenses")
        .content(&tv)
        .build();
    w.present();
}
