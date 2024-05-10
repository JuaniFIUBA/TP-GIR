use std::sync::Arc;

use gtk::{prelude::*, Button};

use crate::{
    tipos_de_dato::{
        comando::Ejecutar,
        comandos::{add::Add, merge::Merge},
        logger::Logger,
        objeto::Objeto,
    },
    utils::{index::leer_index, io::leer_a_string},
};

pub fn boton_conflictos(builder: &gtk::Builder, logger: Arc<Logger>) {
    let boton: Button = builder.object("conflicts-button").unwrap();

    let deshabilidato = Merge::hay_archivos_sin_mergear(logger.clone()).unwrap();
    boton.set_sensitive(deshabilidato);

    let builder = builder.clone();
    boton.connect_clicked(move |_| {
        modal(&builder, logger.clone());
    });
    boton.show_all();
}

fn resaltar_linea(buffer: &gtk::TextBuffer, numero_linea: i32, tag: &str) {
    limpiar_linea(buffer, numero_linea);
    let start_iter = buffer.iter_at_line(numero_linea);
    let end_iter = buffer.iter_at_line(numero_linea + 1);
    buffer.apply_tag_by_name(tag, &start_iter, &end_iter);
}

fn limpiar_linea(buffer: &gtk::TextBuffer, numero_linea: i32) {
    let start_iter = buffer.iter_at_line(numero_linea);
    let end_iter = buffer.iter_at_line(numero_linea + 1);
    buffer.remove_all_tags(&start_iter, &end_iter);
}

fn crear_tags(buffer: &gtk::TextBuffer) {
    let head_titulo = gtk::TextTag::new(Some("head_titulo"));
    head_titulo.set_paragraph_background(Some("#5eead4"));
    let head_contenido = gtk::TextTag::new(Some("head_contenido"));
    head_contenido.set_paragraph_background(Some("#99f6e4"));
    let incoming_titulo = gtk::TextTag::new(Some("incoming_titulo"));
    incoming_titulo.set_paragraph_background(Some("#67e8f9"));
    let incoming_contenido = gtk::TextTag::new(Some("incoming_contenido"));
    incoming_contenido.set_paragraph_background(Some("#a5f3fc"));
    let table = buffer.tag_table().unwrap();
    table.add(&head_titulo);
    table.add(&head_contenido);
    table.add(&incoming_titulo);
    table.add(&incoming_contenido);
}

#[derive(PartialEq)]
enum Estado {
    Head,
    Incoming,
    None,
}

fn resaltar_conflictos(buffer: &gtk::TextBuffer) {
    let texto = buffer
        .text(&buffer.start_iter(), &buffer.end_iter(), false)
        .unwrap();
    let lineas = texto.split('\n').collect::<Vec<&str>>();
    let mut estado = Estado::None;
    for (i, linea) in lineas.iter().enumerate() {
        match *linea {
            l if l.starts_with("<<<<<<") => {
                resaltar_linea(buffer, i as i32, "head_titulo");
                estado = Estado::Head;
                continue;
            }
            l if l.starts_with(">>>>>>") => {
                if estado != Estado::None {
                    resaltar_linea(buffer, i as i32, "incoming_titulo");
                    estado = Estado::None;
                    continue;
                }
            }
            "======" => {
                if estado != Estado::None {
                    estado = Estado::Incoming;
                    continue;
                }
            }
            _ => {}
        }
        match estado {
            Estado::Head => {
                resaltar_linea(buffer, i as i32, "head_contenido");
            }
            Estado::Incoming => {
                resaltar_linea(buffer, i as i32, "incoming_contenido");
            }
            Estado::None => {
                limpiar_linea(buffer, i as i32);
            }
        }
    }
}

fn crear_text_area_de_objeto(objeto: &Objeto) -> gtk::ScrolledWindow {
    let scrollable_window = gtk::ScrolledWindow::new(gtk::Adjustment::NONE, gtk::Adjustment::NONE);
    scrollable_window.set_shadow_type(gtk::ShadowType::None);
    scrollable_window.set_height_request(400);
    scrollable_window.set_width_request(600);
    let viewport = gtk::Viewport::new(gtk::Adjustment::NONE, gtk::Adjustment::NONE);
    let text = gtk::TextView::new();

    scrollable_window.add(&viewport);
    viewport.add(&text);

    text.set_left_margin(5);
    text.set_right_margin(5);
    text.set_top_margin(5);
    text.set_bottom_margin(5);
    text.set_monospace(true);
    let contenido = leer_a_string(objeto.obtener_path()).unwrap();
    let buffer = text.buffer().unwrap();
    buffer.set_text(&contenido);
    crear_tags(&buffer);
    resaltar_conflictos(&buffer);
    buffer.connect_changed(|buffer| {
        resaltar_conflictos(buffer);
    });

    scrollable_window
}

fn guardar_y_addear_archivo_activo(builder: &gtk::Builder, logger: Arc<Logger>) {
    let notebook: gtk::Notebook = builder.object("conflicts-notebook").unwrap();
    let tab_indice = match notebook.current_page() {
        Some(tab_indice) => tab_indice,
        None => return,
    };
    let tab_activa = notebook.nth_page(Some(tab_indice)).unwrap();
    let nombre_archivo = notebook.tab_label_text(&tab_activa).unwrap();
    let scrolled_window: gtk::ScrolledWindow = tab_activa.downcast().unwrap();
    let viewport: gtk::Viewport = scrolled_window.child().unwrap().downcast().unwrap();
    let text: gtk::TextView = viewport.child().unwrap().downcast().unwrap();
    let buffer = text.buffer().unwrap();
    let contenido = buffer
        .text(&buffer.start_iter(), &buffer.end_iter(), false)
        .unwrap();
    std::fs::write(&nombre_archivo, contenido).unwrap();

    let mut add = Add::from(vec![nombre_archivo.to_string()], logger).unwrap();
    add.ejecutar().unwrap();

    notebook.remove_page(Some(tab_indice));
    if notebook.n_pages() == 0 {
        let modal: gtk::Window = builder.object("conflicts-window").unwrap();
        let boton_conflictos: Button = builder.object("conflicts-button").unwrap();

        boton_conflictos.set_sensitive(false);
        boton_conflictos.show();
        modal.hide();
    }
}

fn boton_marcar_resuelto(builder: &gtk::Builder, logger: Arc<Logger>) {
    let boton: Button = builder.object("marcar-resuelto").unwrap();

    let builder_clone = builder.clone();
    boton.connect_clicked(move |_| {
        guardar_y_addear_archivo_activo(&builder_clone, logger.clone());
    });
}

fn crear_notebook(builder: &gtk::Builder, logger: Arc<Logger>) {
    let index = leer_index(logger).unwrap();
    let sin_mergear: Vec<_> = index.iter().filter(|objeto| objeto.merge).collect();
    let notebook: gtk::Notebook = builder.object("conflicts-notebook").unwrap();
    notebook.set_vexpand(true);
    let pages = notebook.n_pages();
    for _ in 0..pages {
        notebook.remove_page(Some(0));
    }

    for objeto in sin_mergear {
        let text_area = crear_text_area_de_objeto(&objeto.objeto);
        let label = gtk::Label::new(Some(
            objeto.objeto.obtener_path().to_string_lossy().as_ref(),
        ));
        notebook.append_page(&text_area, Some(&label));
    }
}

fn modal(builder: &gtk::Builder, logger: Arc<Logger>) {
    let modal: gtk::Window = builder.object("conflicts-window").unwrap();
    modal.set_position(gtk::WindowPosition::Center);
    crear_notebook(builder, logger.clone());

    modal.connect_delete_event(move |modal, _| {
        modal.hide();
        gtk::glib::Propagation::Stop
    });
    modal.set_position(gtk::WindowPosition::Center);
    modal.show_all();
}

pub fn render(builder: &gtk::Builder, logger: Arc<Logger>) {
    boton_conflictos(builder, logger.clone());
    boton_marcar_resuelto(builder, logger.clone());
}
