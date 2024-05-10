use std::env::args;
use std::sync::Arc;

use gir::{
    tipos_de_dato::{comando::Comando, logger::Logger},
    utils::gir_config::conseguir_ubicacion_log_config,
};

fn main() -> Result<(), String> {
    let logger = Arc::new(Logger::new(conseguir_ubicacion_log_config()?)?);

    let mut argv = args().collect::<Vec<String>>();
    argv.remove(0);

    if argv.is_empty() {
        return Err("Ningun comando ingresado".to_string());
    }

    if argv[0] == "gui" {
        if argv.len() > 1 {
            argv.remove(0);
            return Err(format!("Opcion desconocida: {}", argv.join(" ")));
        }

        println!("Iniciando GUI...");
        gir::gui::ejecutar(logger.clone());
        return Ok(());
    }

    let mut comando = match Comando::new(argv, logger.clone()) {
        Ok(comando) => comando,
        Err(err) => {
            println!("ERROR: {}\n", err);
            logger.log(&err);
            return Ok(());
        }
    };

    match comando.ejecutar() {
        Ok(mensaje) => {
            println!("{}", mensaje.clone());
            logger.log(&mensaje);
        }
        Err(mensaje) => {
            println!("ERROR: {}", mensaje);
            logger.log(&mensaje);
        }
    }
    Ok(())
}
