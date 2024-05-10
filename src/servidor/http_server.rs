use std::{
    io::{BufReader, Read, Write},
    net::TcpListener,
    sync::{mpsc::Sender, Arc},
    thread,
};

use crate::{
    tipos_de_dato::{
        http::{
            endpoint::Endpoint, error::ErrorHttp, estado::EstadoHttp, request::Request,
            response::Response,
        },
        logger::Logger,
    },
    utils::gir_config,
};

use super::{
    repos_almacen::ReposAlmacen,
    rutas::{
        actualizar_pull_request, crear_pull_request, listar_pull_request,
        mensaje_servidor::MensajeServidor, mergear_pull_request, obtener_commits_pull_request,
        obtener_pull_request,
    },
    vector_threads::VectorThreads,
};

pub struct ServidorHttp {
    /// Canal para escuchar las conexiones de clientes
    listener: TcpListener,

    /// Logger para registrar los eventos del servidor
    logger: Arc<Logger>,

    main: Option<thread::JoinHandle<()>>,

    threads: VectorThreads,

    tx: Sender<MensajeServidor>,

    repos_almacen: ReposAlmacen,
}

impl ServidorHttp {
    /// # Argumentos:
    /// * `address` - Direccion en la que se va a escuchar las conexiones de los clientes
    /// * `logger` - Logger para registrar los eventos del servidor
    pub fn new(
        logger: Arc<Logger>,
        threads: VectorThreads,
        tx: Sender<MensajeServidor>,
        repos_almacen: ReposAlmacen,
    ) -> Result<Self, String> {
        let puerto = gir_config::conseguir_puerto_http()
            .ok_or("No se pudo conseguir el puerto http, revise el archivo config")?;

        let address = "127.0.0.1:".to_owned() + &puerto;

        let listener = TcpListener::bind(&address).map_err(|e| e.to_string())?;
        println!("Escuchando servidor HTTP en {}", address);
        logger.log("Servidor iniciado");

        Ok(Self {
            listener,
            logger,
            threads,
            main: None,
            tx,
            repos_almacen,
        })
    }

    fn agregar_endpoints(endpoints: &mut Vec<Endpoint>) {
        crear_pull_request::agregar_a_router(endpoints);
        listar_pull_request::agregar_a_router(endpoints);
        obtener_pull_request::agregar_a_router(endpoints);
        obtener_commits_pull_request::agregar_a_router(endpoints);
        actualizar_pull_request::agregar_a_router(endpoints);
        mergear_pull_request::agregar_a_router(endpoints);
    }

    fn aceptar_conexiones(
        endpoints: Arc<Vec<Endpoint>>,
        listener: TcpListener,
        threads: VectorThreads,
        logger: Arc<Logger>,
        tx: Sender<MensajeServidor>,
        repos_almacen: ReposAlmacen,
    ) {
        while let Ok((mut stream, socket)) = listener.accept() {
            logger.log(&format!("Se conecto un cliente por http desde {}", socket));

            let logger_clone = logger.clone();
            let endpoints = endpoints.clone();
            let repos_almacen = repos_almacen.clone();
            let handle = thread::spawn(move || -> Result<(), String> {
                let response = Self::manejar_cliente(
                    logger_clone.clone(),
                    &mut stream,
                    &endpoints,
                    repos_almacen,
                );

                match response {
                    Ok(response) => response.enviar(&mut stream).map_err(|e| e.to_string()),
                    Err(error_http) => {
                        logger_clone.log(&format!("Error procesando request: {:?}", error_http));
                        let response = Response::from_error(logger_clone.clone(), error_http);
                        response.enviar(&mut stream).map_err(|e| e.to_string())
                    }
                }?;

                Ok(())
            });

            let threads = threads.lock();

            if let Ok(mut threads) = threads {
                threads.push(handle);
            } else {
                logger.log("Error al obtener el lock de threads");
            }
        }

        tx.send(MensajeServidor::HttpErrorFatal)
            .expect("Error al enviar mensaje de error fatal al servidor");
    }

    pub fn reiniciar_servidor(&mut self) -> Result<(), String> {
        self.logger.log("Reiniciando servidor http");
        self.main.take();
        self.iniciar_servidor()
    }

    /// Pone en funcionamiento el servidor, spawneando un thread por cada cliente que se conecte al mismo.
    /// Procesa el pedido del cliente y responde en consecuencia.
    pub fn iniciar_servidor(&mut self) -> Result<(), String> {
        let logger = self.logger.clone();
        let listener = self.listener.try_clone().map_err(|e| e.to_string())?;
        let threads = self.threads.clone();
        let tx = self.tx.clone();
        let repos_almacen = self.repos_almacen.clone();
        let main = thread::spawn(|| {
            let mut endpoints = Vec::new();
            Self::agregar_endpoints(&mut endpoints);
            let endpoints = Arc::new(endpoints);
            Self::aceptar_conexiones(endpoints, listener, threads, logger, tx, repos_almacen);
        });

        self.main.replace(main);
        Ok(())
    }

    fn manejar_cliente<R: Read + Write>(
        logger: Arc<Logger>,
        stream: &mut R,
        endpoints: &Vec<Endpoint>,
        repos_almacen: ReposAlmacen,
    ) -> Result<Response, ErrorHttp> {
        let mut reader = BufReader::new(stream);
        let request = Request::from(&mut reader, logger.clone())?;
        for endpoint in endpoints {
            if endpoint.metodo != request.metodo {
                continue;
            }
            let params = match endpoint.matchea_con_patron(&request.ruta) {
                Some(params) => params,
                None => continue,
            };

            let repo = params.get("repo").ok_or_else(|| {
                ErrorHttp::InternalServerError(
                    "No se ha encontrado el nombre del repositorio".to_string(),
                )
            })?;

            let mutex = repos_almacen
                .obtener_mutex_del_repo(repo)
                .map_err(|e| ErrorHttp::InternalServerError(e))?;

            let _lock = mutex
                .lock()
                .map_err(|e| ErrorHttp::InternalServerError(e.to_string()))?;

            let response = (endpoint.handler)(request, params, logger.clone())?;

            return Ok(response);
        }

        let response = Response::new(logger, EstadoHttp::NotFound, None);
        Ok(response)
    }
}

#[cfg(test)]
mod test {
    use std::{path::PathBuf, sync::Mutex};

    use super::*;
    use crate::{
        servidor::gir_server::ServidorGir,
        utils::{
            io,
            testing::{self, crear_repo_para_pr},
        },
    };
    const RUTA_RAIZ: &str = env!("CARGO_MANIFEST_DIR");
    const NOMBRE_REPOSITORIO: &str = "repo";
    const RUTA_REPOSITORIO: &str = "/srv/repo/";

    fn iniciar_servidor_pushear_pr_y_obtener_respuesta_final(
        logger: Arc<Logger>,
        ruta_especifica: &str,
        request: &str,
    ) -> Response {
        let (tx, _) = std::sync::mpsc::channel();
        let logger_clone = logger.clone();
        let repos_almacen = ReposAlmacen::new();
        let repos_almacen_clone = repos_almacen.clone();
        let handle = std::thread::spawn(move || {
            let threads: VectorThreads = Arc::new(Mutex::new(Vec::new()));
            let listener = TcpListener::bind("127.0.0.1:9933").unwrap();

            let mut servidor_gir = ServidorGir {
                listener,
                threads,
                logger: logger_clone,
                main: None,
                tx,
                repos_almacen: repos_almacen_clone,
            };
            servidor_gir.iniciar_servidor().unwrap();
        });

        if handle.is_finished() {
            panic!("No se pudo iniciar el servidor");
        }
        std::thread::sleep(std::time::Duration::from_secs(1));

        let _ = io::rm_directorio(RUTA_RAIZ.to_string() + ruta_especifica);
        let _ = io::rm_directorio(RUTA_RAIZ.to_string() + RUTA_REPOSITORIO);
        io::crear_directorio(RUTA_RAIZ.to_string() + ruta_especifica).unwrap();
        io::cambiar_directorio(RUTA_RAIZ.to_string() + ruta_especifica).unwrap();

        crear_repo_para_pr(logger.clone());
        std::thread::sleep(std::time::Duration::from_secs(1));

        let repo = "repo";
        let body = r#"{
            "title": "Feature X: Implement new functionality",
            "head": "juani:master",
            "base": "rama",
            "body": "This is the description of the pull request."
        }"#;
        let content_length = body.len();

        let request_string = format!(
            "POST /repos/{}/pulls HTTP/1.1\r\n\
            Host: localhost:9933\r\n\
            Accept: application/vnd.github+json\r\n\
            Content-Type: application/json\r\n\
            Content-Length: {}\r\n\
            \r\n\
            {}",
            repo, content_length, body
        );

        let mut mock = testing::MockTcpStream {
            lectura_data: request_string.as_bytes().to_vec(),
            escritura_data: vec![],
        };
        io::cambiar_directorio(RUTA_RAIZ).unwrap();
        let mut endpoints = Vec::new();
        ServidorHttp::agregar_endpoints(&mut endpoints);
        let _ = ServidorHttp::manejar_cliente(
            logger.clone(),
            &mut mock,
            &endpoints,
            repos_almacen.clone(),
        )
        .unwrap();
        mock.lectura_data = request.as_bytes().to_vec();

        let respuesta = ServidorHttp::manejar_cliente(
            logger.clone(),
            &mut mock,
            &endpoints,
            repos_almacen.clone(),
        )
        .unwrap();

        io::rm_directorio(RUTA_RAIZ.to_string() + ruta_especifica).unwrap();
        io::rm_directorio(RUTA_RAIZ.to_string() + RUTA_REPOSITORIO).unwrap();
        respuesta
    }

    #[test]
    fn test01_se_obtiene_not_found_si_no_existe_el_repositorio() {
        let contenido_mock = "GET /repos/repo_inexistente/pulls HTTP/1.1\r\n\r\n";
        let logger = Arc::new(
            Logger::new(PathBuf::from(RUTA_RAIZ.to_string() + "server_logger.txt")).unwrap(),
        );
        let repos_almacen = ReposAlmacen::new();
        let mut mock = testing::MockTcpStream {
            lectura_data: contenido_mock.as_bytes().to_vec(),
            escritura_data: vec![],
        };
        let respuesta =
            ServidorHttp::manejar_cliente(logger.clone(), &mut mock, &vec![], repos_almacen)
                .unwrap();

        assert_eq!(404, respuesta.estado);
        assert_eq!("Not Found", respuesta.mensaje_estado);
    }

    #[test]
    fn test02_crear_pr_en_repo_devuelve_status_201() {
        let logger = Arc::new(
            Logger::new(PathBuf::from(
                RUTA_RAIZ.to_string() + "/tmp/servidor_http_test02",
            ))
            .unwrap(),
        );

        let logger_clone = logger.clone();
        let (tx, _) = std::sync::mpsc::channel();
        let repos_almacen = ReposAlmacen::new();
        let repos_almacen_clone = repos_almacen.clone();

        let handle = std::thread::spawn(move || {
            let threads: VectorThreads = Arc::new(Mutex::new(Vec::new()));
            let listener = TcpListener::bind("127.0.0.1:9933").unwrap();

            let mut servidor_gir = ServidorGir {
                listener,
                threads,
                logger: logger_clone,
                main: None,
                tx,
                repos_almacen: repos_almacen_clone,
            };
            servidor_gir.iniciar_servidor().unwrap();
        });

        if handle.is_finished() {
            panic!("No se pudo iniciar el servidor");
        }
        std::thread::sleep(std::time::Duration::from_secs(1));

        let _ = io::rm_directorio(RUTA_RAIZ.to_string() + "/tmp/servidor_http_test02_dir");
        let _ = io::rm_directorio(RUTA_RAIZ.to_string() + RUTA_REPOSITORIO);
        io::crear_directorio(RUTA_RAIZ.to_string() + "/tmp/servidor_http_test02_dir").unwrap();
        io::cambiar_directorio(RUTA_RAIZ.to_string() + "/tmp/servidor_http_test02_dir").unwrap();

        crear_repo_para_pr(logger.clone());
        std::thread::sleep(std::time::Duration::from_secs(1));

        let repo = "repo";
        let body = r#"{
            "title": "Feature X: Implement new functionality",
            "head": "juani:master",
            "base": "rama",
            "body": "This is the description of the pull request."
        }"#;
        let content_length = body.len();

        let request_string = format!(
            "POST /repos/{}/pulls HTTP/1.1\r\n\
            Host: localhost:9933\r\n\
            Accept: application/vnd.github+json\r\n\
            Content-Type: application/json\r\n\
            Content-Length: {}\r\n\
            \r\n\
            {}",
            repo, content_length, body
        );

        let mut mock = testing::MockTcpStream {
            lectura_data: request_string.as_bytes().to_vec(),
            escritura_data: vec![],
        };
        io::cambiar_directorio(RUTA_RAIZ).unwrap();
        let mut endpoints = Vec::new();
        ServidorHttp::agregar_endpoints(&mut endpoints);

        let respuesta = ServidorHttp::manejar_cliente(
            logger.clone(),
            &mut mock,
            &endpoints,
            repos_almacen.clone(),
        )
        .unwrap();
        io::rm_directorio(RUTA_RAIZ.to_string() + "/tmp/servidor_http_test02_dir").unwrap();
        io::rm_directorio(RUTA_RAIZ.to_string() + RUTA_REPOSITORIO).unwrap();
        assert_eq!(201, respuesta.estado);
        assert_eq!("Created", respuesta.mensaje_estado);
    }

    #[test]
    fn test03_get_prs_exitoso_devuelve_status_200() {
        let logger = Arc::new(
            Logger::new(PathBuf::from(
                RUTA_RAIZ.to_string() + "/tmp/servidor_http_test03",
            ))
            .unwrap(),
        );

        let get_request_string = format!(
            "GET /repos/{}/pulls HTTP/1.1\r\n\
            Host: localhost:9933\r\n\
            Accept: application/vnd.github+json\r\n\
            \r\n",
            NOMBRE_REPOSITORIO,
        );

        let respuesta = iniciar_servidor_pushear_pr_y_obtener_respuesta_final(
            logger,
            "/tmp/servidor_http_test03_dir",
            &get_request_string,
        );
        assert_eq!(200, respuesta.estado);
        assert_eq!("OK", respuesta.mensaje_estado);
    }

    #[test]
    fn test04_get_pr_exitoso_devuelve_status_200() {
        let logger = Arc::new(
            Logger::new(PathBuf::from(
                RUTA_RAIZ.to_string() + "/tmp/servidor_http_test04",
            ))
            .unwrap(),
        );

        let get_request_string = format!(
            "GET /repos/{}/pulls/1 HTTP/1.1\r\n\
            Host: localhost:9933\r\n\
            Accept: application/vnd.github+json\r\n\
            \r\n",
            NOMBRE_REPOSITORIO,
        );
        let respuesta = iniciar_servidor_pushear_pr_y_obtener_respuesta_final(
            logger,
            "/tmp/servidor_http_test04_dir",
            &get_request_string,
        );

        assert_eq!(200, respuesta.estado);
        assert_eq!("OK", respuesta.mensaje_estado);
    }

    #[test]
    fn test05_get_commits_exitoso_devuelve_status_200() {
        let logger = Arc::new(
            Logger::new(PathBuf::from(
                RUTA_RAIZ.to_string() + "/tmp/servidor_http_test05",
            ))
            .unwrap(),
        );

        let get_request_string = format!(
            "GET /repos/{}/pulls/1/commits HTTP/1.1\r\n\
            Host: localhost:9933\r\n\
            Accept: application/vnd.github+json\r\n\
            \r\n",
            NOMBRE_REPOSITORIO,
        );
        let respuesta = iniciar_servidor_pushear_pr_y_obtener_respuesta_final(
            logger,
            "/tmp/servidor_http_test05_dir",
            &get_request_string,
        );
        assert_eq!(200, respuesta.estado);
        assert_eq!("OK", respuesta.mensaje_estado);
    }

    #[test]
    fn test06_merge_pr_exitoso_devuelve_status_200() {
        let logger = Arc::new(
            Logger::new(PathBuf::from(
                RUTA_RAIZ.to_string() + "/tmp/servidor_http_test06",
            ))
            .unwrap(),
        );

        let get_request_string = format!(
            "PUT /repos/{}/pulls/1/merge HTTP/1.1\r\n\
            Host: localhost:9933\r\n\
            Accept: application/vnd.github+json\r\n\
            \r\n",
            NOMBRE_REPOSITORIO,
        );

        let respuesta = iniciar_servidor_pushear_pr_y_obtener_respuesta_final(
            logger,
            "/tmp/servidor_http_test06_dir",
            &get_request_string,
        );
        assert_eq!(200, respuesta.estado);
        assert_eq!("OK", respuesta.mensaje_estado);
    }

    #[test]
    fn test07_patch_exitoso_devuelve_status_200() {
        let logger = Arc::new(
            Logger::new(PathBuf::from(
                RUTA_RAIZ.to_string() + "/tmp/servidor_http_test07",
            ))
            .unwrap(),
        );

        let get_request_string = format!(
            "PATCH /repos/{}/pulls/1 HTTP/1.1\r\n\
            Host: localhost:9933\r\n\
            Accept: application/vnd.github+json\r\n\
            \r\n",
            NOMBRE_REPOSITORIO,
        );

        let respuesta = iniciar_servidor_pushear_pr_y_obtener_respuesta_final(
            logger,
            "/tmp/servidor_http_test07_dir",
            &get_request_string,
        );

        assert_eq!(200, respuesta.estado);
        assert_eq!("OK", respuesta.mensaje_estado);
    }
}
