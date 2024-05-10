use std::{collections::HashMap, path::PathBuf, sync::Arc};

use crate::{
    servidor::{
        pull_request::PullRequest,
    },
    tipos_de_dato::{
        http::{
            endpoint::Endpoint, error::ErrorHttp, estado::EstadoHttp, metodos::MetodoHttp,
            request::Request, response::Response,
        },
        logger::Logger,
    },
};

use super::crear_pull_request::responder_pull_request_en_formato_json;

pub fn agregar_a_router(rutas: &mut Vec<Endpoint>) {
    let endpoint = Endpoint::new(
        MetodoHttp::Get,
        "/repos/{repo}/pulls/{pull_number}".to_string(),
        obtener_pull_request,
    );
    rutas.push(endpoint)
}

fn obtener_pull_request(
    _request: Request,
    params: HashMap<String, String>,
    logger: Arc<Logger>,
) -> Result<Response, ErrorHttp> {
    let pull_request = obtener_pull_request_de_params(&params)?;

    responder_pull_request_en_formato_json(pull_request, logger, EstadoHttp::Ok)
}

///Obtiene el objeto pull request desde los parametros. Para ello en los parametros tiene que estar
/// el `repo` y `pull_number`
///
/// ## Argumuntos
/// - params: los parametros obtenidos de la ruta del pedido. Debe contener `repo` y `pull_number`   
///
/// ## Resultado
/// - el pull request guardado en el directorio `./srv/{repo}/pulls/{pull_number}`
///
/// ## Errores
/// - Si no existe la carpeta `./srv/{repo}/pulls/{pull_number}`
pub fn obtener_pull_request_de_params(
    params: &HashMap<String, String>,
) -> Result<PullRequest, ErrorHttp> {
    let dir_pull_request = obtener_dir_pull_request(params)?;
    let pull_request = PullRequest::cargar_pr(&dir_pull_request)?;
    Ok(pull_request)
}

pub fn obtener_dir_pull_request(params: &HashMap<String, String>) -> Result<PathBuf, ErrorHttp> {
    let repo = params.get("repo").ok_or_else(|| {
        ErrorHttp::InternalServerError("No se ha encontrado el nombre del repositorio".to_string())
    })?;
    let pull_number = params.get("pull_number").ok_or_else(|| {
        ErrorHttp::InternalServerError(
            "No se ha encontrado el pull number del repositorio".to_string(),
        )
    })?;
    let dir_pull_request = PathBuf::from(format!("./srv/{repo}/pulls/{pull_number}"));

    if dir_pull_request.exists() {
        Ok(dir_pull_request)
    } else {
        Err(ErrorHttp::NotFound(format!(
            "No se encontro en el server {:?}",
            dir_pull_request
        )))
    }
}
