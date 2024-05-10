use std::sync::Arc;

use crate::{
    tipos_de_dato::{
        comando::Ejecutar,
        comandos::{add::Add, status::Status},
        logger::Logger,
    },
    utils::index::{leer_index, ObjetoIndex},
};
use gtk::prelude::*;

use super::comando_gui::ComandoGui;

fn crear_label(string: &str) -> gtk::EventBox {
    let event_box = gtk::EventBox::new();
    let label = gtk::Label::new(Some(string));
    label.set_xalign(0.0);
    event_box.add(&label);

    event_box
}

fn extraer_path(objeto: &ObjetoIndex) -> String {
    objeto
        .objeto
        .obtener_path()
        .as_os_str()
        .to_str()
        .unwrap()
        .to_string()
}

fn escribir_archivos_index(builder: &gtk::Builder, logger: Arc<Logger>) {
    let index = leer_index(logger.clone()).unwrap();

    let label = crear_label("Archivos en staging:");
    label.style_context().add_class("verde");

    let list_box: gtk::Box = builder.object("staging").unwrap();
    list_box.add(&label);

    for objeto_index in index {
        let path = extraer_path(&objeto_index);
        let simbolo = if objeto_index.es_eliminado { "-" } else { "+" };
        let label = crear_label(&format!("{simbolo} {}", path));
        label.style_context().add_class("verde");

        let list_box: gtk::Box = builder.object("staging").unwrap();
        list_box.add(&label);
        logger.log("Gui: Agregando archivo a staging");
    }
}

fn escribir_archivos_modificados(builder: &gtk::Builder, logger: Arc<Logger>) {
    let status = Status::from(logger.clone()).unwrap();
    let lineas = status.obtener_trackeados().unwrap();

    let container: gtk::Box = builder.object("staging").unwrap();

    let label = crear_label("\nArchivos con cambios:");
    label.style_context().add_class("rojo");

    container.pack_start(&label, false, false, 0);

    for linea in lineas {
        let split = linea.split(": ").collect::<Vec<&str>>();
        let nombre = split[1];

        let label = crear_label(&format!("+ {nombre}",));
        label.style_context().add_class("rojo");

        let logger_callback = logger.clone();
        let nombre_callback = nombre.to_string();
        let builder_callback = builder.clone();
        label.connect_button_press_event(move |_, _| {
            logger_callback.log("Gui: Agregando archivo a staging");
            let mut add =
                Add::from(vec![nombre_callback.clone()], logger_callback.clone()).unwrap();
            add.ejecutar().unwrap();

            render(&builder_callback, logger_callback.clone());

            gtk::glib::Propagation::Proceed
        });

        container.pack_start(&label, false, true, 0);
    }
}

fn escribir_archivos_untrackeados(builder: &gtk::Builder, logger: Arc<Logger>) {
    let status = Status::from(logger.clone()).unwrap();
    let lineas = status.obtener_untrackeados().unwrap();

    let container: gtk::Box = builder.object("staging").unwrap();

    let label = crear_label("\nArchivos sin trackear:");
    label.style_context().add_class("rojo");

    container.pack_start(&label, false, false, 0);

    for linea in lineas {
        let label = crear_label(&format!("+ {linea}",));
        label.style_context().add_class("rojo");

        let logger_callback = logger.clone();
        let nombre_callback = linea.to_string();
        let builder_callback = builder.clone();
        label.connect_button_press_event(move |_, _| {
            logger_callback.log("Gui: Agregando archivo a staging");

            Add::from(vec![nombre_callback.clone()], logger_callback.clone()).ejecutar_gui();

            render(&builder_callback, logger_callback.clone());

            gtk::glib::Propagation::Proceed
        });

        container.pack_start(&label, false, true, 0);
    }
}

fn limpiar_archivos(builder: &gtk::Builder) {
    let container: gtk::Box = builder.object("staging").unwrap();
    container.children().iter().for_each(|child| {
        container.remove(child);
    });
}

pub fn render(builder: &gtk::Builder, logger: Arc<Logger>) {
    logger.log("Gui: Renderizando staging area");
    let container: gtk::Box = builder.object("staging").unwrap();
    limpiar_archivos(builder);
    escribir_archivos_index(builder, logger.clone());
    escribir_archivos_modificados(builder, logger.clone());
    escribir_archivos_untrackeados(builder, logger);
    container.show_all();
}

pub fn refresh(builder: &gtk::Builder) {
    let container: gtk::Box = builder.object("staging").unwrap();
    container.show_all();
}
