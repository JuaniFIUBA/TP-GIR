use super::path_buf;

///Devuelve un vector con todos los tags
pub fn obtener_tags() -> Result<Vec<String>, String> {
    let ubicacion = "./.gir/refs/tags";
    let mut tags: Vec<String> = Vec::new();

    let tags_entries = std::fs::read_dir(ubicacion)
        .map_err(|e| format!("Error al leer el directorio de tags: {}", e))?;

    for tag_entry in tags_entries {
        let tag_dir = tag_entry
            .map_err(|e| format!("Error al leer el directorio de tags: {}", e))?
            .path();
        let tag = path_buf::obtener_nombre(&tag_dir)?;

        tags.push(tag);
    }

    Ok(tags)
}

pub fn existe_tag(tag: &str) -> bool {
    obtener_tags()
        .unwrap_or_default()
        .contains(&tag.to_string())
}
