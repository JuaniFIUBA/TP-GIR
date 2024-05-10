use std::{collections::HashMap, path::PathBuf, sync::Arc};

use crate::{
    servidor::{pull_request::PullRequest},
    tipos_de_dato::{
        http::{
            endpoint::Endpoint, error::ErrorHttp, estado::EstadoHttp, metodos::MetodoHttp,
            request::Request, response::Response,
        },
        logger::Logger,
    },
};

pub fn agregar_a_router(rutas: &mut Vec<Endpoint>) {
    let endpoint = Endpoint::new(
        MetodoHttp::Post,
        "/repos/{repo}/pulls".to_string(),
        crear_pull_request,
    );
    rutas.push(endpoint)
}

fn crear_pull_request(
    request: Request,
    params: HashMap<String, String>,
    logger: Arc<Logger>,
) -> Result<Response, ErrorHttp> {
    let repo = params.get("repo").ok_or_else(|| {
        ErrorHttp::InternalServerError("No se ha encontrado el nombre del repositorio".to_string())
    })?;

    let body = match request.body {
        Some(body) => body,
        None => {
            return Err(ErrorHttp::BadRequest(
                "No se ha encontrado el cuerpo de la solicitud".to_string(),
            ))
        }
    };

    let pull_request = PullRequest::crear_pr(repo, body)?;
    guadar_pull_request_acorde_al_numero(&pull_request, repo)?;

    responder_pull_request_en_formato_json(pull_request, logger, EstadoHttp::Created)
}

pub fn guadar_pull_request_acorde_al_numero(
    pull_request: &PullRequest,
    repo: &str,
) -> Result<(), ErrorHttp> {
    pull_request.guardar_pr(&PathBuf::from(format!(
        "srv/{repo}/pulls/{numero}",
        numero = pull_request.numero
    )))?;
    Ok(())
}

pub fn responder_pull_request_en_formato_json(
    pull_request: PullRequest,
    logger: Arc<Logger>,
    estado: EstadoHttp,
) -> Result<Response, ErrorHttp> {
    let body_respuesta = serde_json::to_string(&pull_request).map_err(|e| {
        ErrorHttp::InternalServerError(format!("No se ha podido serializar el pull request: {}", e))
    })?;
    let respuesta = Response::new(logger, estado, Some(&body_respuesta));
    Ok(respuesta)
}
