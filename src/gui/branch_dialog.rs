use std::{path::PathBuf, sync::Arc};

use gtk::prelude::*;

use super::{comando_gui::ComandoGui, conflicts_modal, dibujar_dialog, info_dialog, log_list};
use crate::{
    tipos_de_dato::{
        comandos::{branch::Branch, merge::Merge, rebase::Rebase},
        logger::Logger,
    },
    utils::ramas,
};

pub enum AccionBranchDialog {
    Merge,
    Rebase,
}

fn obtener_ramas_disponibles() -> Vec<String> {
    let todas = match Branch::mostrar_ramas() {
        Ok(ramas) => ramas,
        Err(err) => {
            info_dialog::mostrar_error(&err);
            return vec![];
        }
    };
    return todas
        .lines()
        .filter_map(|rama| {
            if rama.starts_with('*') {
                None
            } else {
                Some(rama.trim().to_string())
            }
        })
        .collect();
}

fn combo_box(builder: &gtk::Builder) {
    let combobox: gtk::ComboBoxText = builder.object("branch-combo-box").unwrap();
    let ramas = obtener_ramas_disponibles();
    combobox.remove_all();
    for rama in ramas {
        combobox.append_text(&rama);
    }
}

fn comfirmar(builder: &gtk::Builder, accion: AccionBranchDialog, logger: Arc<Logger>) {
    let confirmar = builder.object::<gtk::Button>("confirm-branc").unwrap();
    let builder_clone = builder.clone();
    confirmar.connect_clicked(move |_| {
        let dialog = builder_clone
            .object::<gtk::Dialog>("branch-dialog")
            .unwrap();
        let combobox: gtk::ComboBoxText = builder_clone.object("branch-combo-box").unwrap();
        let activo = match combobox.active_text() {
            Some(activo) => activo,
            None => return,
        };
        let mut args = vec![activo.to_string()];
        match accion {
            AccionBranchDialog::Merge => Merge::from(&mut args, logger.clone()).ejecutar_gui(),
            AccionBranchDialog::Rebase => {
                if PathBuf::from(".gir/rebase-merge").exists() {
                    args = vec!["--continue".to_string()];
                }
                Rebase::from(args, logger.clone()).ejecutar_gui()
            }
        };

        conflicts_modal::boton_conflictos(&builder_clone, logger.clone());

        dialog.hide();
    });
}

fn cancelar(builder: &gtk::Builder) {
    let cancelar = builder.object::<gtk::Button>("cancel-branc").unwrap();
    let builder_clone = builder.clone();
    cancelar.connect_clicked(move |_| {
        let dialog = builder_clone
            .object::<gtk::Dialog>("branch-dialog")
            .unwrap();
        dialog.hide();
    });
}

pub fn render(builder: &gtk::Builder, logger: Arc<Logger>, accion: AccionBranchDialog) {
    let dialog = builder.object::<gtk::Dialog>("branch-dialog").unwrap();
    dialog.set_position(gtk::WindowPosition::Center);

    combo_box(builder);
    comfirmar(builder, accion, logger.clone());
    cancelar(builder);

    dialog.connect_delete_event(move |dialog, _| {
        dialog.hide();
        gtk::glib::Propagation::Stop
    });

    dibujar_dialog(&dialog);
    conflicts_modal::boton_conflictos(builder, logger.clone());
    let branch = ramas::obtener_rama_actual().unwrap();
    log_list::render(builder, &branch, logger.clone());
}
