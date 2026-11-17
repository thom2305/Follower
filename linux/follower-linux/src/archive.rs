// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Thom (thom2305)

use crate::{ui, App};
use adw::prelude::*;
use follower_core::*;
use gtk::{gdk, gio};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

pub fn build(app: &Rc<App>) -> gtk::Widget {
    let text = adw::EntryRow::builder().title("Text contains").build();
    let author = adw::EntryRow::builder().title("Author (name or ID)").build();
    let author_model = gtk::StringList::new(&["Anyone"]);
    let pick = adw::ComboRow::builder().title("Pick an author").model(&author_model).build();
    let authors: Rc<RefCell<Vec<AuthorChoice>>> = Rc::new(RefCell::new(vec![]));

    let form = adw::PreferencesGroup::new();
    form.add(&text);
    form.add(&author);
    form.add(&pick);

    let search = gtk::Button::builder().label("Search").css_classes(["suggested-action", "pill"]).build();
    let random = gtk::Button::builder().label("Random").css_classes(["pill"]).build();
    let buttons = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    buttons.set_halign(gtk::Align::Center);
    buttons.append(&search);
    buttons.append(&random);

    let title = ui::label("", &["title-3"]);
    title.set_visible(false);
    let list = gtk::ListBox::builder().selection_mode(gtk::SelectionMode::None).css_classes(["boxed-list"]).build();

    let root = gtk::Box::new(gtk::Orientation::Vertical, 12);
    ui::margins(&root, 12);
    for w in [form.upcast_ref::<gtk::Widget>(), buttons.upcast_ref(), title.upcast_ref(), list.upcast_ref()] {
        root.append(w);
    }

    let results: Rc<RefCell<Vec<ArchiveMsg>>> = Rc::new(RefCell::new(vec![]));
    let slot: Rc<RefCell<Option<Rc<dyn Fn()>>>> = Rc::new(RefCell::new(None));

    let render: Rc<dyn Fn()> = {
        let (app, results, list, title, slot) = (app.clone(), results.clone(), list.clone(), title.clone(), slot.clone());
        Rc::new(move || {
            ui::clear_list(&list);
            let res = results.borrow();
            title.set_visible(true);
            title.set_label(&if res.is_empty() { "Nothing found.".to_owned() } else { format!("{} result(s)", res.len()) });
            let zone = app.zone();
            for m in res.iter() {
                let body = m.content.as_deref().map(str::trim).filter(|s| !s.is_empty()).unwrap_or("(no text)");
                let mut sub = ui::trunc(body, 240);
                if !m.attachments.is_empty() {
                    sub.push_str(&format!("\n{} attachment(s)", m.attachments.len()));
                }
                let row = adw::ActionRow::builder()
                    .title(ui::esc(&format!("{} · {}", m.author_name, pretty(&m.message_ts, zone))))
                    .subtitle(ui::esc(&sub))
                    .subtitle_lines(4)
                    .activatable(true)
                    .build();
                row.add_prefix(&ui::avatar(m.author_avatar.as_deref(), &m.author_name, 32));
                row.add_suffix(&gtk::Image::from_icon_name("go-next-symbolic"));
                let (app2, m2, slot2, results2) = (app.clone(), m.clone(), slot.clone(), results.clone());
                row.connect_activated(move |_| {
                    let (results3, slot3) = (results2.clone(), slot2.clone());
                    let page = detail(&app2, m2.clone(), move |gone_id| {
                        results3.borrow_mut().retain(|x| x.message_id != gone_id);
                        if let Some(r) = slot3.borrow().clone() {
                            r();
                        }
                    });
                    app2.nav.push(&page);
                });
                list.append(&row);
            }
        })
    };
    *slot.borrow_mut() = Some(render.clone());

    {
        let (authors2, author2) = (authors.clone(), author.clone());
        pick.connect_selected_notify(move |p| {
            let i = p.selected() as usize;
            if i == 0 {
                author2.set_text("");
            } else if let Some(a) = authors2.borrow().get(i - 1) {
                author2.set_text(&a.id.to_string());
            }
        });
    }

    let run_search: Rc<dyn Fn()> = {
        let (app, results, render, text, author, search) =
            (app.clone(), results.clone(), render.clone(), text.clone(), author.clone(), search.clone());
        Rc::new(move || {
            let (t, a) = (text.text().to_string(), author.text().to_string());
            let repo = app.repo.clone();
            search.set_sensitive(false);
            let (s_ok, s_err) = (search.clone(), search.clone());
            let (results, render) = (results.clone(), render.clone());
            app.task_or(
                move || repo.archive_search(t.trim(), a.trim()),
                move |msgs| {
                    *results.borrow_mut() = msgs;
                    s_ok.set_sensitive(true);
                    render();
                },
                move || s_err.set_sensitive(true),
            );
        })
    };
    {
        let r = run_search.clone();
        search.connect_clicked(move |_| r());
        let r = run_search.clone();
        text.connect_entry_activated(move |_| r());
        let r = run_search.clone();
        author.connect_entry_activated(move |_| r());
    }
    {
        let (app2, author2, results2, slot2) = (app.clone(), author.clone(), results.clone(), slot.clone());
        random.connect_clicked(move |_| {
            let (repo, a) = (app2.repo.clone(), author2.text().trim().to_string());
            let (app3, results3, slot3) = (app2.clone(), results2.clone(), slot2.clone());
            app2.task(move || repo.archive_random(&a), move |m| match m {
                Some(m) => {
                    let page = detail(&app3, m, move |gone_id| {
                        results3.borrow_mut().retain(|x| x.message_id != gone_id);
                        if let Some(r) = slot3.borrow().clone() {
                            r();
                        }
                    });
                    app3.nav.push(&page);
                }
                None => app3.toast("Archive is empty."),
            });
        });
    }

    let loaded = Rc::new(Cell::new(false));
    {
        let (app2, authors2, model, loaded2) = (app.clone(), authors.clone(), author_model.clone(), loaded.clone());
        root.connect_map(move |_| {
            if loaded2.get() || !app2.ready() {
                return;
            }
            loaded2.set(true);
            let (repo, authors3, model2) = (app2.repo.clone(), authors2.clone(), model.clone());
            let loaded3 = loaded2.clone();
            app2.task_or(
                move || repo.authors(),
                move |list| {
                    let labels: Vec<String> =
                        std::iter::once("Anyone".to_owned()).chain(list.iter().map(|a| format!("{} ({})", a.name, a.count))).collect();
                    let refs: Vec<&str> = labels.iter().map(String::as_str).collect();
                    model2.splice(0, model2.n_items(), &refs);
                    *authors3.borrow_mut() = list;
                },
                move || loaded3.set(false),
            );
        });
    }

    ui::scrolled(&root).upcast()
}

fn detail(app: &Rc<App>, m: ArchiveMsg, on_deleted: impl Fn(i64) + 'static) -> adw::NavigationPage {
    let zone = app.zone();
    let c = gtk::Box::new(gtk::Orientation::Vertical, 14);
    ui::margins(&c, 12);

    let head = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    head.append(&ui::avatar(m.author_avatar.as_deref(), &m.author_name, 48));
    head.append(&ui::label(&m.author_name, &["title-3"]));
    c.append(&head);

    let body = ui::label(m.content.as_deref().map(str::trim).filter(|s| !s.is_empty()).unwrap_or("(no text)"), &[]);
    body.set_selectable(true);
    c.append(&body);
    c.append(&ui::label(&format!("Sent {}", pretty(&m.message_ts, zone)), &["caption", "dim-label"]));
    c.append(&ui::label(&format!("Saved {}", pretty(&m.saved_at, zone)), &["caption", "dim-label"]));

    if !m.attachments.is_empty() {
        let g = adw::PreferencesGroup::builder().title("Attachments").build();
        for a in &m.attachments {
            g.add(&attachment_row(app, a, &c));
        }
        c.append(&g);
    }

    if let Some(url) = m.jump_url.clone() {
        let b = gtk::Button::builder().label("Open in Discord").halign(gtk::Align::Start).build();
        let app2 = app.clone();
        b.connect_clicked(move |_| open_uri(&app2, &url));
        c.append(&b);
    }

    let del = gtk::Button::builder().label("Delete").css_classes(["destructive-action", "pill"]).halign(gtk::Align::Center).build();
    let (app2, id) = (app.clone(), m.message_id);
    let on_deleted = Rc::new(on_deleted);
    del.connect_clicked(move |_| {
        let (app3, on_deleted2) = (app2.clone(), on_deleted.clone());
        ui::confirm(&app2, "Delete this message?", "This also deletes the stored attachments.", "Delete", true, move || {
            let (repo, app4, od) = (app3.repo.clone(), app3.clone(), on_deleted2.clone());
            app3.task(move || repo.archive_delete(id), move |_| {
                app4.nav.pop();
                od(id);
                app4.toast("Deleted (including stored attachments).");
            });
        });
    });
    c.append(&del);
    c.append(&ui::label(&format!("ID {}", m.message_id), &["caption", "dim-label"]));

    let tv = adw::ToolbarView::new();
    tv.add_top_bar(&adw::HeaderBar::new());
    tv.set_content(Some(&ui::scrolled(&c)));
    adw::NavigationPage::builder().title("Message").child(&tv).build()
}

fn open_uri(app: &Rc<App>, url: &str) {
    gtk::UriLauncher::new(url).launch(Some(&app.window), None::<&gio::Cancellable>, |_| {});
}

fn attachment_row(app: &Rc<App>, a: &Attachment, content: &gtk::Box) -> adw::ActionRow {
    let is_image = a.content_type.as_deref().map_or(false, |t| t.starts_with("image/"));
    let where_text = if !a.stored {
        "link only".to_owned()
    } else if a.compressed {
        format!("stored (compressed to {})", human_size(a.stored_size))
    } else {
        "stored".to_owned()
    };
    let info = format!("{} · {} · {}", human_size(a.size), a.content_type.as_deref().unwrap_or("unknown"), where_text);
    let row = adw::ActionRow::builder().title(ui::esc(&a.filename)).subtitle(ui::esc(&info)).build();
    row.add_prefix(&gtk::Image::from_icon_name(if is_image { "image-x-generic-symbolic" } else { "text-x-generic-symbolic" }));

    if a.stored {
        if is_image {
            let b = gtk::Button::builder().label("Preview").valign(gtk::Align::Center).build();
            let (app2, content2, id) = (app.clone(), content.clone(), a.id);
            b.connect_clicked(move |btn| {
                btn.set_sensitive(false);
                let (repo, content3, btn2) = (app2.repo.clone(), content2.clone(), btn.clone());
                app2.task_or(
                    move || repo.load_attachment(id),
                    move |bytes| match gdk::Texture::from_bytes(&gtk::glib::Bytes::from_owned(bytes)) {
                        Ok(tex) => {
                            let pic = gtk::Picture::for_paintable(&tex);
                            pic.set_can_shrink(true);
                            pic.set_content_fit(gtk::ContentFit::Contain);
                            pic.set_height_request(320);
                            content3.append(&pic);
                        }
                        Err(_) => btn2.set_sensitive(true),
                    },
                    {
                        let b2 = btn.clone();
                        move || b2.set_sensitive(true)
                    },
                );
            });
            row.add_suffix(&b);
        }
        let b = gtk::Button::builder().label("Save…").valign(gtk::Align::Center).build();
        let (app2, id, name) = (app.clone(), a.id, a.filename.replace('/', "_"));
        b.connect_clicked(move |_| {
            let dlg = gtk::FileDialog::builder().initial_name(if name.is_empty() { "attachment" } else { name.as_str() }).build();
            let app3 = app2.clone();
            dlg.save(Some(&app2.window), None::<&gio::Cancellable>, move |res| {
                let Ok(file) = res else { return };
                let Some(path) = file.path() else { return };
                let (repo, app4) = (app3.repo.clone(), app3.clone());
                app3.task(
                    move || {
                        let bytes = repo.load_attachment(id)?;
                        std::fs::write(&path, bytes).map_err(|e| ApiError(format!("Couldn't save: {e}")))?;
                        Ok(())
                    },
                    move |_| app4.toast("Saved"),
                );
            });
        });
        row.add_suffix(&b);
    } else if let Some(u) = a.url.clone() {
        let b = gtk::Button::builder().label("Open original").valign(gtk::Align::Center).build();
        let app2 = app.clone();
        b.connect_clicked(move |_| open_uri(&app2, &u));
        row.add_suffix(&b);
    }
    row
}
