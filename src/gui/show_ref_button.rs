use std::sync::Arc;

use gtk::{glib::Propagation, prelude::*};

use crate::tipos_de_dato::{comandos::show_ref::ShowRef, logger::Logger};

use super::{comando_gui::ComandoGui, dibujar_dialog, info_dialog};

fn ejecutar(builder: &gtk::Builder, logger: Arc<Logger>) {
    let mut args = vec![];

    let heads: gtk::CheckButton = builder.object("show-ref-heads").unwrap();
    if heads.is_active() {
        args.push("--heads".to_string());
    }

    let head: gtk::CheckButton = builder.object("show-ref-head").unwrap();
    if head.is_active() {
        args.push("--head".to_string());
    }

    let tags: gtk::CheckButton = builder.object("show-ref-tags").unwrap();
    if tags.is_active() {
        args.push("--tags".to_string());
    }

    let resultado = ShowRef::from(args, logger.clone()).ejecutar_gui();

    if let Some(resulado) = resultado {
        info_dialog::mostrar_mensaje("Referencias:", &resulado);
    }
}

fn dialog(builder: &gtk::Builder, logger: Arc<Logger>) {
    let dialog = builder.object::<gtk::Dialog>("show-ref-dialog").unwrap();

    let confirmar = builder.object::<gtk::Button>("confirm-show-ref").unwrap();
    let cancelar = builder.object::<gtk::Button>("cancel-show-ref").unwrap();

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
    let event_box = builder.object::<gtk::EventBox>("show-ref-box").unwrap();

    let builder_clone = builder.clone();
    event_box.connect_button_press_event(move |_, _| {
        let logger_clone = logger.clone();
        dialog(&builder_clone, logger_clone.clone());
        gtk::glib::Propagation::Stop
    });
}
