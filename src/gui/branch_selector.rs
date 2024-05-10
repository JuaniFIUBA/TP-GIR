use std::sync::Arc;

use gtk::prelude::*;

use crate::{
    tipos_de_dato::{
        comandos::{branch::Branch, checkout::Checkout},
        logger::Logger,
    },
    utils::ramas,
};

use super::{comando_gui::ComandoGui, info_dialog, log_list, log_seleccionado, staging_area};

pub fn render(builder: &gtk::Builder, logger: Arc<Logger>) {
    let select: gtk::ComboBoxText = builder.object("select-branch").unwrap();
    let branch_actual = ramas::obtener_rama_actual().unwrap();
    select.remove_all();

    let branches = match Branch::obtener_ramas() {
        Ok(branches) => branches,
        Err(err) => {
            info_dialog::mostrar_error(&err);
            return;
        }
    };

    let mut i = 0;
    branches.iter().for_each(|branch| {
        if branch.is_empty() {
            return;
        }
        select.append_text(branch);
        if *branch == branch_actual {
            select.set_active(Some(i));
        }
        i += 1;
    });

    let builder_clone = builder.clone();
    select.connect_changed(move |a| {
        let active = match a.active_text() {
            Some(text) => text,
            None => return,
        };

        log_list::render(&builder_clone, active.as_str(), logger.clone());
        log_seleccionado::render(&builder_clone, None);

        Checkout::from(vec![active.to_string()], logger.clone()).ejecutar_gui();

        log_list::refresh(&builder_clone);
        staging_area::refresh(&builder_clone);
    });
}
