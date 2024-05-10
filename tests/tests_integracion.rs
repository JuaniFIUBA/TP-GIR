use std::{path::PathBuf, sync::Arc};

use gir::{
    tipos_de_dato::{comando::Comando, logger::Logger},
    utils::{io, testing::eliminar_archivo_gir},
};
use serial_test::serial;

const VERDE: &str = "\x1B[32m";
const RESET: &str = "\x1B[0m";

#[test]
#[serial]
fn init_addear_committear_log() {
    eliminar_archivo_gir();
    let logger = Arc::new(Logger::new(PathBuf::from("tmp/init_addear_committear_log")).unwrap());
    let args_init = vec!["init".to_string()];
    Comando::new(args_init, logger.clone())
        .unwrap()
        .ejecutar()
        .unwrap();

    io::escribir_bytes("tmp/init_addear_committear_log", "contenido").unwrap();
    let args_add = vec![
        "add".to_string(),
        "tmp/init_addear_committear_log".to_string(),
    ];
    Comando::new(args_add, logger.clone())
        .unwrap()
        .ejecutar()
        .unwrap();

    let args_commit = vec![
        "commit".to_string(),
        "-m".to_string(),
        "mensaje".to_string(),
    ];
    Comando::new(args_commit, logger.clone())
        .unwrap()
        .ejecutar()
        .unwrap();

    let args_log = vec!["log".to_string()];
    let log = Comando::new(args_log, logger.clone())
        .unwrap()
        .ejecutar()
        .unwrap();

    let mensaje = log.split("\n\n").collect::<Vec<&str>>();

    assert_eq!(mensaje.len(), 3);
    assert_eq!(mensaje[1], "     mensaje");
}

#[test]
#[serial]
fn init_addear_status() {
    eliminar_archivo_gir();
    let logger = Arc::new(Logger::new(PathBuf::from("tmp/init_addear_status")).unwrap());
    let args_init = vec!["init".to_string()];
    Comando::new(args_init, logger.clone())
        .unwrap()
        .ejecutar()
        .unwrap();

    io::escribir_bytes("tmp/init_addear_status", "contenido").unwrap();
    let args_add = vec!["add".to_string(), "tmp/init_addear_status".to_string()];
    Comando::new(args_add, logger.clone())
        .unwrap()
        .ejecutar()
        .unwrap();

    let args_status = vec!["status".to_string()];
    let status = Comando::new(args_status, logger.clone())
        .unwrap()
        .ejecutar()
        .unwrap();

    let mensaje = status.split('\n').collect::<Vec<&str>>();

    assert_eq!(mensaje[0], "Cambios a ser commiteados:");
    assert_eq!(
        mensaje[1],
        format!("         {VERDE}nuevo archivo: tmp/init_addear_status{RESET}")
    );
    assert_eq!(mensaje[2], "");
    assert_eq!(mensaje[3], "Archivos unmergeados:");
    assert_eq!(mensaje[4], "");
    assert_eq!(mensaje[5], "Cambios no en zona de preparacion:");
    assert_eq!(mensaje[6], "");
    assert_eq!(mensaje[7], "Cambios no trackeados:");
}
