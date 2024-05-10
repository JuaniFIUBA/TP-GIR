use std::{
    collections::{HashMap, HashSet},
    path::Path,
    sync::Arc,
};

use gtk::{prelude::*, Orientation};

use crate::{
    tipos_de_dato::{
        comandos::{branch::Branch, log::Log},
        logger::Logger,
        objetos::commit::CommitObj,
    },
    utils::{
        compresion::descomprimir_objeto_gir,
        io::{self, leer_a_string},
    },
};

use super::{info_dialog, log_seleccionado};

fn obtener_tronco_principal_padres(mut commit: CommitObj, logger: Arc<Logger>) -> HashSet<String> {
    let mut commits: HashSet<String> = HashSet::new();
    commits.insert(commit.hash.clone());

    while let Some(padre) = commit.padres.get(0) {
        let commit_padre = CommitObj::from_hash(padre.to_owned(), logger.clone()).unwrap();
        commits.insert(commit_padre.hash.clone());
        commit = commit_padre;
    }

    commits
}

fn obtener_commits_con_branches(
    rama: &str,
    logger: Arc<Logger>,
) -> Result<Vec<(CommitObj, String)>, String> {
    let ramas_largas = Branch::obtener_ramas()?;

    let ramas: Vec<_> = ramas_largas
        .iter()
        .map(|rama| if rama.len() > 25 { &rama[..25] } else { rama })
        .collect();

    let mut commits_por_ramas = Vec::new();

    for rama in ramas {
        let commit_hash_rama = leer_a_string(".gir/refs/heads/".to_string() + rama)?;
        if commit_hash_rama.is_empty() {
            continue;
        }
        let commit_rama = CommitObj::from_hash(commit_hash_rama, logger.clone())?;
        let commits_rama = obtener_tronco_principal_padres(commit_rama, logger.clone());
        commits_por_ramas.push((rama.to_string(), commits_rama));
    }

    let mut commits_y_ramas: Vec<(CommitObj, String)> = Vec::new();

    let commits = obtener_listas_de_commits(rama, logger.clone())?;

    if commits.is_empty() {
        return Ok(Vec::new());
    }

    for commit in commits {
        let mut encontrados = Vec::new();
        for (rama, commits_rama) in commits_por_ramas.iter() {
            if commits_rama.contains(&commit.hash) {
                encontrados.push((commit.clone(), rama.to_owned()));
            }
        }
        let commit_rama_actual = encontrados.iter().find(|(_, r)| rama == *r);
        if let Some(commit_rama_actual) = commit_rama_actual {
            commits_y_ramas.push(commit_rama_actual.clone());
        } else if !encontrados.is_empty() {
            commits_y_ramas.push(encontrados[0].clone());
        }
    }

    Ok(commits_y_ramas)
}

fn generar_clases_por_rama() -> HashMap<String, String> {
    let clases_posibles = ["green", "blue", "yellow", "red"];

    let mut clases_por_rama = HashMap::new();
    let mut i = 0;

    for rama in Branch::obtener_ramas().unwrap() {
        clases_por_rama.insert(rama, clases_posibles[i].to_string());
        i += 1;
        if i == clases_posibles.len() {
            i = 0;
        }
    }

    clases_por_rama
}

fn obtener_listas_de_commits(branch: &str, logger: Arc<Logger>) -> Result<Vec<CommitObj>, String> {
    let ruta = format!(".gir/refs/heads/{}", branch);
    let ultimo_commit = io::leer_a_string(Path::new(&ruta))?;

    if ultimo_commit.is_empty() {
        return Ok(Vec::new());
    }

    let commit_obj = CommitObj::from_hash(ultimo_commit, logger.clone())?;
    Log::obtener_listas_de_commits(commit_obj, logger.clone())
}

pub fn obtener_mensaje_commit(commit_hash: &str) -> Result<String, String> {
    let commit = descomprimir_objeto_gir(commit_hash).unwrap_or("".to_string());

    let mensaje = commit
        .splitn(2, "\n\n")
        .last()
        .ok_or("Error al obtener mensaje del commit")?;

    let primera_linea = mensaje
        .split('\n')
        .next()
        .ok_or("Error al obtener mensaje del commit")?;

    if primera_linea.len() > 25 {
        Ok(format!("{}...", &primera_linea[..25]))
    } else {
        Ok(primera_linea.to_string())
    }
}

fn crear_label(mensaje: &str, hash: &str, color: &str, branch: &str) -> gtk::EventBox {
    let event_box = gtk::EventBox::new();

    let container = gtk::Box::new(Orientation::Horizontal, 0);
    event_box.add(&container);
    container.set_margin_start(6);
    container.set_margin_top(2);
    container.set_margin_bottom(1);
    container.set_margin_end(18); // Set margin at the end

    let commit_y_hash = gtk::Box::new(Orientation::Horizontal, 8);
    let label_message = gtk::Label::new(Some(mensaje));
    let label_hash = gtk::Label::new(Some(&hash[0..10]));

    label_hash.style_context().add_class("label-grey");

    commit_y_hash.add(&label_message);
    commit_y_hash.add(&label_hash);

    container.add(&commit_y_hash);

    let spacer = gtk::Box::new(Orientation::Horizontal, 0);
    spacer.set_hexpand(true);
    container.pack_start(&spacer, true, true, 0);

    let label_branch = gtk::Label::new(Some(branch));
    label_branch
        .style_context()
        .add_class(&format!("label-{}", color));

    container.add(&label_branch);

    event_box
        .style_context()
        .add_class(&format!("commit-label-{}", color));

    event_box
}

pub fn render(builder: &gtk::Builder, branch: &str, logger: Arc<Logger>) {
    let container: gtk::Box = builder.object("log-container").unwrap();
    container.children().iter().for_each(|child| {
        container.remove(child);
    });

    let commits = match obtener_commits_con_branches(branch, logger.clone()) {
        Ok(commits) => commits,
        Err(err) => {
            info_dialog::mostrar_error(&err);
            return;
        }
    };

    if commits.is_empty() {
        return;
    }

    let clases = generar_clases_por_rama();

    for (commit, branch) in commits {
        let event_box = crear_label(
            &obtener_mensaje_commit(&commit.hash).unwrap(),
            &commit.hash,
            clases.get(&branch).unwrap(),
            &branch,
        );

        let builder_clone = builder.clone();
        event_box.connect_button_press_event(move |_, _| {
            log_seleccionado::render(&builder_clone, Some(&commit.hash));
            gtk::glib::Propagation::Stop
        });
        container.add(&event_box);
    }
    if !container.children().is_empty() {
        let children = container.children();
        let ultimo = children.last().unwrap();
        ultimo.style_context().add_class("last-commit-label");
    }
    container.show_all();
}

pub fn refresh(builder: &gtk::Builder) {
    let container: gtk::Box = builder.object("log-container").unwrap();
    container.show_all();
}
