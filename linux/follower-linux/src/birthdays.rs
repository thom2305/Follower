// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Thom (thom2305)

use crate::{ui, App};
use adw::prelude::*;
use follower_core::*;
use std::rc::Rc;

pub fn build(app: &Rc<App>) -> gtk::Widget {
    let title = ui::label("Next 60 days", &["heading"]);
    let scale = gtk::Scale::with_range(gtk::Orientation::Horizontal, 7.0, 366.0, 1.0);
    scale.set_value(60.0);
    scale.set_hexpand(true);
    let list = gtk::ListBox::builder().selection_mode(gtk::SelectionMode::None).css_classes(["boxed-list"]).build();
    let empty = ui::label("No birthdays in that window.", &["dim-label"]);

    let root = gtk::Box::new(gtk::Orientation::Vertical, 12);
    ui::margins(&root, 12);
    for w in [title.upcast_ref::<gtk::Widget>(), scale.upcast_ref(), list.upcast_ref(), empty.upcast_ref()] {
        root.append(w);
    }

    let render: Rc<dyn Fn()> = {
        let (app, scale, title, list, empty) = (app.clone(), scale.clone(), title.clone(), list.clone(), empty.clone());
        Rc::new(move || {
            let days = scale.value() as i64;
            title.set_label(&format!("Next {days} days"));
            let zone = app.zone();
            let mut upcoming: Vec<(i64, PersonEntry)> = app
                .people
                .borrow()
                .iter()
                .filter_map(|e| {
                    let (mo, d) = (e.person.birthday_month?, e.person.birthday_day?);
                    let n = days_until_birthday(mo, d, zone);
                    (n <= days).then(|| (n, e.clone()))
                })
                .collect();
            upcoming.sort_by_key(|u| u.0);

            ui::clear_list(&list);
            for (_, e) in &upcoming {
                let (mo, d) = (e.person.birthday_month.unwrap(), e.person.birthday_day.unwrap());
                let row = adw::ActionRow::builder()
                    .title(ui::esc(&e.name()))
                    .subtitle(ui::esc(&fmt_birthday(mo, d, e.person.birthday_year, zone)))
                    .activatable(true)
                    .build();
                row.add_prefix(&ui::avatar(e.profile.as_ref().and_then(|p| p.avatar.as_deref()), &e.name(), 40));
                row.add_suffix(&gtk::Image::from_icon_name("go-next-symbolic"));
                let (app2, id) = (app.clone(), e.person.user_id);
                row.connect_activated(move |_| app2.open_person(id));
                list.append(&row);
            }
            list.set_visible(!upcoming.is_empty());
            empty.set_visible(upcoming.is_empty());
        })
    };

    let r = render.clone();
    scale.connect_value_changed(move |_| r());
    let r = render.clone();
    app.on_people_changed(move || r());
    render();
    ui::scrolled(&root).upcast()
}
