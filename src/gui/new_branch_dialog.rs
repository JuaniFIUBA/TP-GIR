use std::sync::Arc;

use gtk::prelude::*;

use crate::tipos_de_dato::{comandos::branch::Branch, logger::Logger};

use super::{branch_selector, comando_gui::ComandoGui};

fn run_dialog(builder: &gtk::Builder) {
    let branch_button: gtk::Button = builder.object("branch-button").unwrap();
    let dialog: gtk::MessageDialog = builder.object("branch").unwrap();

    dialog.set_position(gtk::WindowPosition::Center);

    branch_button.connect_clicked(move |_| {
        dialog.run();
        dialog.hide();
    });
}

fn boton_cancel_dialog(builder: &gtk::Builder) {
    let cancel: gtk::Button = builder.object("cancel-branch").unwrap();
    let dialog: gtk::MessageDialog = builder.object("branch").unwrap();

    cancel.connect_clicked(move |_| {
        dialog.hide();
    });
}

fn boton_confimar_dialog(builder: &gtk::Builder, logger: Arc<Logger>) {
    let confirm: gtk::Button = builder.object("confirm-branch").unwrap();
    let dialog: gtk::MessageDialog = builder.object("branch").unwrap();
    let input: gtk::Entry = builder.object("branch-input").unwrap();

    let builder_clone = builder.clone();
    confirm.connect_clicked(move |_| {
        let branch =
            Branch::from(&mut vec![input.text().to_string()], logger.clone()).ejecutar_gui();

        if branch.is_none() {
            return;
        }

        branch_selector::render(&builder_clone, logger.clone());
        input.set_text("");
        dialog.hide();
    });
}

pub fn render(builder: &gtk::Builder, logger: Arc<Logger>) {
    run_dialog(builder);

    boton_cancel_dialog(builder);
    boton_confimar_dialog(builder, logger);
}
