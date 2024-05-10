use std::{
    path::PathBuf,
    sync::{
        mpsc::{channel, Receiver, Sender},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};

use gir::servidor::vector_threads::VectorThreads;
use gir::{
    servidor::{
        gir_server::ServidorGir, http_server::ServidorHttp, repos_almacen::ReposAlmacen,
        rutas::mensaje_servidor::MensajeServidor,
    },
    tipos_de_dato::logger::Logger,
};

const MAX_INTENTOS_REINICIO: u8 = 5;
static MINIMO_TIEMPO_DE_FUNCIONAMIENTO: u64 = 60;

fn correr_servidor(
    logger: Arc<Logger>,
    channel: (Sender<MensajeServidor>, Receiver<MensajeServidor>),
    threads: VectorThreads,
) -> Result<(), String> {
    let (tx, rx) = channel;
    let repos_almacen = ReposAlmacen::new();

    let mut intentos_gir = 0;
    let mut intentos_http = 0;

    let mut servidor_http = ServidorHttp::new(
        logger.clone(),
        threads.clone(),
        tx.clone(),
        repos_almacen.clone(),
    )?;
    servidor_http.iniciar_servidor()?;

    let mut servidor_gir = ServidorGir::new(
        logger.clone(),
        threads.clone(),
        tx.clone(),
        repos_almacen.clone(),
    )?;
    servidor_gir.iniciar_servidor()?;

    let mut ultimo_gir = Instant::now();
    let mut ultimo_http = Instant::now();

    while let Ok(error_servidor) = rx.recv() {
        match error_servidor {
            MensajeServidor::GirErrorFatal => {
                servidor_gir.reiniciar_servidor()?;
                if ultimo_gir.elapsed() < Duration::from_secs(MINIMO_TIEMPO_DE_FUNCIONAMIENTO) {
                    ultimo_gir = Instant::now();
                    intentos_gir += 1;
                }
            }
            MensajeServidor::HttpErrorFatal => {
                servidor_http.reiniciar_servidor()?;
                if ultimo_http.elapsed() < Duration::from_secs(MINIMO_TIEMPO_DE_FUNCIONAMIENTO) {
                    ultimo_http = Instant::now();
                    intentos_http += 1;
                }
            }
        };

        if intentos_gir >= MAX_INTENTOS_REINICIO || intentos_http >= MAX_INTENTOS_REINICIO {
            return Err("No se pudo reiniciar el servidor".to_owned());
        }
    }

    Ok(())
}

fn main() -> Result<(), String> {
    let logger = Arc::new(Logger::new(PathBuf::from("server_logger.txt"))?);

    let channel = channel::<MensajeServidor>();
    let threads = Arc::new(Mutex::new(Vec::new()));

    correr_servidor(logger.clone(), channel, threads.clone())?;

    if let Ok(mut threads) = threads.lock() {
        for handle in threads.drain(..) {
            let _ = handle.join();
        }
    } else {
        logger.log("Error al obtener el lock de threads desde main");
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use std::sync::mpsc::channel;

    #[test]
    fn test01_correr_servidor_rompe_luego_de_3_intentos() {
        let logger = Arc::new(Logger::new(PathBuf::from("server_logger.txt")).unwrap());

        let channel = channel::<MensajeServidor>();
        let threads = Arc::new(Mutex::new(Vec::new()));

        let result = correr_servidor(logger.clone(), channel, threads.clone());

        assert!(result.is_err());
    }
}
