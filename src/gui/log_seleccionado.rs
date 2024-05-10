use gtk::prelude::*;

use crate::utils::compresion::descomprimir_objeto_gir;

fn estilar_log_info(builder: &gtk::Builder) {
    let text: gtk::TextView = builder.object("log-description").unwrap();

    text.style_context().add_class("commit-info");
    text.set_wrap_mode(gtk::WrapMode::Word);

    text.set_editable(false);
    text.set_pixels_above_lines(5);
    text.set_pixels_below_lines(5);
    text.set_pixels_inside_wrap(5);
    text.set_left_margin(5);
}

pub fn render(builder: &gtk::Builder, commit: Option<&str>) {
    let log_info: gtk::TextBuffer = builder.object("log-info").unwrap();
    let text_view: gtk::TextView = builder.object("log-description").unwrap();
    estilar_log_info(builder);
    if let Some(commit) = commit {
        let contenido = descomprimir_objeto_gir(commit).unwrap();
        let contenido_split = contenido.split('\0').collect::<Vec<&str>>();
        log_info.set_text(contenido_split[1]);
        text_view.set_justification(gtk::Justification::Left);
    } else {
        log_info.set_text("Ningun commit seleccionado");
        text_view.set_justification(gtk::Justification::Center);
    }
}
