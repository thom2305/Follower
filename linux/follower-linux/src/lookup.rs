// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Thom (thom2305)

use crate::{ui, App};
use adw::prelude::*;
use follower_core::*;
use std::cell::RefCell;
use std::rc::Rc;

pub fn build(app: &Rc<App>) -> gtk::Widget {
    let text = gtk::TextView::builder()
        .wrap_mode(gtk::WrapMode::WordChar)
        .top_margin(8).bottom_margin(8).left_margin(8).right_margin(8)
        .build();
    let text_scroll = gtk::ScrolledWindow::builder().min_content_height(110).child(&text).css_classes(["card"]).build();
    let hint = ui::label("0 ID(s) found · paste as many as you like (max 25)", &["caption", "dim-label"]);

    let save = adw::SwitchRow::builder().title("Also add to my list").build();
    let mut labels = vec!["Don't set"];
    labels.extend(STATUSES.iter().map(|s| s.1));
    let rel = adw::ComboRow::builder().title("Status").model(&gtk::StringList::new(&labels)).visible(false).build();
    let tag = adw::EntryRow::builder().title("Tag (optional)").visible(false).build();
    let opts = adw::PreferencesGroup::builder().title(ui::esc("User IDs or <@mentions>")).build();
    opts.add(&save);
    opts.add(&rel);
    opts.add(&tag);

    let go = gtk::Button::builder().label("Look up").css_classes(["suggested-action", "pill"]).halign(gtk::Align::Center).build();
    let save_all = gtk::Button::builder().label("Save all to list").css_classes(["pill"]).halign(gtk::Align::Center).visible(false).build();
    let list = gtk::ListBox::builder().selection_mode(gtk::SelectionMode::None).css_classes(["boxed-list"]).build();
    let results_title = ui::label("Results", &["title-3"]);
    results_title.set_visible(false);

    let root = gtk::Box::new(gtk::Orientation::Vertical, 12);
    ui::margins(&root, 12);
    for w in [
        opts.upcast_ref::<gtk::Widget>(), text_scroll.upcast_ref(), hint.upcast_ref(), go.upcast_ref(),
        save_all.upcast_ref(), results_title.upcast_ref(), list.upcast_ref(),
    ] {
        root.append(w);
    }

    let results: Rc<RefCell<Vec<LookupEntry>>> = Rc::new(RefCell::new(vec![]));

    let slot: Rc<RefCell<Option<Rc<dyn Fn()>>>> = Rc::new(RefCell::new(None));

    let render: Rc<dyn Fn()> = {
        let (app, results, list, save_all, results_title, slot) =
            (app.clone(), results.clone(), list.clone(), save_all.clone(), results_title.clone(), slot.clone());
        Rc::new(move || {
            ui::clear_list(&list);
            let res = results.borrow();
            results_title.set_visible(!res.is_empty());
            let untracked = res.iter().filter(|r| r.person.is_none()).count();
            save_all.set_visible(untracked > 1);
            save_all.set_label(&format!("Save all {untracked} to list"));

            for r in res.iter() {
                let name = r.profile.as_ref().map(|p| p.display_name().to_owned()).unwrap_or_else(|| "Not found".into());
                let mut sub = r.profile.as_ref().map(|p| format!("@{} · ", p.username)).unwrap_or_default();
                sub.push_str(&r.user_id.to_string());
                sub.push('\n');
                sub.push_str(&match &r.person {
                    Some(p) => format!("In your list: {}", status_label(&p.relation)),
                    None => "Not tracked".into(),
                });
                for c in &r.changes {
                    sub.push_str(&format!("\n{}: {} → {}", c.key, ui::trunc(&c.from, 60), ui::trunc(&c.to, 60)));
                }
                let row = adw::ActionRow::builder().title(ui::esc(&name)).subtitle(ui::esc(&sub)).activatable(true).build();
                row.add_prefix(&ui::avatar(r.profile.as_ref().and_then(|p| p.avatar.as_deref()), &name, 40));
                let (app2, id) = (app.clone(), r.user_id);
                row.connect_activated(move |_| app2.open_person(id));
                if r.person.is_none() {
                    let b = gtk::Button::builder().label("Save").valign(gtk::Align::Center).build();
                    let (app3, results2, slot2) = (app.clone(), results.clone(), slot.clone());
                    b.connect_clicked(move |_| save_ids(&app3, &results2, vec![id], slot2.borrow().clone()));
                    row.add_suffix(&b);
                }
                list.append(&row);
            }
        })
    };
    *slot.borrow_mut() = Some(render.clone());

    {
        let (app2, results2, slot2) = (app.clone(), results.clone(), slot.clone());
        save_all.connect_clicked(move |_| {
            let ids: Vec<i64> = results2.borrow().iter().filter(|x| x.person.is_none()).map(|x| x.user_id).collect();
            save_ids(&app2, &results2, ids, slot2.borrow().clone());
        });
    }

    {
        let hint2 = hint.clone();
        text.buffer().connect_changed(move |b| {
            let t = b.text(&b.start_iter(), &b.end_iter(), false);
            hint2.set_label(&format!("{} ID(s) found · paste as many as you like (max 25)", parse_ids(&t).len()));
        });
        let (rel2, tag2) = (rel.clone(), tag.clone());
        save.connect_active_notify(move |s| {
            rel2.set_visible(s.is_active());
            tag2.set_visible(s.is_active());
        });
    }

    {
        let (app2, results2, r, go2, text2, save2, rel2, tag2) =
            (app.clone(), results.clone(), render.clone(), go.clone(), text.clone(), save.clone(), rel.clone(), tag.clone());
        go.connect_clicked(move |_| {
            let buf = text2.buffer();
            let ids: Vec<i64> = parse_ids(&buf.text(&buf.start_iter(), &buf.end_iter(), false)).into_iter().take(25).collect();
            if ids.is_empty() {
                app2.toast("Paste at least one Discord ID first.");
                return;
            }
            let save_flag = save2.is_active();
            let relation = if save_flag && rel2.selected() > 0 {
                STATUSES.get(rel2.selected() as usize - 1).map(|s| s.0.to_owned())
            } else {
                None
            };
            let tg = norm_tag(tag2.text().as_str());
            let tg = (save_flag && !tg.is_empty()).then_some(tg);

            go2.set_sensitive(false);
            go2.set_label("Looking up…");
            let repo = app2.repo.clone();
            let (go_ok, go_err) = (go2.clone(), go2.clone());
            let (res, r2, app3) = (results2.clone(), r.clone(), app2.clone());
            app2.task_or(
                move || repo.lookup(&ids, save_flag, relation.as_deref(), tg.as_deref()),
                move |out| {
                    *res.borrow_mut() = out;
                    go_ok.set_sensitive(true);
                    go_ok.set_label("Look up");
                    r2();
                    if save_flag {
                        app3.load_people();
                    }
                },
                move || {
                    go_err.set_sensitive(true);
                    go_err.set_label("Look up");
                },
            );
        });
    }

    ui::scrolled(&root).upcast()
}

fn save_ids(app: &Rc<App>, results: &Rc<RefCell<Vec<LookupEntry>>>, ids: Vec<i64>, render: Option<Rc<dyn Fn()>>) {
    if ids.is_empty() {
        return;
    }
    let repo = app.repo.clone();
    let (res, app2) = (results.clone(), app.clone());
    let n = ids.len();
    app.task(
        move || {
            repo.save_many(&ids)?;
            let mut persons = vec![];
            for id in &ids {
                persons.push((*id, repo.get_person(*id)?));
            }
            Ok(persons)
        },
        move |persons| {
            for (id, p) in persons {
                if let Some(e) = res.borrow_mut().iter_mut().find(|e| e.user_id == id) {
                    e.person = p;
                }
            }
            if let Some(r) = render {
                r();
            }
            app2.load_people();
            app2.toast(&format!("Saved {n} to your list"));
        },
    );
}
