use std::sync::Arc;

use gtk::{prelude::*, Orientation};

use crate::{
    tipos_de_dato::{comandos::tag::Tag, logger::Logger, objetos::commit::CommitObj},
    utils::io::{self},
};

use super::{comando_gui::ComandoGui, log_seleccionado};

fn crear_label(string: &str, color: &str, hash: &str) -> gtk::EventBox {
    let event_box = gtk::EventBox::new();

    let container = gtk::Box::new(Orientation::Horizontal, 0);
    event_box.add(&container);
    container.set_margin_start(6);
    container.set_margin_top(2);
    container.set_margin_bottom(1);
    container.set_margin_end(18); // Set margin at the end

    let label_message = gtk::Label::new(Some(string));
    container.add(&label_message);

    let spacer = gtk::Box::new(Orientation::Horizontal, 0);
    spacer.set_hexpand(true);
    container.pack_start(&spacer, true, true, 0);

    let label_branch = gtk::Label::new(Some(&hash[0..10]));
    label_branch
        .style_context()
        .add_class(&format!("label-{}", color));

    container.add(&label_branch);

    event_box
        .style_context()
        .add_class(&format!("commit-label-{}", color));

    event_box
}

fn hide_tags(builder: &gtk::Builder) {
    let container: gtk::Box = builder.object("tags").unwrap();
    container.hide();
}

pub fn render(builder: &gtk::Builder, logger: Arc<Logger>) {
    let container: gtk::Box = builder.object("tag-container").unwrap();
    container.children().iter().for_each(|child| {
        container.remove(child);
    });

    let mut tags = if let Some(tags_text) = Tag::from(vec![], logger.clone()).ejecutar_gui() {
        let splitteado = tags_text.lines();
        splitteado
            .map(|line| line.to_string())
            .collect::<Vec<String>>()
    } else {
        vec![]
    };

    if tags.is_empty() {
        hide_tags(builder);
        return;
    }

    tags.sort_by_key(|tag| {
        let hash = io::leer_a_string(".gir/refs/tags/".to_string() + tag).unwrap();
        let commit = CommitObj::from_hash(hash, logger.clone()).unwrap();
        commit.date.tiempo.clone()
    });

    for tag in tags {
        let hash = io::leer_a_string(".gir/refs/tags/".to_string() + &tag).unwrap();
        let event_box = crear_label(&tag, "blue", &hash);
        let builder_clone = builder.clone();
        event_box.connect_button_press_event(move |_, _| {
            log_seleccionado::render(&builder_clone, Some(&hash));
            gtk::glib::Propagation::Stop
        });
        container.add(&event_box);
    }

    if !container.children().is_empty() {
        let children = container.children();
        let ultimo = children.last().unwrap();
        ultimo.style_context().add_class("last-commit-label");
    }

    container.show_all();
}
