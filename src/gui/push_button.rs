use gtk::prelude::*;
use gtk::{self};
use std::sync::Arc;

use crate::gui::comando_gui::ComandoGui;
use crate::tipos_de_dato::comandos::push::Push;

use crate::tipos_de_dato::logger::Logger;

use super::{info_dialog, upstream_dialog};

pub fn render(builder: &gtk::Builder, logger: Arc<Logger>) {
    let push_button = builder.object::<gtk::Button>("push-button").unwrap();

    let builder_clone = builder.clone();
    push_button.connect_clicked(move |_| {
        upstream_dialog::render(&builder_clone, logger.clone());

        let resultado = Push::new(&mut Vec::new(), logger.clone()).ejecutar_gui();

        if let Some(resultado) = resultado {
            info_dialog::mostrar_mensaje(&resultado, "");
        }
    });
}
