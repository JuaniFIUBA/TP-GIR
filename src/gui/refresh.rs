use std::sync::Arc;

use gtk::prelude::*;

use crate::{tipos_de_dato::logger::Logger, utils::ramas};

use super::hidratar_componentes;

pub fn render(builder: &gtk::Builder, logger: Arc<Logger>) {
    let icon = builder.object::<gtk::Button>("refresh-button").unwrap();

    let builder = builder.clone();
    icon.connect_clicked(move |_| {
        let branch_actual = ramas::obtener_rama_actual().unwrap();
        hidratar_componentes(&builder, logger.clone(), &branch_actual);
    });
    icon.show_all();
}
