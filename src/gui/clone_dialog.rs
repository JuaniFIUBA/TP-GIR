use std::{path::PathBuf, sync::Arc};

use gtk::prelude::*;

use crate::tipos_de_dato::{comandos::clone::Clone, logger::Logger};

use super::comando_gui::ComandoGui;

fn run_dialog(builder: &gtk::Builder) {
    let dialog: gtk::MessageDialog = builder.object("clone").unwrap();
    dialog.run();
    dialog.hide();
}

fn clonar_dialog(builder: &gtk::Builder, logger: Arc<Logger>) {
    let confirm: gtk::Button = builder.object("confirm-clone").unwrap();
    let dialog: gtk::MessageDialog = builder.object("clone").unwrap();
    let input: gtk::Entry = builder.object("clone-input").unwrap();
    dialog.set_position(gtk::WindowPosition::Center);

    confirm.connect_clicked(move |_| {
        Clone::from(&mut vec![input.text().to_string()], logger.clone(), true).ejecutar_gui();
        input.set_text("");
        dialog.hide();
    });
}

fn error_no_repo_dialog(builder: &gtk::Builder) {
    let dialog: gtk::MessageDialog = builder.object("no-repo-dialog").unwrap();
    let aceptar_button: gtk::Button = builder.object("no-repo-close").unwrap();
    dialog.set_position(gtk::WindowPosition::Center);

    let dialog_clone = dialog.clone();
    aceptar_button.connect_clicked(move |_| {
        dialog_clone.hide();
    });

    dialog.run();
}

pub fn render(builder: &gtk::Builder, logger: Arc<Logger>) -> bool {
    clonar_dialog(builder, logger);
    run_dialog(builder);

    if !PathBuf::from(".gir").is_dir() {
        error_no_repo_dialog(builder);
        return false;
    }
    true
}
