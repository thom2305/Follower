// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Thom (thom2305)

use crate::{ui, App};
use adw::prelude::*;
use follower_core::*;
use std::cell::RefCell;
use std::collections::BTreeSet;
use std::rc::Rc;

#[derive(Default)]
struct State {
    profile: Option<Profile>,
    person: Option<Person>,
    changes: Vec<Change>,
    gone: bool,
    history: Option<PersonHistory>,
}

struct Pv {
    app: Rc<App>,
    id: i64,
    page: adw::NavigationPage,
    content: gtk::Box,
    spinner: gtk::Spinner,
    refresh_btn: gtk::Button,
    st: RefCell<State>,
}

pub fn page(app: &Rc<App>, id: i64) -> adw::NavigationPage {
    let content = gtk::Box::new(gtk::Orientation::Vertical, 18);
    ui::margins(&content, 12);

    let spinner = gtk::Spinner::new();
    let refresh_btn = gtk::Button::from_icon_name("view-refresh-symbolic");
    refresh_btn.set_tooltip_text(Some("Refresh from Discord"));
    let header = adw::HeaderBar::new();
    header.pack_end(&refresh_btn);
    header.pack_end(&spinner);

    let tv = adw::ToolbarView::new();
    tv.add_top_bar(&header);
    tv.set_content(Some(&ui::scrolled(&content)));
    let page = adw::NavigationPage::builder().title(id.to_string()).child(&tv).build();

    let pv = Rc::new(Pv { app: app.clone(), id, page: page.clone(), content, spinner, refresh_btn, st: RefCell::new(State::default()) });
    {
        let pv2 = pv.clone();
        pv.refresh_btn.connect_clicked(move |_| refresh(&pv2));
    }
    render(&pv);
    load(&pv);
    page
}

fn set_busy(pv: &Rc<Pv>, busy: bool) {
    pv.spinner.set_spinning(busy);
    pv.spinner.set_visible(busy);
    pv.refresh_btn.set_visible(!busy);
}

fn load(pv: &Rc<Pv>) {
    set_busy(pv, true);
    let (repo, id) = (pv.app.repo.clone(), pv.id);
    let (p_ok, p_err) = (pv.clone(), pv.clone());
    pv.app.task_or(
        move || repo.get_detail(id),
        move |(person, profile)| {
            {
                let mut s = p_ok.st.borrow_mut();
                s.person = person;
                s.profile = profile;
            }
            render(&p_ok);
            refresh(&p_ok);
        },
        move || set_busy(&p_err, false),
    );
}

fn refresh(pv: &Rc<Pv>) {
    set_busy(pv, true);
    let (repo, id) = (pv.app.repo.clone(), pv.id);
    let (p_ok, p_err) = (pv.clone(), pv.clone());
    pv.app.task_or(
        move || repo.take_snapshot(id),
        move |r| {
            {
                let mut s = p_ok.st.borrow_mut();
                s.gone = r.profile.is_none();
                if let Some(p) = r.profile {
                    s.profile = Some(p);
                }
                s.changes = r.changes;
            }
            set_busy(&p_ok, false);
            render(&p_ok);
        },
        move || set_busy(&p_err, false),
    );
}

fn mutate(pv: &Rc<Pv>, op: impl FnOnce(&Repo) -> Result<(), ApiError> + Send + 'static) {
    let (repo, id) = (pv.app.repo.clone(), pv.id);
    let p2 = pv.clone();
    pv.app.task(
        move || {
            op(&repo)?;
            repo.get_person(id)
        },
        move |person| {
            p2.st.borrow_mut().person = person;
            render(&p2);
            p2.app.load_people();
        },
    );
}

fn add_tags(pv: &Rc<Pv>, raw: &str) {
    let tags: BTreeSet<String> = raw.split(',').map(norm_tag).filter(|t| !t.is_empty()).collect();
    if tags.is_empty() {
        return;
    }
    let id = pv.id;
    mutate(pv, move |r| {
        for t in &tags {
            r.add_tag(id, t)?;
        }
        Ok(())
    });
}

fn render(pv: &Rc<Pv>) {
    ui::clear_box(&pv.content);
    let (profile, person, changes, gone, history) = {
        let s = pv.st.borrow();
        (s.profile.clone(), s.person.clone(), s.changes.clone(), s.gone, s.history.clone())
    };
    let zone = pv.app.zone();
    let name = profile.as_ref().map(|p| p.display_name().to_owned()).unwrap_or_else(|| pv.id.to_string());
    pv.page.set_title(&name);
    let c = &pv.content;

    let head = gtk::Box::new(gtk::Orientation::Horizontal, 14);
    head.append(&ui::avatar(profile.as_ref().and_then(|p| p.avatar.as_deref()), &name, 72));
    let titles = gtk::Box::new(gtk::Orientation::Vertical, 2);
    titles.set_valign(gtk::Align::Center);
    titles.append(&ui::label(&name, &["title-2"]));
    if let Some(p) = &profile {
        titles.append(&ui::label(&format!("@{}", p.username), &["dim-label"]));
    }
    let idl = ui::label(&pv.id.to_string(), &["caption", "dim-label"]);
    idl.set_selectable(true);
    titles.append(&idl);
    head.append(&titles);
    c.append(&head);

    if gone {
        c.append(&ui::label(
            "Couldn't fetch this account from Discord (deleted or unavailable). Showing the last saved profile.",
            &["error"],
        ));
    }
    if let Some(p) = &profile {
        let g = adw::PreferencesGroup::new();
        g.add(&ui::prop_row("Created", &pretty_str(&p.created_at, zone)));
        g.add(&ui::prop_row("Badges", &if p.badges.is_empty() { "none".to_owned() } else { p.badges.join(", ") }));
        if p.bot {
            g.add(&ui::prop_row("Bot", "yes"));
        }
        c.append(&g);
    }

    if !changes.is_empty() {
        let g = adw::PreferencesGroup::builder().title("Changed since last check").build();
        for ch in &changes {
            g.add(&ui::prop_row(&ch.key, &format!("{} → {}", trunc_s(&ch.from, 60), trunc_s(&ch.to, 60))));
        }
        c.append(&g);
    }

    let g = adw::PreferencesGroup::builder().title("Relation").build();
    match &person {
        Some(p) => {
            let labels: Vec<&str> = STATUSES.iter().map(|s| s.1).collect();
            let row = adw::ComboRow::builder()
                .title("Status")
                .subtitle(ui::esc(&format!("Tracked since {}", pretty_date(&p.first_seen, zone))))
                .model(&gtk::StringList::new(&labels))
                .build();
            row.set_selected(STATUSES.iter().position(|s| s.0 == p.relation).unwrap_or(STATUSES.len() - 1) as u32);
            let (pv2, current, id) = (pv.clone(), p.relation.clone(), pv.id);
            row.connect_selected_notify(move |r| {
                if let Some((key, _)) = STATUSES.get(r.selected() as usize) {
                    if *key != current {
                        let key = key.to_string();
                        mutate(&pv2, move |repo| repo.set_relation(id, &key));
                    }
                }
            });
            g.add(&row);
        }
        None => {
            let row = adw::ActionRow::builder().title("Not on your list yet.").build();
            let b = gtk::Button::builder().label("Add to list").valign(gtk::Align::Center).css_classes(["suggested-action"]).build();
            let (pv2, id) = (pv.clone(), pv.id);
            b.connect_clicked(move |_| mutate(&pv2, move |repo| repo.ensure_person(id)));
            row.add_suffix(&b);
            g.add(&row);
        }
    }
    c.append(&g);

    if let Some(p) = &person {
        let g = adw::PreferencesGroup::builder().title("Tags").build();
        if !p.tags.is_empty() {
            let flow = flow_box();
            for t in &p.tags {
                let b = gtk::Button::builder().label(format!("#{t}  ✕")).css_classes(["pill"]).tooltip_text("Remove tag").build();
                let (pv2, t2, id) = (pv.clone(), t.clone(), pv.id);
                b.connect_clicked(move |_| {
                    let t3 = t2.clone();
                    mutate(&pv2, move |repo| repo.remove_tag(id, &t3));
                });
                flow.append(&b);
            }
            g.add(&flow);
        }
        let entry = adw::EntryRow::builder().title("Add tag(s), comma-separated").build();
        let pv2 = pv.clone();
        entry.connect_entry_activated(move |e| {
            let text = e.text().to_string();
            e.set_text("");
            add_tags(&pv2, &text);
        });
        g.add(&entry);
        c.append(&g);

        let suggestions: Vec<String> =
            pv.app.tags.borrow().iter().map(|t| t.tag.clone()).filter(|t| !p.tags.contains(t)).take(12).collect();
        if !suggestions.is_empty() {
            c.append(&ui::label("Existing tags", &["caption", "dim-label"]));
            let flow = flow_box();
            for t in suggestions {
                let b = gtk::Button::builder().label(format!("#{t}")).css_classes(["pill"]).build();
                let (pv2, t2) = (pv.clone(), t.clone());
                b.connect_clicked(move |_| add_tags(&pv2, &t2));
                flow.append(&b);
            }
            c.append(&flow);
        }

        let g = adw::PreferencesGroup::builder().title("Details").build();
        if let (Some(mo), Some(d)) = (p.birthday_month, p.birthday_day) {
            g.add(&ui::prop_row("Birthday", &fmt_birthday(mo, d, p.birthday_year, zone)));
        }
        if let Some(m) = &p.met_context {
            g.add(&ui::prop_row("Know them from", m));
        }
        if let Some(n) = &p.notes {
            g.add(&ui::prop_row("Notes", n));
        }
        let edit = adw::ActionRow::builder().title("Edit details").activatable(true).build();
        edit.add_prefix(&gtk::Image::from_icon_name("document-edit-symbolic"));
        let (pv2, p2) = (pv.clone(), p.clone());
        edit.connect_activated(move |_| edit_dialog(&pv2, &p2));
        g.add(&edit);
        c.append(&g);
    }

    let g = adw::PreferencesGroup::builder().title("History").build();
    match &history {
        Some(h) => {
            for s in &h.statuses {
                g.add(&ui::prop_row(&format!("Relation: {}", status_label(&s.status)), &pretty(&s.at, zone)));
            }
            if h.profile_changes.is_empty() {
                g.add(&adw::ActionRow::builder().title("No snapshots yet.").build());
            }
            for ch in &h.profile_changes {
                let lines: Vec<String> =
                    ch.changes.iter().map(|x| format!("{}: {} → {}", x.key, trunc_s(&x.from, 40), trunc_s(&x.to, 40))).collect();
                let title = pretty(&ch.at, zone) + if ch.is_first { " · first snapshot" } else { "" };
                g.add(&ui::prop_row(&title, &lines.join("\n")));
            }
        }
        None => {
            let row = adw::ActionRow::builder().title("Show history").activatable(true).build();
            row.add_prefix(&gtk::Image::from_icon_name("document-open-recent-symbolic"));
            let (pv2, repo, id) = (pv.clone(), pv.app.repo.clone(), pv.id);
            row.connect_activated(move |_| {
                let (pv3, repo) = (pv2.clone(), repo.clone());
                pv2.app.task(move || repo.history(id), move |h| {
                    pv3.st.borrow_mut().history = Some(h);
                    render(&pv3);
                });
            });
            g.add(&row);
        }
    }
    c.append(&g);

    if person.is_some() {
        let b = gtk::Button::builder().label("Remove from list").css_classes(["destructive-action", "pill"]).halign(gtk::Align::Center).build();
        let pv2 = pv.clone();
        b.connect_clicked(move |_| {
            let pv3 = pv2.clone();
            ui::confirm(
                &pv2.app,
                "Remove from list?",
                "Tags and details will be deleted. Profile snapshots are kept.",
                "Remove",
                true,
                move || {
                    let (repo, id, pv4) = (pv3.app.repo.clone(), pv3.id, pv3.clone());
                    pv3.app.task(move || repo.remove_person(id), move |_| {
                        pv4.app.nav.pop();
                        pv4.app.load_people();
                        pv4.app.toast("Removed from your list (profile snapshots kept).");
                    });
                },
            );
        });
        c.append(&b);
    }
}

fn flow_box() -> gtk::FlowBox {
    gtk::FlowBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .homogeneous(false)
        .max_children_per_line(12)
        .row_spacing(6)
        .column_spacing(6)
        .build()
}

fn trunc_s(s: &str, n: usize) -> String {
    ui::trunc(s, n)
}

fn edit_dialog(pv: &Rc<Pv>, person: &Person) {
    let dlg = adw::MessageDialog::new(Some(&pv.app.window), Some("Person details"), None);

    let notes = gtk::TextView::builder().wrap_mode(gtk::WrapMode::WordChar).top_margin(6).bottom_margin(6).left_margin(6).right_margin(6).build();
    notes.buffer().set_text(person.notes.as_deref().unwrap_or(""));
    let notes_scroll = gtk::ScrolledWindow::builder().min_content_height(110).child(&notes).css_classes(["card"]).build();
    let met = gtk::Entry::builder().placeholder_text("e.g. school, a Discord server").max_length(60).text(person.met_context.as_deref().unwrap_or("")).build();
    let bday = gtk::Entry::builder()
        .placeholder_text("YYYY-MM-DD or MM-DD")
        .max_length(10)
        .text(birthday_input(person.birthday_month, person.birthday_day, person.birthday_year))
        .build();

    let b = gtk::Box::new(gtk::Orientation::Vertical, 6);
    b.append(&ui::label("Notes", &["caption-heading"]));
    b.append(&notes_scroll);
    b.append(&ui::label("Where you know them from", &["caption-heading"]));
    b.append(&met);
    b.append(&ui::label("Birthday", &["caption-heading"]));
    b.append(&bday);
    dlg.set_extra_child(Some(&b));
    dlg.add_responses(&[("cancel", "Cancel"), ("save", "Save")]);
    dlg.set_response_appearance("save", adw::ResponseAppearance::Suggested);
    dlg.set_default_response(Some("save"));
    dlg.set_close_response("cancel");

    let pv2 = pv.clone();
    dlg.connect_response(None, move |_, resp| {
        if resp != "save" {
            return;
        }
        let buf = notes.buffer();
        let n: String = buf.text(&buf.start_iter(), &buf.end_iter(), false).trim().chars().take(1000).collect();
        let m: String = met.text().trim().chars().take(60).collect();
        let parts = match parse_birthday(bday.text().as_str()) {
            Ok(p) => p,
            Err(()) => {
                pv2.app.toast("Birthday not saved: use 1999-03-05 or 03-05");
                return;
            }
        };
        let id = pv2.id;
        mutate(&pv2, move |r| {
            r.update_details(id, (!n.is_empty()).then_some(n.as_str()), (!m.is_empty()).then_some(m.as_str()), parts.year, parts.month, parts.day)
        });
    });
    dlg.present();
}
