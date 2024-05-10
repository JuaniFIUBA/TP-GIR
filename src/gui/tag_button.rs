use std::sync::Arc;

use gtk::{glib::Propagation, prelude::*};

use crate::tipos_de_dato::{comando::Ejecutar, comandos::tag::Tag, logger::Logger};

use super::{dibujar_dialog, info_dialog, tag_list};

fn run_dialog(builder: &gtk::Builder) {
    let tag_box: gtk::EventBox = builder.object("tag-box").unwrap();
    let dialog: gtk::MessageDialog = builder.object("tag-dialog").unwrap();

    dialog.set_position(gtk::WindowPosition::Center);

    tag_box.connect_button_press_event(move |_, _| {
        dibujar_dialog(&dialog);
        Propagation::Stop
    });
}

fn boton_cancel_dialog(builder: &gtk::Builder) {
    let cancel: gtk::Button = builder.object("cancel-tag").unwrap();
    let dialog: gtk::MessageDialog = builder.object("tag-dialog").unwrap();

    cancel.connect_clicked(move |_| {
        dialog.hide();
    });
}

fn boton_confimar_dialog(builder: &gtk::Builder, logger: Arc<Logger>) {
    let confirm: gtk::Button = builder.object("confirm-tag").unwrap();
    let dialog: gtk::MessageDialog = builder.object("tag-dialog").unwrap();
    let input: gtk::Entry = builder.object("tag-input").unwrap();

    let builder_clone = builder.clone();
    confirm.connect_clicked(move |_| {
        match Tag::from(vec![input.text().to_string()], logger.clone())
            .unwrap()
            .ejecutar()
        {
            Ok(_) => {}
            Err(err) => {
                info_dialog::mostrar_error(&err);
                return;
            }
        };

        tag_list::render(&builder_clone, logger.clone());
        input.set_text("");
        dialog.hide();
    });
}

pub fn render(builder: &gtk::Builder, logger: Arc<Logger>) {
    run_dialog(builder);

    boton_cancel_dialog(builder);
    boton_confimar_dialog(builder, logger);
}
