use std::sync::Arc;

use gtk::prelude::*;

use crate::tipos_de_dato::logger::Logger;

use super::branch_dialog::{self, AccionBranchDialog};

pub fn render(builder: &gtk::Builder, logger: Arc<Logger>) {
    let event_box = builder.object::<gtk::EventBox>("rebase-box").unwrap();

    let builder_clone = builder.clone();
    event_box.connect_button_press_event(move |_, _| {
        branch_dialog::render(&builder_clone, logger.clone(), AccionBranchDialog::Rebase);
        gtk::glib::Propagation::Stop
    });
}
