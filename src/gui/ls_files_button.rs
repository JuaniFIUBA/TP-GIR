use std::sync::Arc;

use gtk::{glib::Propagation, prelude::*};

use crate::tipos_de_dato::{comandos::ls_files::LsFiles, logger::Logger};

use super::{comando_gui::ComandoGui, dibujar_dialog, info_dialog};

fn ejecutar(input: &gtk::Entry, logger: Arc<Logger>) {
    let input_texto = input.text().to_string();
    let splitted = input_texto.split_ascii_whitespace().collect::<Vec<&str>>();
    let mut args = splitted
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<String>>();

    let mensaje = LsFiles::from(logger.clone(), &mut args).ejecutar_gui();

    if let Some(mensaje) = mensaje {
        info_dialog::mostrar_mensaje("Output:", &mensaje);
    }
}

fn dialog(builder: &gtk::Builder, logger: Arc<Logger>) {
    let dialog = builder.object::<gtk::Dialog>("ls-files-dialog").unwrap();

    let confirmar = builder.object::<gtk::Button>("confirm-ls-files").unwrap();
    let cancelar = builder.object::<gtk::Button>("cancel-ls-files").unwrap();

    let input = builder.object::<gtk::Entry>("ls-files-input").unwrap();
    input.set_text("");

    let dialog_clone = dialog.clone();

    let confirmar_id = confirmar.connect_clicked(move |_| {
        ejecutar(&input, logger.clone());
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
    let event_box = builder.object::<gtk::EventBox>("ls-files-box").unwrap();

    let builder_clone = builder.clone();
    event_box.connect_button_press_event(move |_, _| {
        let logger_clone = logger.clone();
        dialog(&builder_clone, logger_clone.clone());
        gtk::glib::Propagation::Stop
    });
}
