use super::comando_gui::ComandoGui;
use super::{info_dialog, log_list, upstream_dialog};
use crate::tipos_de_dato::comandos::pull::Pull;
use gtk::prelude::*;
use gtk::{self};
use std::sync::Arc;

pub fn render(
    builder: &gtk::Builder,
    logger: Arc<crate::tipos_de_dato::logger::Logger>,
    branch_actual: String,
) {
    let pull_button = builder.object::<gtk::Button>("pull-button").unwrap();

    let builder_clone = builder.clone();
    pull_button.connect_clicked(move |_| {
        upstream_dialog::render(&builder_clone, logger.clone());
        let resultado = Pull::from(Vec::new(), logger.clone()).ejecutar_gui();

        if let Some(resultado) = resultado {
            info_dialog::mostrar_mensaje(&resultado, "");
            log_list::render(&builder_clone, &branch_actual, logger.clone());
        }
    });
}
