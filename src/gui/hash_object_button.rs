use std::sync::Arc;

use gtk::{glib::Propagation, prelude::*};

use crate::tipos_de_dato::{comandos::hash_object::HashObject, logger::Logger};

use super::{comando_gui::ComandoGui, dibujar_dialog, info_dialog};

fn ejecutar(builder: &gtk::Builder, input: &gtk::Entry, logger: Arc<Logger>) {
    let input_texto = input.text();
    let mut args = vec![input_texto.to_string()];

    let checkbox1: gtk::CheckButton = builder.object("hash-object-check").unwrap();
    if checkbox1.is_active() {
        args.insert(0, "-w".to_string());
    }

    let resultado =
        HashObject::from(&mut vec![input_texto.to_string()], logger.clone()).ejecutar_gui();

    if let Some(resulado) = resultado {
        info_dialog::mostrar_mensaje("Hash del objeto:", &resulado);
    }
}

fn dialog(builder: &gtk::Builder, logger: Arc<Logger>) {
    // let tree_view = builder.object::<gtk::TreeView>("hash-object-tree").unwrap();
    let dialog = builder.object::<gtk::Dialog>("hash-object-dialog").unwrap();

    let confirmar = builder
        .object::<gtk::Button>("confirm-hash-object")
        .unwrap();
    let cancelar = builder.object::<gtk::Button>("cancel-hash-object").unwrap();

    let input = builder.object::<gtk::Entry>("hash-object-input").unwrap();
    input.set_text("");

    let dialog_clone = dialog.clone();
    let builder_clone = builder.clone();

    let confirmar_id = confirmar.connect_clicked(move |_| {
        ejecutar(&builder_clone, &input, logger.clone());
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
    let event_box = builder.object::<gtk::EventBox>("hash-object-box").unwrap();

    let builder_clone = builder.clone();
    event_box.connect_button_press_event(move |_, _| {
        let logger_clone = logger.clone();
        dialog(&builder_clone, logger_clone.clone());
        gtk::glib::Propagation::Stop
    });
}
