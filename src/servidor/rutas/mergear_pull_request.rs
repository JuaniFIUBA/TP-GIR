use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc},
};

use crate::{
    servidor::{pull_request::PullRequest},
    tipos_de_dato::{
        comando::Ejecutar,
        comandos::{merge::Merge, rebase::Rebase},
        http::{
            endpoint::Endpoint, error::ErrorHttp, estado::EstadoHttp, metodos::MetodoHttp,
            request::Request, response::Response,
        },
        logger::Logger,
    },
    utils::{index, io, ramas},
};

use super::obtener_pull_request::obtener_pull_request_de_params;

#[derive(Debug, PartialEq, Eq)]
enum MetodoMerge {
    Merge,
    Rebase,
    Squash,
}

pub fn agregar_a_router(rutas: &mut Vec<Endpoint>) {
    let endpoint = Endpoint::new(
        MetodoHttp::Put,
        "/repos/{repo}/pulls/{pull_number}/merge".to_string(),
        mergear_pull_request,
    );
    rutas.push(endpoint)
}

fn armar_body_merge(hash_merge: String) -> String {
    let body_merge = format!(
        r#"{{
            "sha": "{}",
            "merged": true,
            "message": "Pull Request mergeado con exito"
        }}"#,
        hash_merge
    );
    body_merge
}

fn verificar_sha_head(sha: &str, pull_request: &PullRequest) -> Result<bool, ErrorHttp> {
    pull_request.entrar_a_repositorio()?;
    let hash_head_previo_merge = ramas::obtener_hash_commit_asociado_rama(&pull_request.rama_head)
        .map_err(|error| {
            ErrorHttp::InternalServerError(format!(
                "No se ha podido obtener el hash del commit de la rama {}: {}",
                pull_request.rama_head, error
            ))
        })?;
    pull_request.salir_del_repositorio()?;

    Ok(sha == hash_head_previo_merge)
}

fn obtener_params_body(
    request: Request,
    pull_request: &PullRequest,
) -> Result<MetodoMerge, ErrorHttp> {
    let body = match request.body {
        Some(body) => body,
        None => return Ok(MetodoMerge::Merge),
    };

    if let Some(sha) = body.get("sha") {
        if !verificar_sha_head(sha, pull_request)? {
            return Err(ErrorHttp::Conflict(
                "El sha del head no coincide con el sha del pull request".to_string(),
            ));
        }
    }

    if let Some(merge_method) = body.get("merge_method") {
        match merge_method.as_str() {
            "merge" => return Ok(MetodoMerge::Merge),
            "rebase" => return Ok(MetodoMerge::Rebase),
            "squash" => return Ok(MetodoMerge::Squash),
            _ => {
                return Err(ErrorHttp::ValidationFailed(
                    "merge_method invalido".to_string(),
                ))
            }
        }
    };
    Ok(MetodoMerge::Merge)
}

fn pr_mergeado_con_exito(
    rama_base: &str,
    pull_request: &mut PullRequest,
    logger: Arc<Logger>,
) -> Result<Response, ErrorHttp> {
    let hash_merge = ramas::obtener_hash_commit_asociado_rama(rama_base).map_err(|error| {
        ErrorHttp::InternalServerError(format!(
            "No se ha podido obtener el hash del commit de la rama {}: {}",
            rama_base, error
        ))
    })?;
    let body_response = armar_body_merge(hash_merge);
    pull_request.estado = "closed".to_string();

    let dir_pull_request = PathBuf::from(format!("pulls/{}", pull_request.numero));
    pull_request.guardar_pr(&dir_pull_request)?;
    let response = Response::new(logger, EstadoHttp::Ok, Some(&body_response));
    Ok(response)
}

fn volver_a_estado_previo_al_merge() -> Result<(), ErrorHttp> {
    io::rm_directorio(".gir/MERGE_HEAD").map_err(|error| {
        ErrorHttp::InternalServerError(format!(
            "No se ha podido eliminar el archivo MERGE_HEAD: {}",
            error
        ))
    })?;
    index::limpiar_archivo_index().map_err(|error| {
        ErrorHttp::InternalServerError(format!(
            "No se ha podido limpiar el archivo index: {}",
            error
        ))
    })?;
    Ok(())
}

fn mergear_pr_ejecutado_con_fallos(
    logger: Arc<Logger>,
    error: String,
    merge_method: MetodoMerge,
) -> Result<Response, ErrorHttp> {
    let hay_conflictos = index::hay_archivos_con_conflictos(logger.clone());
    if merge_method == MetodoMerge::Merge {
        volver_a_estado_previo_al_merge()?;
    } else {
        volver_a_estado_previo_al_rebase(logger.clone())?;
    }

    if hay_conflictos {
        let response = Response::new(logger, EstadoHttp::MethodNotAllowed, None);
        Ok(response)
    } else {
        Err(ErrorHttp::InternalServerError(format!(
            "No se ha podido mergear el pull request: {}",
            error
        )))
    }
}

fn mergear_pull_request_utilizando_merge(
    pull_request: &mut PullRequest,
    logger: Arc<Logger>,
) -> Result<Response, ErrorHttp> {
    let rama_base = pull_request.rama_base.clone();
    let rama_head = pull_request.rama_head.clone();

    let mut merge = Merge {
        logger: logger.clone(),
        branch_actual: rama_base.clone(),
        branch_a_mergear: rama_head,
        abort: false,
        no_fast_forward: true,
    };

    pull_request.entrar_a_repositorio()?;

    let resultado = match merge.ejecutar() {
        Ok(_) => pr_mergeado_con_exito(&rama_base, pull_request, logger),
        Err(error) => {
            mergear_pr_ejecutado_con_fallos(logger, error.to_string(), MetodoMerge::Merge)
        }
    };

    pull_request.salir_del_repositorio()?;

    resultado
}

fn volver_a_estado_previo_al_rebase(logger: Arc<Logger>) -> Result<(), ErrorHttp> {
    let mut rebase = Rebase::from(vec!["--abort".to_string()], logger).map_err(|error| {
        ErrorHttp::InternalServerError(format!("No se ha podido abortar el rebase: {}", error))
    })?;
    rebase.ejecutar().map_err(|error| {
        ErrorHttp::InternalServerError(format!("No se ha podido abortar el rebase: {}", error))
    })?;
    Ok(())
}

fn mergear_pull_request_utilizando_rebase(
    pull_request: &mut PullRequest,
    logger: Arc<Logger>,
) -> Result<Response, ErrorHttp> {
    let rama_base = pull_request.rama_base.clone();
    let rama_head = pull_request.rama_head.clone();

    let mut rebase = Rebase {
        logger: logger.clone(),
        rama_actual: rama_base.clone(),
        rama: Some(rama_head),
        continue_: false,
        abort: false,
    };

    pull_request.entrar_a_repositorio()?;

    let resultado = match rebase.ejecutar() {
        Ok(_) => pr_mergeado_con_exito(&rama_base, pull_request, logger),
        Err(error) => {
            mergear_pr_ejecutado_con_fallos(logger, error.to_string(), MetodoMerge::Rebase)
        }
    };

    pull_request.salir_del_repositorio()?;

    resultado
}

fn mergear_pull_request(
    request: Request,
    params: HashMap<String, String>,
    logger: Arc<Logger>,
) -> Result<Response, ErrorHttp> {
    let mut pull_request = obtener_pull_request_de_params(&params)?;

    if pull_request.estado != "open" {
        let response = Response::new(logger, EstadoHttp::ValidationFailed, None);
        return Ok(response);
    }

    let merge_method = obtener_params_body(request, &pull_request)?;

    match merge_method {
        MetodoMerge::Merge => mergear_pull_request_utilizando_merge(&mut pull_request, logger),
        MetodoMerge::Rebase => mergear_pull_request_utilizando_rebase(&mut pull_request, logger),
        MetodoMerge::Squash => Err(ErrorHttp::NotImplemented(
            "Metodo squash no implementado".to_string(),
        )),
    }
}
