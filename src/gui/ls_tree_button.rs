use std::sync::Arc;

use gtk::{glib::Propagation, prelude::*};

use crate::tipos_de_dato::{comandos::ls_tree::LsTree, logger::Logger};

use super::{comando_gui::ComandoGui, dibujar_dialog, info_dialog};

fn ejecutar(builder: &gtk::Builder, input: &gtk::Entry, logger: Arc<Logger>) {
    let mut args = vec![];

    let recursivo: gtk::CheckButton = builder.object("recursivo").unwrap();
    if recursivo.is_active() {
        args.push("-r".to_string());
    }

    let size: gtk::CheckButton = builder.object("size").unwrap();
    if size.is_active() {
        args.push("-l".to_string());
    }

    let solo_arboles: gtk::CheckButton = builder.object("solo-arboles").unwrap();
    if solo_arboles.is_active() {
        args.push("-d".to_string());
    }

    let input_texto = input.text();
    args.push(input_texto.to_string());

    let resultado = LsTree::from(logger.clone(), &mut args).ejecutar_gui();

    if let Some(resulado) = resultado {
        info_dialog::mostrar_mensaje("Hash del objeto:", &resulado);
    }
}

fn dialog(builder: &gtk::Builder, logger: Arc<Logger>) {
    let dialog = builder.object::<gtk::Dialog>("ls-tree-dialog").unwrap();

    let confirmar = builder.object::<gtk::Button>("confirm-ls-tree").unwrap();
    let cancelar = builder.object::<gtk::Button>("cancel-ls-tree").unwrap();

    let input = builder.object::<gtk::Entry>("ls-tree-input").unwrap();
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
    let event_box = builder.object::<gtk::EventBox>("ls-tree-box").unwrap();

    let builder_clone = builder.clone();
    event_box.connect_button_press_event(move |_, _| {
        let logger_clone = logger.clone();
        dialog(&builder_clone, logger_clone.clone());
        gtk::glib::Propagation::Stop
    });
}
