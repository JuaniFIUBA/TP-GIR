use gtk::prelude::*;

pub fn setup(build: &gtk::Builder) {
    let dialog: gtk::MessageDialog = build.object("error-dialog").unwrap();
    let aceptar_button: gtk::Button = build.object("error-close").unwrap();
    dialog.set_position(gtk::WindowPosition::Center);

    let dialog_clone = dialog.clone();
    aceptar_button.connect_clicked(move |_| {
        dialog_clone.hide();
    });
}

pub fn mostrar_mensaje(titulo: &str, error: &str) {
    let glade_src = include_str!("glade1.glade");
    let builder = gtk::Builder::from_string(glade_src);
    let dialog: gtk::MessageDialog = builder.object("error-dialog").unwrap();
    let error_label: gtk::Label = builder.object("error-label").unwrap();
    let aceptar_button: gtk::Button = builder.object("error-close").unwrap();
    let titulo_label: gtk::Label = builder.object("error-title").unwrap();

    titulo_label.set_text(titulo);
    error_label.set_text(error);
    dialog.set_position(gtk::WindowPosition::Center);

    let dialog_clone = dialog.clone();
    aceptar_button.connect_clicked(move |_| {
        dialog_clone.hide();
    });
    dialog.run();
}

pub fn mostrar_error(error: &str) {
    mostrar_mensaje("Error", error)
}
