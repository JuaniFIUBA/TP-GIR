use std::sync::Arc;

use gtk::prelude::*;

use crate::{
    tipos_de_dato::{comandos::commit::Commit, logger::Logger},
    utils::ramas,
};

use super::{comando_gui::ComandoGui, log_list, staging_area};

fn run_dialog(builder: &gtk::Builder) {
    let commit_button: gtk::Button = builder.object("commit-button").unwrap();
    let dialog: gtk::MessageDialog = builder.object("commit").unwrap();

    dialog.set_position(gtk::WindowPosition::Center);

    commit_button.connect_clicked(move |_| {
        dialog.run();
        dialog.hide();
    });
}

fn boton_cancel_dialog(builder: &gtk::Builder) {
    let cancel: gtk::Button = builder.object("cancel-commit").unwrap();
    let dialog: gtk::MessageDialog = builder.object("commit").unwrap();

    cancel.connect_clicked(move |_| {
        dialog.hide();
    });
}

fn boton_confimar_dialog(builder: &gtk::Builder, logger: Arc<Logger>) {
    let confirm: gtk::Button = builder.object("confirm-commit").unwrap();
    let dialog: gtk::MessageDialog = builder.object("commit").unwrap();
    let input: gtk::Entry = builder.object("commit-input").unwrap();
    let builder_clone = builder.clone();
    confirm.connect_clicked(move |_| {
        let mut args = match input.text().as_str() {
            "" => vec![],
            input_str => vec!["-m".to_string(), input_str.to_string()],
        };

        let commit = Commit::from(&mut args, logger.clone()).ejecutar_gui();

        if commit.is_none() {
            return;
        }

        let branch_actual = ramas::obtener_rama_actual().unwrap();

        log_list::render(&builder_clone, &branch_actual, logger.clone());
        staging_area::render(&builder_clone, logger.clone());
        input.set_text("");
        dialog.hide();
    });
}

pub fn render(builder: &gtk::Builder, logger: Arc<Logger>) {
    run_dialog(builder);

    boton_cancel_dialog(builder);
    boton_confimar_dialog(builder, logger);
}
