pub mod branch_dialog;
mod branch_selector;
mod cat_file_button;
mod check_ignore_button;
mod clone_dialog;
mod comando_gui;
mod conflicts_modal;
mod hash_object_button;
mod info_dialog;
mod log_list;
mod log_seleccionado;
mod ls_files_button;
mod ls_tree_button;
mod merge_button;
mod new_branch_dialog;
mod new_commit_dialog;
mod pull_button;
mod push_button;
mod rebase_button;
mod refresh;
mod remote_button;
mod rm_button;
mod show_ref_button;
mod staging_area;
mod tag_button;
mod tag_list;
mod upstream_dialog;

use std::path::PathBuf;
use std::sync::Arc;

use crate::tipos_de_dato::logger::Logger;
use crate::utils::ramas;
use gtk::{self, StyleContext};
use gtk::{gdk, prelude::*};

/// Dibuja el dialogo 3 veces para que se vea bien, de forma contraria
/// en ocasiones no se dibujan los bordes del mismo
/// esto es por un bug en la propiedad "Transient for" de gtk
pub fn dibujar_dialog<T>(dialog: &T)
where
    T: IsA<gtk::Dialog>,
    T: gtk::prelude::IsA<gtk::Widget>,
{
    dialog.show_all();
    dialog.hide();
    dialog.show_all();
    dialog.hide();
    dialog.run();
}

fn hidratar_componentes(builder: &gtk::Builder, logger: Arc<Logger>, branch_actual: &str) {
    let screen = gdk::Screen::default().unwrap();
    estilos(screen);
    new_branch_dialog::render(builder, logger.clone());
    branch_selector::render(builder, logger.clone());
    log_list::render(builder, branch_actual, logger.clone());
    log_seleccionado::render(builder, None);
    staging_area::render(builder, logger.clone());
    new_commit_dialog::render(builder, logger.clone());
    push_button::render(builder, logger.clone());
    info_dialog::setup(builder);
    pull_button::render(builder, logger.clone(), branch_actual.to_string());
    conflicts_modal::render(builder, logger.clone());
    refresh::render(builder, logger.clone());
    merge_button::render(builder, logger.clone());
    rebase_button::render(builder, logger.clone());
    hash_object_button::render(builder, logger.clone());
    cat_file_button::render(builder, logger.clone());
    check_ignore_button::render(builder, logger.clone());
    show_ref_button::render(builder, logger.clone());
    ls_tree_button::render(builder, logger.clone());
    ls_files_button::render(builder, logger.clone());
    rm_button::render(builder, logger.clone());
    tag_list::render(builder, logger.clone());
    tag_button::render(builder, logger.clone());
    remote_button::render(builder, logger.clone());
}

pub fn estilos(screen: gdk::Screen) {
    let css_provider = gtk::CssProvider::new();
    css_provider
        .load_from_data(include_str!("estilos.css").as_bytes())
        .unwrap();

    StyleContext::add_provider_for_screen(
        &screen,
        &css_provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}

pub fn ejecutar(logger: Arc<Logger>) {
    if gtk::init().is_err() {
        println!("Failed to initialize GTK.");
        return;
    }

    std::env::set_var("GSK_RENDERER", "cairo");

    let glade_src = include_str!("glade1.glade");
    let builder = gtk::Builder::from_string(glade_src);
    let window: gtk::Window = builder.object("home-v2").unwrap();
    window.set_position(gtk::WindowPosition::Center);
    window.set_default_size(800, 600);

    if !PathBuf::from(".gir").is_dir() && !clone_dialog::render(&builder, logger.clone()) {
        return;
    }

    let branch_actual = ramas::obtener_rama_actual().unwrap();

    hidratar_componentes(&builder, logger.clone(), &branch_actual);

    window.show_all();
    tag_list::render(&builder, logger.clone());

    window.connect_delete_event(|_, _| {
        gtk::main_quit();
        gtk::glib::Propagation::Proceed
    });

    gtk::main();
}
