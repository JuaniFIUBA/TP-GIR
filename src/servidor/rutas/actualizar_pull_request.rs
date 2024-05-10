use std::{collections::HashMap, sync::Arc};

use crate::{
    tipos_de_dato::{
        http::{
            endpoint::Endpoint, error::ErrorHttp, estado::EstadoHttp, metodos::MetodoHttp,
            request::Request, response::Response,
        },
        logger::Logger,
    },
};

use super::{
    crear_pull_request::{
        guadar_pull_request_acorde_al_numero, responder_pull_request_en_formato_json,
    },
    obtener_pull_request::obtener_pull_request_de_params,
};

pub fn agregar_a_router(rutas: &mut Vec<Endpoint>) {
    let endpoint = Endpoint::new(
        MetodoHttp::Patch,
        "/repos/{repo}/pulls/{pull_number}".to_string(),
        actualizar_pull_request,
    );
    rutas.push(endpoint)
}

fn actualizar_pull_request(
    request: Request,
    params: HashMap<String, String>,
    logger: Arc<Logger>,
) -> Result<Response, ErrorHttp> {
    let mut pull_request = obtener_pull_request_de_params(&params.clone())?;

    let repo = params.get("repo").ok_or_else(|| {
        ErrorHttp::InternalServerError("No se ha encontrado el nombre del repositorio".to_string())
    })?;

    if let Some(body) = request.body {
        pull_request.actualizar(body)?;
    }
    guadar_pull_request_acorde_al_numero(&pull_request, repo)?;
    responder_pull_request_en_formato_json(pull_request, logger, EstadoHttp::Ok)
}
