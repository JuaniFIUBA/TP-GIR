use std::sync::Arc;

use gtk::{glib::Propagation, prelude::*};

use crate::tipos_de_dato::{comandos::cat_file::CatFile, logger::Logger};

use super::{comando_gui::ComandoGui, dibujar_dialog, info_dialog};

fn ejecutar(input: &gtk::Entry, logger: Arc<Logger>) {
    let input_texto = input.text().to_string();

    let tamanio = CatFile::from(
        &mut vec!["-s".to_string(), input_texto.to_owned()],
        logger.clone(),
    )
    .ejecutar_gui();
    let tipo = CatFile::from(
        &mut vec!["-t".to_string(), input_texto.to_owned()],
        logger.clone(),
    )
    .ejecutar_gui();
    let contenido = CatFile::from(
        &mut vec!["-p".to_string(), input_texto.to_owned()],
        logger.clone(),
    )
    .ejecutar_gui();

    if tipo.is_none() || contenido.is_none() || tamanio.is_none() {
        return;
    }

    let mensaje = format!(
        "Tamaño: {}\n\nTipo: {}\n\nContenido:\n {}",
        tamanio.unwrap(),
        tipo.unwrap(),
        contenido.unwrap()
    );

    info_dialog::mostrar_mensaje("Información del objeto:", &mensaje);
}

fn dialog(builder: &gtk::Builder, logger: Arc<Logger>) {
    let dialog = builder.object::<gtk::Dialog>("cat-file-dialog").unwrap();

    let confirmar = builder.object::<gtk::Button>("confirm-cat-file").unwrap();
    let cancelar = builder.object::<gtk::Button>("cancel-cat-file").unwrap();

    let input = builder.object::<gtk::Entry>("cat-file-input").unwrap();
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
    let event_box = builder.object::<gtk::EventBox>("cat-file-box").unwrap();

    let builder_clone = builder.clone();
    event_box.connect_button_press_event(move |_, _| {
        let logger_clone = logger.clone();
        dialog(&builder_clone, logger_clone.clone());
        gtk::glib::Propagation::Stop
    });
}
