use std::sync::Arc;

use gtk::{glib::Propagation, prelude::*};

use crate::tipos_de_dato::{comandos::rm::Remove, logger::Logger};

use super::{comando_gui::ComandoGui, dibujar_dialog, staging_area};

fn ejecutar(builder: &gtk::Builder, logger: Arc<Logger>) {
    let input = builder.object::<gtk::Entry>("rm-input").unwrap();
    let input_texto = input.text().to_string();
    let splitted = input_texto.split_ascii_whitespace().collect::<Vec<&str>>();
    let args = splitted
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<String>>();

    Remove::from(args, logger.clone()).ejecutar_gui();
    staging_area::render(builder, logger)
}

fn dialog(builder: &gtk::Builder, logger: Arc<Logger>) {
    let dialog = builder.object::<gtk::Dialog>("rm-dialog").unwrap();

    let confirmar = builder.object::<gtk::Button>("confirm-rm").unwrap();
    let cancelar = builder.object::<gtk::Button>("cancel-rm").unwrap();
    let input = builder.object::<gtk::Entry>("rm-input").unwrap();
    input.set_text("");

    let dialog_clone = dialog.clone();
    let builder_clone = builder.clone();
    let confirmar_id = confirmar.connect_clicked(move |_| {
        ejecutar(&builder_clone, logger.clone());
        dialog_clone.hide();
    });

    let dialog_clone = dialog.clone();
    let cancelar_id = cancelar.connect_clicked(move |_| {
        dialog_clone.hide();
    });

    let destroy_id = dialog.connect_destroy_event(|dialog, _| {
        dialog.hide();
        Propagation::Stop
    });

    dibujar_dialog(&dialog);

    confirmar.disconnect(confirmar_id);
    cancelar.disconnect(cancelar_id);
    dialog.disconnect(destroy_id);
}

pub fn render(builder: &gtk::Builder, logger: Arc<Logger>) {
    let event_box = builder.object::<gtk::EventBox>("rm-box").unwrap();

    let builder_clone = builder.clone();
    event_box.connect_button_press_event(move |_, _| {
        let logger_clone = logger.clone();
        dialog(&builder_clone, logger_clone.clone());
        gtk::glib::Propagation::Stop
    });
}
