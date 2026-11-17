// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Thom (thom2305)

use crate::{ui, App};
use adw::prelude::*;
use follower_core::*;
use std::cell::RefCell;
use std::rc::Rc;

pub fn build(app: &Rc<App>) -> gtk::Widget {
    let search = gtk::SearchEntry::builder()
        .placeholder_text("Search name, @username, ID, notes")
        .hexpand(true)
        .build();
    let rel_labels: Vec<&str> = std::iter::once("All statuses").chain(STATUSES.iter().map(|s| s.1)).collect();
    let rel = ui::dropdown(&rel_labels);
    let tag_dd = ui::dropdown(&["All tags"]);
    let tag_names: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(vec![]));
    let update_all = gtk::Button::with_label("Update all");
    let reload = gtk::Button::from_icon_name("view-refresh-symbolic");
    reload.set_tooltip_text(Some("Reload list"));

    let bar = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    for w in [search.upcast_ref::<gtk::Widget>(), rel.upcast_ref(), tag_dd.upcast_ref(), update_all.upcast_ref(), reload.upcast_ref()] {
        bar.append(w);
    }
    let progress = ui::label("", &["dim-label"]);
    let count = ui::label("People", &["title-3"]);
    let empty = ui::label("", &["dim-label"]);
    empty.set_margin_top(24);
    let list = gtk::ListBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .css_classes(["boxed-list"])
        .build();

    let root = gtk::Box::new(gtk::Orientation::Vertical, 12);
    ui::margins(&root, 12);
    for w in [bar.upcast_ref::<gtk::Widget>(), progress.upcast_ref(), count.upcast_ref(), list.upcast_ref(), empty.upcast_ref()] {
        root.append(w);
    }

    let visible: Rc<RefCell<Vec<i64>>> = Rc::new(RefCell::new(vec![]));

    let sync_tags = {
        let app = app.clone();
        let tag_dd = tag_dd.clone();
        let tag_names = tag_names.clone();
        move || {
            let mut counts: std::collections::HashMap<String, usize> = Default::default();
            for e in app.people.borrow().iter() {
                for t in &e.person.tags {
                    *counts.entry(t.clone()).or_default() += 1;
                }
            }
            let mut v: Vec<(String, usize)> = counts.into_iter().collect();
            v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
            let names: Vec<String> = v.into_iter().map(|x| x.0).collect();

            let prev = {
                let i = tag_dd.selected() as usize;
                if i == 0 { None } else { tag_names.borrow().get(i - 1).cloned() }
            };
            if *tag_names.borrow() == names {
                return;
            }
            if let Some(model) = tag_dd.model().and_then(|m| m.downcast::<gtk::StringList>().ok()) {
                let labels: Vec<String> = std::iter::once("All tags".to_owned()).chain(names.iter().map(|n| format!("#{n}"))).collect();
                let refs: Vec<&str> = labels.iter().map(String::as_str).collect();
                model.splice(0, model.n_items(), &refs);
            }
            *tag_names.borrow_mut() = names;
            let idx = prev.and_then(|p| tag_names.borrow().iter().position(|n| *n == p)).map(|i| i + 1).unwrap_or(0);
            tag_dd.set_selected(idx as u32);
        }
    };

    let render: Rc<dyn Fn()> = {
        let app = app.clone();
        let (search, rel, tag_dd, tag_names) = (search.clone(), rel.clone(), tag_dd.clone(), tag_names.clone());
        let (list, count, empty, progress, update_all, visible) =
            (list.clone(), count.clone(), empty.clone(), progress.clone(), update_all.clone(), visible.clone());
        Rc::new(move || {
            let q = search.text().trim().to_lowercase();
            let ri = rel.selected() as usize;
            let rel_key = if ri == 0 { None } else { STATUSES.get(ri - 1).map(|s| s.0) };
            let ti = tag_dd.selected() as usize;
            let tag_sel = if ti == 0 { None } else { tag_names.borrow().get(ti - 1).cloned() };

            ui::clear_list(&list);
            let mut ids = vec![];
            let people = app.people.borrow();
            for e in people.iter() {
                let p = &e.person;
                let ok = rel_key.map_or(true, |k| p.relation == k)
                    && tag_sel.as_ref().map_or(true, |t| p.tags.contains(t))
                    && (q.is_empty()
                        || e.name().to_lowercase().contains(&q)
                        || e.profile.as_ref().map_or(false, |pr| pr.username.to_lowercase().contains(&q))
                        || p.user_id.to_string().contains(&q)
                        || p.notes.as_ref().map_or(false, |n| n.to_lowercase().contains(&q))
                        || p.met_context.as_ref().map_or(false, |n| n.to_lowercase().contains(&q)));
                if !ok {
                    continue;
                }
                ids.push(p.user_id);

                let mut sub = e.profile.as_ref().map(|pr| format!("@{} · ", pr.username)).unwrap_or_default();
                sub.push_str(&status_label(&p.relation));
                if !p.tags.is_empty() {
                    sub.push('\n');
                    sub.push_str(&p.tags.iter().map(|t| format!("#{t}")).collect::<Vec<_>>().join(" "));
                }
                let row = adw::ActionRow::builder().title(ui::esc(&e.name())).subtitle(ui::esc(&sub)).activatable(true).build();
                row.add_prefix(&ui::avatar(e.profile.as_ref().and_then(|pr| pr.avatar.as_deref()), &e.name(), 40));
                row.add_suffix(&gtk::Image::from_icon_name("go-next-symbolic"));
                let (app2, id) = (app.clone(), p.user_id);
                row.connect_activated(move |_| app2.open_person(id));
                list.append(&row);
            }

            count.set_label(&format!("People ({})", ids.len()));
            list.set_visible(!ids.is_empty());
            empty.set_visible(ids.is_empty());
            empty.set_label(if app.loading.get() && people.is_empty() {
                "Loading…"
            } else if people.is_empty() {
                "Nobody on your list yet. Use the Lookup tab to add people."
            } else {
                "Nobody matches that."
            });
            let prog = app.progress.borrow();
            progress.set_visible(prog.is_some());
            progress.set_label(prog.as_deref().unwrap_or(""));
            update_all.set_sensitive(prog.is_none() && !ids.is_empty());
            *visible.borrow_mut() = ids;
        })
    };

    {
        let r = render.clone();
        search.connect_search_changed(move |_| r());
        let r = render.clone();
        rel.connect_selected_notify(move |_| r());
        let r = render.clone();
        tag_dd.connect_selected_notify(move |_| r());
        let (app2, vis) = (app.clone(), visible.clone());
        update_all.connect_clicked(move |_| app2.refresh_all(vis.borrow().clone()));
        let app2 = app.clone();
        reload.connect_clicked(move |_| app2.load_people());
    }
    {
        let r = render.clone();
        app.on_people_changed(move || {
            sync_tags();
            r();
        });
    }
    render();
    ui::scrolled(&root).upcast()
}
